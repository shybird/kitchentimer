extern crate termion;
extern crate signal_hook;
mod alarm;
mod clock;
mod common;
mod layout;

use std::{time, thread, env};
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use signal_hook::flag;
use termion::{clear, color, cursor, style};
use termion::raw::{IntoRawMode, RawTerminal};
use termion::event::Key;
use termion::input::TermRead;
use clock::Clock;
use alarm::{Countdown, AlarmRoster, alarm_exec};
use layout::{Layout, Position};


const NAME: &str = "kitchentimer";
const VERSION: &str = "0.0.1";
const USAGE: &str =
"USAGE: kitchentimer [-h|-v] [-p] [ALARM TIME(s)] [-e|--exec COMMAND [...]]
PARAMETERS:
  [ALARM TIME]          None or multiple alarm times (HH:MM:SS).
OPTIONS:
  -h, --help            Display this help.
  -v, --version         Display version information.
  -p, --plain           Use simpler block chars.
  -e, --exec [COMMAND]  Execute COMMAND on alarm. Must be the last flag on
                        the command line. Everything after it is passed as
                        argument to COMMAND. Every \"%s\" will be replaced
                        with the elapsed time in (HH:)MM:SS format.

SIGNALS: <SIGUSR1> Reset clock.
         <SIGUSR2> Pause or un-pause clock.";
const MENUBAR: &str =
"[0-9] Add alarm  [d] Delete alarm  [SPACE] Pause  [r] Reset  [c] Clear color  [q] Quit";
const MENUBAR_SHORT: &str =
"[0-9] Add  [d] Delete  [SPACE] Pause  [r] Reset  [c] Clear  [q] Quit";
// Needed for signal_hook.
const SIGTSTP: usize = signal_hook::consts::SIGTSTP as usize;
const SIGWINCH: usize = signal_hook::consts::SIGWINCH as usize;
const SIGCONT: usize = signal_hook::consts::SIGCONT as usize;
const SIGTERM: usize = signal_hook::consts::SIGTERM as usize;
const SIGINT: usize = signal_hook::consts::SIGINT as usize;
const SIGUSR1: usize = signal_hook::consts::SIGUSR1 as usize;
const SIGUSR2: usize = signal_hook::consts::SIGUSR2 as usize;

pub struct Config {
    plain: bool,
    alarm_exec: Option<Vec<String>>,
}


fn main() {
    let mut config = Config {
        plain: false,
        alarm_exec: None,
    };
    let mut alarm_roster = AlarmRoster::new();
    parse_args(&mut config, &mut alarm_roster);

    let mut stdout = std::io::stdout().into_raw_mode()
        .unwrap_or_else(|error| {
            eprintln!("Error opening stdout: {}", error);
            std::process::exit(1);
        });
    let mut input_keys = termion::async_stdin().keys();
    let mut layout = Layout::new(&config);
    let mut clock = Clock::new();
    let mut buffer = String::new();
    let mut buffer_updated: bool = false;
    let mut countdown = Countdown::new();

    // Register signal handlers.
    let signal = Arc::new(AtomicUsize::new(0));
    register_signal_handlers(&signal, &layout);
    
    // Clear window and hide cursor.
    write!(stdout,
        "{}{}",
        clear::All,
        cursor::Hide)
        .unwrap_or_else(|error| {
            eprintln!("Error writing to stdout: {}", error);
            std::process::exit(1);
        });

    // Enter main loop.
    loop {
        // Process received signals.
        match signal.swap(0, Ordering::Relaxed) {
            // No signal received.
            0 => (),
            // Suspend execution on SIGTSTP.
            SIGTSTP => {
                suspend(&mut stdout);
                // Clear SIGCONT, as we have already taken care to reset the
                // terminal.
                signal.compare_and_swap(SIGCONT, 0, Ordering::Relaxed);
                layout.force_redraw = true;
                // Jump to the start of the main loop.
                continue;
            },
            // Continuing after SIGSTOP.
            SIGCONT => {
                // This is reached when the process was suspended by SIGSTOP.
                restore_after_suspend(&mut stdout);
                layout.force_redraw = true;
            },
            // Exit main loop on SIGTERM and SIGINT.
            SIGTERM | SIGINT => break,
            // Reset clock on SIGUSR1.
            SIGUSR1 => {
                clock.reset();
                alarm_roster.reset_all();
                layout.force_recalc.store(true, Ordering::Relaxed);
                layout.force_redraw = true;
            },
            // (Un-)Pause clock on SIGUSR2.
            SIGUSR2 => clock.toggle(),
            // We didn't register anything else.
            _ => unreachable!(),
        }

        // Process input.
        if let Some(key) = input_keys.next() {
            match key.expect("Error reading input") {
                // Reset clock on 'r'.
                Key::Char('r') => {
                    clock.reset();
                    alarm_roster.reset_all();
                    layout.force_recalc.store(true, Ordering::Relaxed);
                    layout.force_redraw = true;
                },
                // (Un-)Pause on space.
                Key::Char(' ') => {
                    clock.toggle();
                },
                // Clear clock color on 'c'.
                Key::Char('c') => {
                    clock.color_index = None;
                    layout.force_redraw = true;
                },
                // Delete last alarm on 'd'.
                Key::Char('d') => {
                    if alarm_roster.pop().is_some() {
                        // If we remove the last alarm we have to reset "countdown"
                        // manually. It is safe to do it anyway.
                        countdown.reset();
                        layout.force_redraw = true;
                    }
                },
                // Exit on q and ^C.
                Key::Char('q') | Key::Ctrl('c') => break,
                // Force redraw on ^R.
                Key::Ctrl('r') => layout.force_redraw = true,
                // Suspend an ^Z.
                Key::Ctrl('z') => {
                    suspend(&mut stdout);
                    // Clear SIGCONT, as we have already taken care to reset
                    // the terminal.
                    signal.compare_and_swap(SIGCONT, 0, Ordering::Relaxed);
                    layout.force_redraw = true;
                    // Jump to the start of the main loop.
                    continue;
                },
                // Enter.
                Key::Char('\n') => {
                    if !buffer.is_empty() {
                        if let Err(e) = alarm_roster.add(&buffer) {
                            // Error while processing input buffer.
                            error_msg(&mut stdout, &layout, e);
                        } else {
                            // Input buffer processed without error.
                            layout.force_redraw = true;
                        }
                        buffer.clear();
                    }
                },
                // Escape ^W, and ^U clear input buffer.
                Key::Esc | Key::Ctrl('w') | Key::Ctrl('u') => {
                    buffer.clear();
                    buffer_updated = true;
                },
                // Backspace.
                Key::Backspace => {
                    // Delete last char in buffer. It makes no difference to us
                    // if this succeeds of fails.
                    let _ = buffer.pop();
                    buffer_updated = true;
                },
                Key::Char(c) => {
                    if c.is_ascii_digit() {
                        buffer.push(c);
                        buffer_updated = true;
                    } else if !buffer.is_empty() && c == ':' {
                        buffer.push(':');
                        buffer_updated = true;
                    }
                },
                // Any other key.
                _ => (),
            }
        }

        // Update input buffer display.
        if buffer_updated {
            draw_buffer(&mut stdout, &layout, &buffer);
            buffer_updated = false;
            stdout.flush().unwrap();
        }

        let elapsed = if clock.paused {
                clock.elapsed
            } else {
                // Should never overflow as we reestablish a new "start"
                // instant every 24 hours.
                clock.start.elapsed().as_secs() as u32
            };

        // Update window content if necessary.
        if elapsed != clock.elapsed || layout.force_redraw  {
            // Update clock. Advance one day after 24 hours.
            if elapsed < 24 * 60 * 60 {
                clock.elapsed = elapsed;
            } else {
                clock.next_day();
                // "clock.elapsed" set by "clock.next_day()".
                alarm_roster.reset_all();
                layout.force_recalc.store(true, Ordering::Relaxed);
            }

            // Update window size information and calculate the clock position.
            // Also enforce recalculation of layout if we start displaying
            // hours.
            layout.update(clock.elapsed >= 3600, clock.elapsed == 3600);

            // Check for exceeded alarms.
            if alarm_roster.check(&mut clock, &layout, &mut countdown) {
                // Write ASCII bell code.
                write!(stdout, "{}", 0x07 as char).unwrap();
                layout.force_redraw = true;

                // Run command if configured.
                if config.alarm_exec.is_some() {
                    alarm_exec(&config, clock.elapsed);
                }
            }

            // Clear the window and redraw menu bar, alarm roster and buffer if
            // requested.
            if layout.force_redraw {
                write!(stdout,
                    "{}{}{}{}{}",
                    clear::All,
                    cursor::Goto(1, 1),
                    style::Faint,
                    // Use a compressed version of the menu bar if necessary.
                    if layout.width >= MENUBAR.len() as u16 {
                        MENUBAR
                    } else if layout.width >= MENUBAR_SHORT.len() as u16 {
                        MENUBAR_SHORT
                    } else {
                        ""
                    },
                    style::Reset,)
                    .unwrap();

                // Redraw list of alarms.
                alarm_roster.draw(&mut stdout, &mut layout);

                // Redraw buffer.
                draw_buffer(&mut stdout, &layout, &buffer);
            }

            clock.draw(&mut stdout, &layout);

            // Display countdown.
            if countdown.value > 0 {
                countdown.draw(&mut stdout);
            }

            // Reset redraw_all and flush stdout.
            layout.force_redraw = false;
            stdout.flush().unwrap();
        }

        // Main loop delay.
        thread::sleep(time::Duration::from_millis(100));
    }

    // Main loop exited. Clear window and restore cursor.
    write!(stdout,
        "{}{}{}",
        clear::BeforeCursor,
        cursor::Goto(1, 1),
        cursor::Show)
        .unwrap();
}

fn usage() {
    println!("{}", USAGE);
    std::process::exit(0);
}

// Parse command line arguments into "config".
fn parse_args(config: &mut Config, alarm_roster: &mut AlarmRoster) {
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => usage(),
            "-v" | "--version" => {
                println!("{} {}", NAME, VERSION);
                std::process::exit(0);
            },
            "-p" | "--plain" => { config.plain = true; },
            "-e" | "--exec" => {
                // Find position of this flag.
                let i = env::args().position(|s| { s == "-e" || s == "--exec" }).unwrap();
                // Copy everything thereafter.
                let exec: Vec<String> = env::args().skip(i + 1).collect();
                if exec.is_empty() {
                    usage();
                } else {
                    config.alarm_exec = Some(exec);
                    // Ignore everything after this flag.
                    break;
                }
            },
            any if any.starts_with('-') => {
                // Unrecognized flag.
                println!("Unrecognized option: \"{}\"", any);
                println!("Use \"-h\" or \"--help\" for a list of valid command line options");
                std::process::exit(1);
            },
            any => {
                if let Err(error) = alarm_roster.add(&String::from(any)) {
                    println!("Error adding \"{}\" as alarm. ({})", any, error);
                    std::process::exit(1);
                }
            },
        }
    }
}

fn register_signal_handlers(signal: &Arc<AtomicUsize>, layout: &Layout) {

    flag::register_usize(SIGTSTP as i32, Arc::clone(&signal), SIGTSTP).unwrap();
    flag::register_usize(SIGCONT as i32, Arc::clone(&signal), SIGCONT).unwrap();
    flag::register_usize(SIGTERM as i32, Arc::clone(&signal), SIGTERM).unwrap();
    flag::register_usize(SIGINT as i32, Arc::clone(&signal), SIGINT).unwrap();
    flag::register_usize(SIGUSR1 as i32, Arc::clone(&signal), SIGUSR1).unwrap();
    flag::register_usize(SIGUSR2 as i32, Arc::clone(&signal), SIGUSR2).unwrap();

    // SIGWINCH sets "force_recalc" directly.
    flag::register(SIGWINCH as i32, Arc::clone(&layout.force_recalc)).unwrap();
}

// Suspend execution on SIGTSTP.
fn suspend<W: Write>(mut stdout: &mut RawTerminal<W>) {
    write!(stdout,
        "{}{}{}",
        cursor::Goto(1,1),
        clear::All,
        cursor::Show)
        .unwrap();
    stdout.flush().unwrap();
    stdout.suspend_raw_mode()
        .unwrap_or_else(|error| {
            eprintln!("Failed to leave raw terminal mode prior to suspend: {}", error);
        });

    if let Err(error) = signal_hook::low_level::emulate_default_handler(SIGTSTP as i32) {
        eprintln!("Error raising SIGTSTP: {}", error);
    }

    restore_after_suspend(&mut stdout);
}

// Set up terminal after SIGTSTP or SIGSTOP.
fn restore_after_suspend<W: Write>(stdout: &mut RawTerminal<W>) {
    stdout.activate_raw_mode()
        .unwrap_or_else(|error| {
            eprintln!("Failed to re-enter raw terminal mode after suspend: {}", error);
            std::process::exit(1);
        });
    write!(stdout,
        "{}{}",
        clear::All,
        cursor::Hide)
        .unwrap_or_else(|error| {
            eprintln!("Error writing to stdout: {}", error);
            std::process::exit(1);
        });
}

// Draw input buffer.
fn draw_buffer<W: Write>(stdout: &mut RawTerminal<W>, layout: &Layout, buffer: &String) {
    if !buffer.is_empty() {
        write!(stdout,
            "{}{}Add alarm: {}",
            cursor::Goto(layout.buffer.col, layout.buffer.line),
            clear::CurrentLine,
            buffer)
            .unwrap();
    } else {
        // Clear buffer display.
        write!(stdout,
            "{}{}",
            cursor::Goto(layout.buffer.col, layout.buffer.line),
            clear::CurrentLine)
            .unwrap();
    }
}

// Print error message.
fn error_msg<W: Write>(stdout: &mut RawTerminal<W>, layout: &Layout, msg: &'static str) {
    write!(stdout,
        "{}{}{}{}",
        cursor::Goto(layout.error.col, layout.error.line),
        color::Fg(color::LightRed),
        msg,
        color::Fg(color::Reset))
        .unwrap();
}

