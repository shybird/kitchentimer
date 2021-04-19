// Copyright 2021, Shy.
//
// This file is part of Kitchentimer.
//
// Kitchentimer is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Kitchentimer is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Kitchentimer.  If not, see <https://www.gnu.org/licenses/>.

extern crate signal_hook;
extern crate termion;
mod alarm;
mod buffer;
mod clock;
mod consts;
mod cradle;
mod layout;
#[cfg(test)]
mod tests;
mod utils;

pub use alarm::AlarmRoster;
use alarm::Countdown;
use buffer::Buffer;
use clock::{font, Clock};
pub use consts::ui::*;
use cradle::Cradle;
use layout::Layout;
use signal_hook::consts::signal::*;
use signal_hook::iterator::Signals;
use signal_hook::low_level;
use std::io::Write;
use std::{env, process, thread, time};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};
use termion::{clear, cursor, style};

pub fn run(
    mut config: Config,
    mut alarm_roster: AlarmRoster,
) -> Result<(), std::io::Error> {
    let mut layout = Layout::new();
    // Initialise roster_width.
    layout.set_roster_width(alarm_roster.width());
    let mut clock = Clock::new(&config);
    let mut countdown = Countdown::new();
    let mut buffer = Buffer::new();

    let async_stdin = termion::async_stdin();
    let mut input_keys = async_stdin.keys();
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock().into_raw_mode()?;
    let mut force_redraw = true;

    // Register signals.
    let mut signals = Signals::new(&[
        SIGTSTP, SIGCONT, SIGWINCH, SIGTERM, SIGINT, SIGUSR1, SIGUSR2,
    ])?;

    // Main loop entry.
    loop {
        // Process received signals.
        'outer: for signal in signals.pending() {
            match signal {
                // Suspend execution on SIGTSTP.
                SIGTSTP => suspend(&mut stdout)?,
                // Continuing after SIGTSTP or SIGSTOP.
                SIGCONT => {
                    restore_after_suspend(&mut stdout)?;
                    force_redraw = true;
                }
                SIGWINCH => layout.schedule_recalc(),
                // Exit main loop on SIGTERM and SIGINT.
                SIGTERM | SIGINT => break 'outer,
                // Reset clock on SIGUSR1.
                SIGUSR1 => {
                    clock.reset();
                    alarm_roster.reset_all();
                    layout.schedule_recalc();
                    force_redraw = true;
                }
                // (Un-)Pause clock on SIGUSR2.
                SIGUSR2 => {
                    clock.toggle();
                    force_redraw = true;
                }
                // We didn't register anything else.
                _ => unreachable!(),
            }
        }

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
        if elapsed != clock.elapsed || force_redraw {
            // Update clock. Advance one day after 24 hours.
            if elapsed < 24 * 60 * 60 {
                clock.elapsed = elapsed;
            } else {
                clock.next_day();
                // "clock.elapsed" set by "clock.next_day()".
                alarm_roster.reset_all();
                layout.schedule_recalc();
            }

            // Update window size information and calculate the clock position.
            // Also enforce recalculation of layout if we start displaying
            // hours.
            match layout.update(&clock, clock.elapsed == 3600) {
                Ok(true) => force_redraw = true,
                Ok(false) => (),
                Err(e) => return Err(e),
            }

            // Check on last spawned child process prior to processing the
            // alarm roster and possibly spawning a new set.
            config.commands.tend();

            // Check for exceeded alarms.
            if let Some(alarm) =
                alarm_roster.check(&mut clock, &layout, &mut countdown, force_redraw)
            {
                // Do not react to exceeded alarms if the clock is paused.
                if !clock.paused {
                    force_redraw = true;

                    // Write ASCII bell code.
                    write!(stdout, "{}", 0x07 as char)?;

                    // Run commands.
                    config.commands.run_all(alarm.time, &alarm.label);

                    // Quit if configured.
                    if config.quit && alarm_roster.idle() {
                        break;
                    };
                }
            }

            // Clear the window and redraw menu bar, alarm roster and buffer if
            // requested.
            if force_redraw {
                // Write menu at the top.
                write!(
                    stdout,
                    "{}{}{}{}{}",
                    cursor::Goto(1, 1),
                    style::Faint,
                    // Switch menu bars. Use a compressed version or none at
                    // all if necessary.
                    match buffer.visible {
                        _ if clock.paused && layout.can_hold(MENUBAR_PAUSED) => MENUBAR_PAUSED,
                        true if layout.can_hold(MENUBAR_INS) => MENUBAR_INS,
                        false if layout.can_hold(MENUBAR) => MENUBAR,
                        false if layout.can_hold(MENUBAR_SHORT) => MENUBAR_SHORT,
                        // Clearing the screen from position 1, 1 seems to have
                        // unwanted side effects. We avoid this by writing a
                        // single space here.
                        _ => " ",
                    },
                    clear::AfterCursor,
                    style::NoFaint
                )?;

                // Redraw list of alarms.
                alarm_roster.draw(&mut stdout, &mut layout, &config)?;

                // Redraw buffer.
                buffer.draw(&mut stdout, &mut layout)?;
            }

            clock.draw(&mut stdout, &layout, force_redraw)?;

            // Display countdown.
            if countdown.value > 0 {
                countdown.draw(&mut stdout)?;
            }

            // End of conditional inner loop.
            // Reset redraw_all and flush stdout.
            force_redraw = false;
            stdout.flush()?;
        }

        // Update buffer whenever the cursor should be visible.
        if buffer.visible {
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
                        }
                        buffer.clear();
                        buffer.visible = false;
                        force_redraw = true;
                    }
                }
                // Escape and ^U clear input buffer.
                Key::Esc | Key::Ctrl('u') => {
                    buffer.reset();
                    buffer.visible = false;
                    force_redraw = true;
                }
                // ^W removes last word.
                Key::Ctrl('w') => {
                    buffer.strip_word();
                    if buffer.is_empty() {
                        buffer.visible = false;
                        force_redraw = true;
                    }
                }
                // Backspace.
                Key::Backspace => {
                    // Delete last char in buffer.
                    buffer.strip_char();
                    if buffer.is_empty() {
                        buffer.visible = false;
                        force_redraw = true;
                    }
                }
                // Set clock.
                Key::Up if clock.paused => {
                    clock.shift(10);
                    // We would very likely not detect us passing the hour
                    // barrier and would panic when trying to draw hours
                    // without position if we do not schedule a recalculation
                    // here.
                    layout.schedule_recalc();
                    force_redraw = true;
                }
                Key::Down if clock.paused => {
                    clock.shift(-10);
                    alarm_roster.time_travel(&mut clock);
                    layout.schedule_recalc();
                    force_redraw = true;
                }
                // Scroll alarm roster.
                Key::PageUp => {
                    alarm_roster.scroll_up(&layout);
                    force_redraw = true;
                }
                Key::PageDown => {
                    alarm_roster.scroll_down(&layout);
                    force_redraw = true;
                }
                // Forward every char if in insert mode.
                Key::Char(c) if buffer.visible => {
                    buffer.push(c);
                }
                // Reset clock on 'r'.
                Key::Char('r') => {
                    clock.reset();
                    alarm_roster.reset_all();
                    layout.schedule_recalc();
                    force_redraw = true;
                }
                // (Un-)Pause on space.
                Key::Char(' ') => {
                    clock.toggle();
                    force_redraw = true;
                }
                // Clear clock color on 'c'.
                Key::Char('c') => {
                    clock.color_index = None;
                    force_redraw = true;
                }
                // Delete last alarm on 'd'.
                Key::Char('d') => {
                    if alarm_roster.pop().is_some() {
                        // If we remove the last alarm we have to reset "countdown"
                        // manually. It is safe to do it anyway.
                        layout.set_roster_width(alarm_roster.width());
                        countdown.reset();
                        force_redraw = true;
                    }
                }
                // Exit on q and ^C.
                Key::Char('q') | Key::Ctrl('c') => break,
                // Force redraw on ^R.
                Key::Ctrl('r') => force_redraw = true,
                // Suspend an ^Z.
                Key::Ctrl('z') => {
                    suspend(&mut stdout)?;
                    // Clear SIGCONT, as we have already taken care to reset
                    // the terminal.
                    //signal.compare_and_swap(SIGCONT, 0, Ordering::Relaxed);
                    force_redraw = true;
                    // Jump to the start of the main loop.
                    continue;
                }
                Key::Char(c) => {
                    if c.is_ascii_digit() {
                        buffer.push(c);
                        buffer.visible = true;
                        force_redraw = true;
                    } else if !buffer.is_empty() && c == ':' {
                        buffer.push(':');
                    }
                }
                // Any other key.
                _ => (),
            }
        } else {
            // Main loop delay. Skipped after key press.
            thread::sleep(time::Duration::from_millis(100));
        }
    }

    // Main loop exited. Clear screen and restore cursor.
    write!(stdout, "{}{}{}", clear::All, cursor::Restore, cursor::Show)?;
    stdout.flush()?;

    Ok(())
}

pub struct Config {
    quit: bool,
    fancy: bool,
    font: &'static font::Font,
    commands: Cradle,
}

impl Config {
    // Parse command line arguments into "config".
    pub fn new(
        args: env::Args,
        alarm_roster: &mut AlarmRoster
    ) -> Result<Config, String> {
        let mut config = Config {
            quit: false,
            fancy: false,
            font: &font::NORMAL,
            commands: Cradle::new(),
        };
        let mut iter = args.skip(1);

        loop {
            if let Some(arg) = iter.next() {
                match arg.as_str() {
                    "-h" | "--help" => {
                        // Print usage information and exit
                        println!("{}", USAGE);
                        process::exit(0);
                    }
                    "-v" | "--version" => {
                        println!("{} {}", NAME, VERSION);
                        process::exit(0);
                    }
                    "-p" | "--plain" => config.font = &font::PLAIN,
                    "-f" | "--fancy" => {
                        config.fancy = true;
                        config.font = &font::CHROME;
                    }
                    "-q" | "--quit" => config.quit = true,
                    "-e" | "--exec" => {
                        if let Some(cmd) = iter.next() {
                            //config.command = Some(Config::parse_to_command(&e));
                            config.commands.add(Cradle::parse(&cmd));
                        } else {
                            return Err(format!("Missing parameter to \"{}\".", arg));
                        }
                    }
                    any if any.starts_with('-') => {
                        // Unrecognized flag.
                        return Err(format!("Unrecognized option: \"{}\"\nUse \"-h\" or \"--help\" for a list of valid command line options.", any));
                    }
                    any => {
                        // Alarm to add.
                        if let Err(error) = alarm_roster.add(&String::from(any)) {
                            return Err(format!("Error adding \"{}\" as alarm. ({})", any, error));
                        }
                    }
                }
            } else {
                break;
            } // All command line parameters processed.
        }
        Ok(config)
    }
}

// Prepare to suspend execution. Called on SIGTSTP.
fn suspend<W: Write>(stdout: &mut RawTerminal<W>) -> Result<(), std::io::Error> {
    write!(
        stdout,
        "{}{}{}",
        cursor::Goto(1, 1),
        clear::AfterCursor,
        cursor::Show
    )?;
    stdout.flush()?;
    stdout.suspend_raw_mode().unwrap_or_else(|error| {
        eprintln!(
            "Failed to leave raw terminal mode prior to suspend: {}",
            error
        );
    });

    if let Err(error) = low_level::emulate_default_handler(SIGTSTP as i32) {
        eprintln!("Error raising SIGTSTP: {}", error);
    }

    //restore_after_suspend(&mut stdout)
    Ok(())
}

// Set up terminal after SIGTSTP or SIGSTOP.
fn restore_after_suspend<W: Write>(
    stdout: &mut RawTerminal<W>
) -> Result<(), std::io::Error> {
    stdout.activate_raw_mode().unwrap_or_else(|error| {
        eprintln!(
            "Failed to re-enter raw terminal mode after suspend: {}",
            error
        );
        process::exit(1);
    });
    Ok(())
}
