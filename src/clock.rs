pub mod font;

use std::time;
use std::io::Write;
use termion::{color, cursor, style};
use termion::raw::RawTerminal;
use crate::consts::COLOR;
use crate::Config;
use crate::layout::{Layout, Position};

pub struct Clock {
    pub start: time::Instant,
    pub elapsed: u32,
    pub days: u32,
    pub paused: bool,
    paused_at: Option<time::Instant>,
    pub color_index: Option<usize>,
    pub font: &'static font::Font,
}

impl Clock {
    pub fn new(config: &Config) -> Clock {
        Clock {
            start: time::Instant::now(),
            elapsed: 0,
            days: 0,
            paused: false,
            paused_at: None,
            color_index: None,
            font: config.font,
        }
    }

    pub fn reset(&mut self) {
        self.start = time::Instant::now();
        self.elapsed = 0;
        self.days = 0;
        self.color_index = None;

        // unpause will panic if we do not trigger a new pause here.
        if self.paused {
            self.pause();
        }
    }

    fn pause(&mut self) {
        self.paused_at = Some(time::Instant::now());
        self.paused = true;
    }

    fn unpause(&mut self) {
        // Try to derive a new start instant.
        if let Some(delay) = self.paused_at {
            if let Some(new_start) = self.start.checked_add(delay.elapsed()) {
                self.start = new_start;
            }
        }

        self.paused_at = None;
        self.paused = false;
    }

    pub fn toggle(&mut self) {
        if self.paused {
            self.unpause();
        } else {
            self.pause();
        }
    }

    pub fn get_width(&self) -> u16 {
        if self.elapsed >= 3600 {
            // Hours
            self.font.width * 6 + 3 + 10
        } else {
            // Minutes and seconds only.
            self.font.width * 4 + 2 + 5
        }
    }

    pub fn next_day(&mut self) {
        // Shift start 24h into the future.
        let next = self.start.clone() + time::Duration::from_secs(60 * 60 * 24);

        // Take care not to shift start into the future.
        if next <= time::Instant::now() {
            self.start = next;
            self.elapsed = 0;
            self.days = self.days.saturating_add(1);
        }
    }

    // Draw clock according to layout.
    pub fn draw<W: Write>(
        &self,
        mut stdout: &mut RawTerminal<W>,
        layout: &Layout,
    ) -> Result<(), std::io::Error>
    {
        // Setup style and color if appropriate.
        if self.paused {
            write!(stdout, "{}", style::Faint)?;
        }
        if let Some(c) = self.color_index {
            write!(stdout, "{}", color::Fg(COLOR[c]))?;
        }

        // Run once every hour or on request.
        if layout.force_redraw || self.elapsed % 3600 == 0 {
            // Draw hours if necessary.
            if self.elapsed >= 3600 {
                self.draw_digit_pair(
                    &mut stdout,
                    self.elapsed / 3600,
                    &layout.clock_hr,
                )?;

                // Draw colon.
                self.draw_colon(
                    &mut stdout,
                    &layout.clock_colon1,
                )?;
            }

            // Draw days.
            if self.days > 0 {
                let day_count = format!(
                    "+ {} {}",
                    self.days,
                    if self.days == 1 { "DAY" } else { "DAYS" },
                );

                write!(stdout,
                    "{}{:>11}",
                    cursor::Goto(
                        layout.clock_days.col,
                        layout.clock_days.line,
                    ),
                    day_count,
                )?;
            }
        }

        // Draw minutes if necessary. Once every minute or on request.
        if layout.force_redraw || self.elapsed % 60 == 0 {
            self.draw_digit_pair(
                &mut stdout,
                (self.elapsed % 3600) / 60,
                &layout.clock_min,
            )?;
        }

        // Draw colon if necessary.
        if layout.force_redraw {
            self.draw_colon(
                &mut stdout,
                &layout.clock_colon0,
            )?;
        }

        // Draw seconds.
        self.draw_digit_pair(
            &mut stdout,
            self.elapsed % 60,
            &layout.clock_sec,
        )?;

        // Reset color and style.
        if self.paused || self.color_index != None {
            write!(stdout,
                "{}{}",
                style::NoFaint,
                color::Fg(color::Reset),
            )?;
        }
        Ok(())
    }

    fn draw_digit_pair<W: Write>(
        &self,
        stdout: &mut RawTerminal<W>,
        value: u32,
        pos: &Position,
    ) -> Result<(), std::io::Error>
    {
        let left = self.font.digits[value as usize / 10].iter();
        let right = self.font.digits[value as usize % 10].iter();

        for (i, (left, right)) in left.zip(right).enumerate() {
            write!(stdout,
                "{}{} {}",
                cursor::Goto(pos.col, pos.line + i as u16),
                left,
                right,
            )?;
        }

        Ok(())
    }

    fn draw_colon<W: Write>(
        &self,
        stdout: &mut RawTerminal<W>,
        pos: &Position,
    ) -> Result<(), std::io::Error>
    {
        write!(stdout,
            "{}{}{}{}",
            cursor::Goto(pos.col, pos.line + 1),
            self.font.dot,
            cursor::Goto(pos.col, pos.line + 3),
            self.font.dot,
        )?;
        Ok(())
    }
}

