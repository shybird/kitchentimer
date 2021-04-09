extern crate termion;
extern crate signal_hook;
extern crate unicode_segmentation;
mod alarm;
mod clock;
mod common;
mod layout;
#[cfg(test)]
mod tests;

use std::{time, thread, env};
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use signal_hook::{flag, low_level};
use termion::{clear, color, cursor, style};
use termion::raw::{IntoRawMode, RawTerminal};
use termion::event::Key;
use termion::input::TermRead;
use clock::Clock;
use alarm::{Countdown, AlarmRoster, exec_command};
use layout::{Layout, Position};
use common::{Config, str_length};


const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const USAGE: &str = concat!("USAGE: ", env!("CARGO_PKG_NAME"),
" [-h|-v] [-e|--exec COMMAND] [-p] [-q] [ALARM TIME(s)]

PARAMETERS:
  [ALARM TIME]          None or multiple alarm times (HH:MM:SS).

OPTIONS:
  -h, --help            Show this help.
  -v, --version         Show version information.
  -e, --exec [COMMAND]  Execute COMMAND on alarm. Every occurrence of {}
                        will be replaced by the elapsed time in (HH:)MM:SS
                        format.
  -p, --plain           Use simpler block chars.
  -q, --quit            Quit program after last alarm.

SIGNALS: <SIGUSR1> Reset clock.
         <SIGUSR2> Pause or un-pause clock.");
const MENUBAR: &str =
"[0-9] Add alarm  [d] Delete alarm  [SPACE] Pause  [r] Reset  [c] Clear color  [q] Quit";
const MENUBAR_SHORT: &str =
"[0-9] Add  [d] Delete  [SPACE] Pause  [r] Reset  [c] Clear  [q] Quit";
const MENUBAR_INS: &str =
"Format: HH:MM:SS/LABEL  [ENTER] Accept  [ESC] Cancel  [CTR-C] Quit";
// Needed for signal_hook.
const SIGTSTP: usize = signal_hook::consts::SIGTSTP as usize;
const SIGWINCH: usize = signal_hook::consts::SIGWINCH as usize;
const SIGCONT: usize = signal_hook::consts::SIGCONT as usize;
const SIGTERM: usize = signal_hook::consts::SIGTERM as usize;
const SIGINT: usize = signal_hook::consts::SIGINT as usize;
const SIGUSR1: usize = signal_hook::consts::SIGUSR1 as usize;
const SIGUSR2: usize = signal_hook::consts::SIGUSR2 as usize;


fn main() {
    let mut config = Config {
        plain: false,
        quit: false,
        command: None,
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
    let mut buffer_updated = false;
    let mut countdown = Countdown::new();
    // True if in insert mode.
    let mut insert_mode = false;
    let mut update_menu = true;
    // Child process of exec_command().
    let mut spawned: Option<std::process::Child> = None;

    // Initialise roster_width.
    layout.set_roster_width(alarm_roster.width());

    // Register signal handlers.
    let signal = Arc::new(AtomicUsize::new(0));
    register_signal_handlers(&signal, &layout.force_recalc);
    
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
                // Enter.
                Key::Char('\n') => {
                    if !buffer.is_empty() {
                        if let Err(e) = alarm_roster.add(&buffer) {
                            // Error while processing input buffer.
                            error_msg(&mut stdout, &layout, e);
                        } else {
                            // Input buffer processed without error.
                            layout.set_roster_width(alarm_roster.width());
                            layout.force_redraw = true;
                        }
                        buffer.clear();
                        insert_mode = false;
                        update_menu = true;
                    }
                },
                // Escape ^W, and ^U clear input buffer.
                Key::Esc | Key::Ctrl('w') | Key::Ctrl('u') => {
                    buffer.clear();
                    insert_mode = false;
                    update_menu = true;
                    layout.force_redraw = true;
                    buffer_updated = true;
                },
                // Backspace.
                Key::Backspace => {
                    // Delete last char in buffer.
                    if buffer.pop().is_some() {
                        if buffer.is_empty() {
                            insert_mode = false;
                            update_menu = true;
                            layout.force_redraw = true;
                        }
                    }
                    buffer_updated = true;
                },
                // Forward every char if in insert mode.
                Key::Char(c) if insert_mode => {
                    buffer.push(c);
                    buffer_updated = true;
                },
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
                    if alarm_roster.drop_last() {
                        // If we remove the last alarm we have to reset "countdown"
                        // manually. It is safe to do it anyway.
                        layout.set_roster_width(alarm_roster.width());
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
                Key::Char(c) => {
                    if c.is_ascii_digit() {
                        buffer.push(c);
                        insert_mode = true;
                        update_menu = true;
                        layout.force_redraw = true;
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
            draw_buffer(&mut stdout, &mut layout, &buffer);
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
            if let Some(time) = alarm_roster.check(&mut clock, &layout, &mut countdown) {
                // Write ASCII bell code.
                write!(stdout, "{}", 0x07 as char).unwrap();
                layout.force_redraw = true;

                // Run command if configured.
                if config.command.is_some() {
                    if spawned.is_none() {
                        spawned = exec_command(&config, time);
                    } else {
                        // The last command is still running.
                        eprintln!("Not executing command, as its predecessor is still running");
                    }
                }
                // Quit if configured.
                if config.quit && !alarm_roster.active() {
                    break;
                }
            }

            // Clear the window and redraw menu bar, alarm roster and buffer if
            // requested.
            if layout.force_redraw {
                write!(stdout, "{}", clear::All).unwrap();

                // Redraw list of alarms.
                alarm_roster.draw(&mut stdout, &mut layout);

                // Redraw buffer.
                draw_buffer(&mut stdout, &mut layout, &buffer);

                // Schedule menu redraw.
                update_menu = true;
            }

            if update_menu {
                update_menu = false;
                write!(stdout,
                    "{}{}{}{}",
                    cursor::Goto(1, 1),
                    style::Faint,
                    // Switch menu bars. Use a compressed version or none at
                    // all if necessary.
                    match insert_mode {
                        true if layout.can_hold(MENUBAR_INS) => MENUBAR_INS,
                        false if layout.can_hold(MENUBAR) => MENUBAR,
                        false if layout.can_hold(MENUBAR_SHORT) => MENUBAR_SHORT,
                        _ => "",
                    },
                    style::Reset,)
                    .unwrap();
            }

            clock.draw(&mut stdout, &layout);

            // Display countdown.
            if countdown.value > 0 {
                countdown.draw(&mut stdout);
            }

            // Move cursor to buffer position.
            if insert_mode {
                write!(
                    stdout,
                    "{}",
                    cursor::Goto(layout.cursor.col, layout.cursor.line))
                    .unwrap();
            }

            // Check any spawned child process.
            if let Some(ref mut child) = spawned {
                match child.try_wait() {
                    // Process exited successfully.
                    Ok(Some(status)) if status.success() => spawned = None,
                    // Abnormal exit.
                    Ok(Some(status)) => {
                        eprintln!("Spawned process terminated with non-zero exit status. ({})", status);
                        spawned = None;
                    },
                    // Process is still running.
                    Ok(None) => (),
                    // Other error.
                    Err(error) => {
                        eprintln!("Error executing command. ({})", error);
                        spawned = None;
                    },
                }
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

    // Reset terminal.
    drop(stdout);
    drop(input_keys);

    // Wait for remaining spawned processes to exit.
    if let Some(ref mut child) = spawned {
        print!("Waiting for spawned processes (PID {}) to exit ...", child.id());
        std::io::stdout().flush().unwrap();

        match child.wait() {
            Ok(status) => println!(" ok ({})", status),
            Err(error) => println!(" failed ({})", error),
        }
    }
}

// Print usage information and exit.
fn usage() {
    println!("{}", USAGE);
    std::process::exit(0);
}

// Parse command line arguments into "config".
fn parse_args(config: &mut Config, alarm_roster: &mut AlarmRoster) {
    let mut iter = env::args().skip(1);

    loop {
        if let Some(arg) = iter.next() {
            match arg.as_str() {
                "-h" | "--help" => usage(),
                "-v" | "--version" => {
                    println!("{} {}", NAME, VERSION);
                    std::process::exit(0);
                },
                "-p" | "--plain" => config.plain = true,
                "-q" | "--quit" => config.quit = true,
                "-e" | "--exec" => {
                    if let Some(e) = iter.next() {
                        config.command = Some(parse_to_command(&e));
                    } else {
                        println!("Missing parameter to \"{}\".", arg);
                        std::process::exit(1);
                    }
                },
                any if any.starts_with('-') => {
                    // Unrecognized flag.
                    println!("Unrecognized option: \"{}\"", any);
                    println!("Use \"-h\" or \"--help\" for a list of valid command line options");
                    std::process::exit(1);
                },
                any => {
                    // Alarm to add.
                    if let Err(error) = alarm_roster.add(&String::from(any)) {
                        println!("Error adding \"{}\" as alarm. ({})", any, error);
                        std::process::exit(1);
                    }
                },
            }
        } else { break; } // All command line parameters processed.
    }
}

// Parse command line argument to --command into a vector of strings suitable
// for process::Command::new().
fn parse_to_command(input: &str) -> Vec<String> {
    let mut command: Vec<String> = Vec::new();
    let mut subs: String = String::new();
    let mut quoted = false;
    let mut escaped = false;

    for byte in input.chars() {
        match byte {
            '\\' if !escaped => {
                // Next char is escaped.
                escaped = true;
                continue;
            },
            ' ' if escaped || quoted => { &subs.push(' '); },
            ' ' => {
                if !&subs.is_empty() {
                    command.push(subs.clone());
                    &subs.clear();
                }
            },
            '"' | '\'' if !escaped => quoted = !quoted,
            _ => {
                if escaped { &subs.push('\\'); }
                &subs.push(byte);
            },
        }
        escaped = false;
    }
    command.push(subs);
    command.shrink_to_fit();
    command
}

fn register_signal_handlers(
    signal: &Arc<AtomicUsize>,
    recalc_flag: &Arc<AtomicBool>,
) {
    flag::register_usize(SIGTSTP as i32, Arc::clone(&signal), SIGTSTP).unwrap();
    flag::register_usize(SIGCONT as i32, Arc::clone(&signal), SIGCONT).unwrap();
    flag::register_usize(SIGTERM as i32, Arc::clone(&signal), SIGTERM).unwrap();
    flag::register_usize(SIGINT as i32, Arc::clone(&signal), SIGINT).unwrap();
    flag::register_usize(SIGUSR1 as i32, Arc::clone(&signal), SIGUSR1).unwrap();
    flag::register_usize(SIGUSR2 as i32, Arc::clone(&signal), SIGUSR2).unwrap();
    // SIGWINCH sets "force_recalc" directly.
    flag::register(SIGWINCH as i32, Arc::clone(&recalc_flag)).unwrap();
}

// Prepare to suspend execution. Called on SIGTSTP.
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

    if let Err(error) = low_level::emulate_default_handler(SIGTSTP as i32) {
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
fn draw_buffer<W: Write>(
    stdout: &mut RawTerminal<W>,
    layout: &mut Layout,
    buffer: &String,
) {
    if !buffer.is_empty() {
        write!(stdout,
            "{}{}Add alarm: {}{}",
            cursor::Goto(layout.buffer.col, layout.buffer.line),
            clear::CurrentLine,
            cursor::Show,
            buffer)
            .unwrap();
        layout.cursor.col = layout.buffer.col + 11 + str_length(buffer);
    } else {
        // Clear buffer display.
        write!(stdout,
            "{}{}{}",
            cursor::Goto(layout.buffer.col, layout.buffer.line),
            clear::CurrentLine,
            cursor::Hide)
            .unwrap();
    }
}

// Draw error message.
fn error_msg<W: Write>(stdout: &mut RawTerminal<W>, layout: &Layout, msg: &str) {
    write!(stdout,
        "{}{}{}{}{}",
        cursor::Goto(layout.error.col, layout.error.line),
        color::Fg(color::LightRed),
        msg,
        color::Fg(color::Reset),
        cursor::Hide)
        .unwrap();
}

