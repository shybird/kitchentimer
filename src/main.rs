extern crate termion;
extern crate libc;
mod alarm;
mod clock;
mod common;
mod layout;

use std::{time, thread, env};
use std::io::Write;
use termion::{clear, color, cursor, style};
use termion::raw::{IntoRawMode, RawTerminal};
use termion::event::Key;
use termion::input::TermRead;
use clock::Clock;
use alarm::{Countdown, AlarmRoster, alarm_exec};
use layout::{Layout, Position};


const NAME: &str = "kt (kitchentime)";
const VERSION: &str = "0.0.1";
const USAGE: &str =
"USAGE: kt [-h|--help] [-v|--version] [-p|--plain] [-e|--exec COMMAND [...]]
  -p, --plain           Use simpler block chars.
  -e, --exec [COMMAND]  Execute \"COMMAND\" on alarm. Must be the last flag on
                        the command line. Everything after it is passed as
                        argument to \"COMMAND\". Every \"%s\" will be replaced
                        with the elapsed time in [(HH:)MM:SS] format.";
const MENUBAR: &str =
"[0-9] Add alarm  [d] Delete alarm  [SPACE] Pause  [r] Reset  [c] Clear color  [q] Quit";
const MENUBAR_SHORT: &str =
"[0-9] Add  [d] Delete  [SPACE] Pause  [r] Reset  [c] Clear  [q] Quit";

pub struct Config {
    plain: bool,
    alarm_exec: Vec<String>,
}


fn main() {
    let mut config = Config {
        plain: false,
        alarm_exec: Vec::new(),
    };
    parse_args(&mut config);

    let mut stdout = std::io::stdout().into_raw_mode()
        .unwrap_or_else(|error| {
            eprintln!("Error opening stdout: {}", error);
            std::process::exit(1);
        });
    let mut input_keys = termion::async_stdin().keys();
    let mut layout = Layout::new(&config);
    let mut clock = Clock::new();
    let mut alarm_roster = AlarmRoster::new();
    let mut buffer = String::new();
    let mut buffer_updated: bool = false;
    let mut countdown = Countdown::new();
    
    // Clear screen and hide cursor.
    write!(stdout,
        "{}{}",
        clear::All,
        cursor::Hide)
        .unwrap_or_else(|error| {
            eprintln!("Error writing to stdout: {}", error);
            std::process::exit(1);
        });

    loop {
        // Process input.
        if let Some(key) = input_keys.next() {
            match key.expect("Error reading input") {
                // Reset clock on 'r'.
                Key::Char('r') => {
                    clock.reset();
                    alarm_roster.reset_all();
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
                    layout.force_redraw = true;
                },
                // Enter.
                Key::Char('\n') => {
                    if buffer.len() > 0 {
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
                    } else if buffer.len() > 0 && c == ':' {
                        buffer.push(':');
                        buffer_updated = true;
                    }
                },
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
                // Should never owerflow as we reestablish a new "start"
                // instant every 24 hours.
                clock.start.elapsed().as_secs() as u32
            };

        // Update screen content if necessary.
        if elapsed != clock.elapsed || layout.force_redraw  {
            // Update clock. Advance one day after 24 hours.
            if elapsed < 24 * 60 * 60 {
                clock.elapsed = elapsed;
            } else {
                clock.next_day();
                // "clock.elapsed" set by "clock.next_day()".
                alarm_roster.reset_all();
                layout.force_recalc = true;
            }

            // Force recalculation of layout if we start displaying hours.
            if clock.elapsed == 3600 { layout.force_recalc = true };
            // Update screen size information and calculate the clock position.
            layout.update(clock.elapsed >= 3600);

            // Check for exceeded alarms.
            if alarm_roster.check(&mut clock, &layout, &mut countdown) {
                // Write ASCII bell code.
                write!(stdout, "{}", 0x07 as char).unwrap();
                layout.force_redraw = true;

                if config.alarm_exec.len() > 0 {
                    alarm_exec(&config, clock.elapsed);
                }
            }

            // Clear the screen and redraw menu bar, alarm roster and buffer if
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

    // Main loop exited. Clear screen and restore cursor.
    write!(stdout,
        "{}{}{}",
        clear::BeforeCursor,
        cursor::Goto(1, 1),
        cursor::Show)
        .unwrap();
}

fn usage() {
    println!("{}\n{}", NAME, USAGE);
    std::process::exit(0);
}

// Parse command line arguments into "config".
fn parse_args(config: &mut Config) {
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => usage(),
            "-v" | "--version" => {
                println!("{} {}", NAME, VERSION);
                std::process::exit(0);
            }
            "-p" | "--plain" => { config.plain = true; },
            "-e" | "--exec" => {
                // Find position of this flag.
                let i = env::args().position(|s| { s == "-e" || s == "--exec" }).unwrap();
                // Copy everything thereafter.
                config.alarm_exec = env::args().skip(i + 1).collect();
                if config.alarm_exec.len() == 0 {
                    usage();
                } else {
                    // Ignore everything after this flag.
                    break;
                }
            }
            _ => usage(), // Unrecognized flag.
        }
    }
}

// Suspend execution by raising SIGTSTP.
fn suspend<W: Write>(stdout: &mut RawTerminal<W>) {
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

    let result = unsafe { libc::raise(libc::SIGTSTP) };
    if result != 0 {
        panic!("{}", std::io::Error::last_os_error());
    }

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
    if buffer.len() > 0 {
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

