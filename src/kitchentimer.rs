extern crate termion;

use std::{process, thread, time};
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use signal_hook::low_level;
use termion::{clear, color, cursor, style};
use termion::raw::{IntoRawMode, RawTerminal};
use termion::event::Key;
use termion::input::TermRead;
//use termion::cursor::DetectCursorPos;
use crate::clock::Clock;
use crate::alarm::{Countdown, AlarmRoster, exec_command};
use crate::layout::Layout;
use crate::consts::*;
use crate::utils::*;
use crate::Config;

pub fn kitchentimer(
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
    let mut buffer = String::new();

    // State variables.
    let mut update_buffer = false;
    let mut update_menu = false;
    let mut insert_mode = false;

    let async_stdin = termion::async_stdin();
    let mut input_keys = async_stdin.keys();
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock().into_raw_mode()
        .unwrap_or_else(|error| {
            eprintln!("Error opening stdout: {}", error);
            process::exit(1);
        });

    // Clear window and hide cursor.
    write!(stdout,
        "{}{}",
        clear::All,
        cursor::Hide)
        .unwrap_or_else(|error| {
            eprintln!("Error writing to stdout: {}", error);
            process::exit(1);
        });

    // Enter main loop.
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
                            error_msg(&mut stdout, &layout, e)?;
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
                    update_buffer = true;
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
                    update_buffer = true;
                },
                // Forward every char if in insert mode.
                Key::Char(c) if insert_mode => {
                    buffer.push(c);
                    update_buffer = true;
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
                        update_buffer = true;
                    } else if !buffer.is_empty() && c == ':' {
                        buffer.push(':');
                        update_buffer = true;
                    }
                },
                // Any other key.
                _ => (),
            }
        } else {
            // Main loop delay.
            thread::sleep(time::Duration::from_millis(200));
        }

        // Update input buffer display.
        if update_buffer {
            draw_buffer(&mut stdout, &mut layout, &buffer)?;
            update_buffer = false;
            stdout.flush()?;
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
                draw_buffer(&mut stdout, &mut layout, &buffer)?;

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

            // Move cursor to buffer position.
            if insert_mode {
                write!(
                    stdout,
                    "{}",
                    cursor::Goto(layout.cursor.col, layout.cursor.line))?;
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

            // Reset redraw_all and flush stdout.
            layout.force_redraw = false;
            stdout.flush()?;
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

// Draw input buffer.
fn draw_buffer<W: Write>(
    stdout: &mut RawTerminal<W>,
    layout: &mut Layout,
    buffer: &String,
) -> Result<(), std::io::Error>
{
    if !buffer.is_empty() {
        write!(stdout,
            "{}{}Add alarm: {}{}",
            cursor::Goto(layout.buffer.col, layout.buffer.line),
            clear::CurrentLine,
            cursor::Show,
            buffer)?;
        layout.cursor.col = layout.buffer.col + 11 + unicode_length(buffer);
        // TODO: This would be a much better alternative, but panics because
        // of interference with async_reader.
        //layout.cursor.col = stdout.cursor_pos()?.0;
    } else {
        // Clear buffer display.
        write!(stdout,
            "{}{}{}",
            cursor::Goto(layout.buffer.col, layout.buffer.line),
            clear::CurrentLine,
            cursor::Hide)?;
    }
    Ok(())
}

// Draw error message at input buffer position.
fn error_msg<W: Write>(
    stdout: &mut RawTerminal<W>,
    layout: &Layout,
    msg: &str
) -> Result<(), std::io::Error>
{
    write!(stdout,
        "{}{}{}{}{}",
        cursor::Goto(layout.error.col, layout.error.line),
        color::Fg(color::LightRed),
        msg,
        color::Fg(color::Reset),
        cursor::Hide)?;
    Ok (())
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

    restore_after_suspend(&mut stdout);
    Ok(())
}

// Set up terminal after SIGTSTP or SIGSTOP.
fn restore_after_suspend<W: Write>(stdout: &mut RawTerminal<W>) {
    stdout.activate_raw_mode()
        .unwrap_or_else(|error| {
            eprintln!("Failed to re-enter raw terminal mode after suspend: {}", error);
            process::exit(1);
        });
    write!(stdout,
        "{}{}",
        clear::All,
        cursor::Hide)
        .unwrap_or_else(|error| {
            eprintln!("Error writing to stdout: {}", error);
            process::exit(1);
        });
}

