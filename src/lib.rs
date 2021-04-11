extern crate termion;
pub mod alarm;
mod buffer;
pub mod clock;
pub mod consts;
pub mod layout;
pub mod utils;
#[cfg(test)]
mod tests;

use std::{env, process, thread, time};
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use signal_hook::{flag, low_level};
use termion::{clear, cursor, style};
use termion::raw::{IntoRawMode, RawTerminal};
use termion::event::Key;
use termion::input::TermRead;
use buffer::Buffer;
use clock::Clock;
use layout::Layout;
use alarm::{Countdown, exec_command};
pub use alarm::AlarmRoster;
pub use consts::ui::*;

const SIGTSTP: usize = signal_hook::consts::SIGTSTP as usize;
const SIGWINCH: usize = signal_hook::consts::SIGWINCH as usize;
const SIGCONT: usize = signal_hook::consts::SIGCONT as usize;
const SIGTERM: usize = signal_hook::consts::SIGTERM as usize;
const SIGINT: usize = signal_hook::consts::SIGINT as usize;
const SIGUSR1: usize = signal_hook::consts::SIGUSR1 as usize;
const SIGUSR2: usize = signal_hook::consts::SIGUSR2 as usize;


pub fn run(
    config: Config,
    mut alarm_roster: AlarmRoster,
    signal: Arc<AtomicUsize>,
    sigwinch: Arc<AtomicBool>,
    spawned: &mut Option<process::Child>,
) -> Result<(), std::io::Error>
{
    let mut layout = Layout::new(&config);
    layout.force_recalc = sigwinch;
    // Initialise roster_width.
    layout.set_roster_width(alarm_roster.width());
    let mut clock = Clock::new();
    let mut countdown = Countdown::new();
    let mut buffer = Buffer::new();

    // State variables.
    // Request redraw of menu.
    let mut update_menu = false;
    // Are we in insert mode?
    let mut insert_mode = false;

    let async_stdin = termion::async_stdin();
    let mut input_keys = async_stdin.keys();
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock().into_raw_mode()?;

    // Clear window and hide cursor.
    write!(stdout, "{}{}", clear::All, cursor::Hide)?;

    // Main loop entry.
    loop {
        // Process received signals.
        match signal.swap(0, Ordering::Relaxed) {
            // No signal received.
            0 => (),
            // Suspend execution on SIGTSTP.
            SIGTSTP => {
                suspend(&mut stdout)?;
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
                restore_after_suspend(&mut stdout)?;
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

        // Update input buffer display, if requested.
        /*
        if buffer.altered {
            buffer.draw(&mut stdout, &mut layout)?;
            stdout.flush()?;
        }
        */

        // Update elapsed time.
        let elapsed = if clock.paused {
                clock.elapsed
            } else {
                // Should never overflow as we reestablish a new "start"
                // instant every 24 hours.
                clock.start.elapsed().as_secs() as u32
            };

        // Conditional inner loop. Runs once every second or when explicitly
        // requested.
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
            if let Some((time, label)) = alarm_roster.check(&mut clock, &layout, &mut countdown) {
                // Write ASCII bell code.
                write!(stdout, "{}", 0x07 as char)?;
                layout.force_redraw = true;

                // Run command if configured.
                if config.command.is_some() {
                    if spawned.is_none() {
                        *spawned = exec_command(&config, time, &label);
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
                write!(stdout, "{}", clear::All)?;

                // Redraw list of alarms.
                alarm_roster.draw(&mut stdout, &mut layout);

                // Redraw buffer.
                buffer.draw(&mut stdout, &mut layout)?;

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
                    style::Reset)?;
            }

            clock.draw(&mut stdout, &layout);

            // Display countdown.
            if countdown.value > 0 {
                countdown.draw(&mut stdout);
            }

            // Check any spawned child process.
            if let Some(ref mut child) = spawned {
                match child.try_wait() {
                    // Process exited successfully.
                    Ok(Some(status)) if status.success() => *spawned = None,
                    // Abnormal exit.
                    Ok(Some(status)) => {
                        eprintln!("Spawned process terminated with non-zero exit status. ({})", status);
                        *spawned = None;
                    },
                    // Process is still running.
                    Ok(None) => (),
                    // Other error.
                    Err(error) => {
                        eprintln!("Error executing command. ({})", error);
                        *spawned = None;
                    },
                }
            }

            // End of conditional inner loop.
            // Reset redraw_all and flush stdout.
            layout.force_redraw = false;
            stdout.flush()?;
        }

        // Update buffer whenever the cursor is visible.
        if insert_mode || buffer.altered {
            buffer.draw(&mut stdout, &mut layout)?;
            stdout.flush()?;
        }

        // Process input.
        if let Some(key) = input_keys.next() {
            match key.expect("Error reading input") {
                // Enter.
                Key::Char('\n') => {
                    if !buffer.is_empty() {
                        if let Err(e) = alarm_roster.add(buffer.read()) {
                            // Error while processing input buffer.
                            buffer.message(e);
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
                Key::Esc | Key::Ctrl('u') => {
                    buffer.reset();
                    insert_mode = false;
                    update_menu = true;
                    layout.force_redraw = true;
                },
                // ^W removes last word.
                Key::Ctrl('w') => {
                    if !buffer.strip_word() {
                        insert_mode = false;
                        update_menu = true;
                        layout.force_redraw = true;
                    }
                },
                // Backspace.
                Key::Backspace => {
                    // Delete last char in buffer.
                    if buffer.strip_char() && buffer.is_empty() {
                        insert_mode = false;
                        update_menu = true;
                        layout.force_redraw = true;
                    }
                },
                // Forward every char if in insert mode.
                Key::Char(c) if insert_mode => {
                    buffer.push(c);
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
                    suspend(&mut stdout)?;
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
                    } else if !buffer.is_empty() && c == ':' {
                        buffer.push(':');
                    }
                },
                // Any other key.
                _ => (),
            }
        } else {
            // Main loop delay.
            thread::sleep(time::Duration::from_millis(100));
        }
    }

    // Main loop exited. Clear window and restore cursor.
    write!(stdout,
        "{}{}{}",
        clear::BeforeCursor,
        cursor::Goto(1, 1),
        cursor::Show)?;

    Ok(())
}

pub struct Config {
    plain: bool,
    quit: bool,
    command: Option<Vec<String>>,
}

impl Config {
    // Parse command line arguments into "config".
    pub fn new(args: env::Args, alarm_roster: &mut AlarmRoster)
        -> Result<Config, String>
    {
        let mut config = Config {
            plain: false,
            quit: false,
            command: None,
        };
        let mut iter = args.skip(1);

        loop {
            if let Some(arg) = iter.next() {
                match arg.as_str() {
                    "-h" | "--help" => {
                        // Print usage information and exit
                        println!("{}", USAGE);
                        process::exit(0);
                    },
                    "-v" | "--version" => {
                        println!("{} {}", NAME, VERSION);
                        process::exit(0);
                    },
                    "-p" | "--plain" => config.plain = true,
                    "-q" | "--quit" => config.quit = true,
                    "-e" | "--exec" => {
                        if let Some(e) = iter.next() {
                            config.command = Some(Config::parse_to_command(&e));
                        } else {
                            return Err(format!("Missing parameter to \"{}\".", arg));
                        }
                    },
                    any if any.starts_with('-') => {
                        // Unrecognized flag.
                        return Err(format!("Unrecognized option: \"{}\"\nUse \"-h\" or \"--help\" for a list of valid command line options.", any));
                    },
                    any => {
                        // Alarm to add.
                        if let Err(error) = alarm_roster.add(&String::from(any)) {
                            return Err(format!("Error adding \"{}\" as alarm. ({})", any, error));
                        }
                    },
                }
            } else { break; } // All command line parameters processed.
        }
        Ok(config)
    }

    // Parse command line argument to --command into a vector of strings suitable
    // for process::Command::new().
    fn parse_to_command(input: &str) -> Vec<String> {
        let mut command: Vec<String> = Vec::new();
        let mut buffer: String = String::new();
        let mut quoted = false;
        let mut escaped = false;

        for byte in input.chars() {
            match byte {
                '\\' if !escaped => {
                    // Next char is escaped.
                    escaped = true;
                    continue;
                },
                ' ' if escaped || quoted => { &buffer.push(' '); },
                ' ' => {
                    if !&buffer.is_empty() {
                        command.push(buffer.clone());
                        &buffer.clear();
                    }
                },
                '"' | '\'' if !escaped => quoted = !quoted,
                _ => {
                    if escaped { &buffer.push('\\'); }
                    &buffer.push(byte);
                },
            }
            escaped = false;
        }
        command.push(buffer);
        command.shrink_to_fit();
        command
    }
}

pub fn register_signals(
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
fn suspend<W: Write>(mut stdout: &mut RawTerminal<W>)
    -> Result<(), std::io::Error>
{
    write!(stdout,
        "{}{}{}",
        cursor::Goto(1,1),
        clear::All,
        cursor::Show)?;
    stdout.flush()?;
    stdout.suspend_raw_mode()
        .unwrap_or_else(|error| {
            eprintln!("Failed to leave raw terminal mode prior to suspend: {}", error);
        });

    if let Err(error) = low_level::emulate_default_handler(SIGTSTP as i32) {
        eprintln!("Error raising SIGTSTP: {}", error);
    }

    restore_after_suspend(&mut stdout)
}

// Set up terminal after SIGTSTP or SIGSTOP.
fn restore_after_suspend<W: Write>(stdout: &mut RawTerminal<W>)
    -> Result<(), std::io::Error>
{
    stdout.activate_raw_mode()
        .unwrap_or_else(|error| {
            eprintln!("Failed to re-enter raw terminal mode after suspend: {}", error);
            process::exit(1);
        });
    write!(stdout,
        "{}{}",
        clear::All,
        cursor::Hide)?;
    Ok(())
}

