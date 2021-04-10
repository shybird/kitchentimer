use std::time;
use std::io::Write;
use termion::{color, cursor};
use termion::raw::RawTerminal;
use crate::consts::*;
use crate::Layout;
use crate::Position;


pub struct Clock {
    pub start: time::Instant,
    pub elapsed: u32,
    pub days: u32,
    pub paused: bool,
    paused_at: Option<time::Instant>,
    pub color_index: Option<usize>,
}

impl Clock {
    pub fn new() -> Clock {
        Clock {
            start: time::Instant::now(),
            elapsed: 0,
            days: 0,
            paused: false,
            paused_at: None,
            color_index: None,
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
        &mut self,
        mut stdout: &mut RawTerminal<W>,
        layout: &Layout,
    ) {
        // Draw hours if necessary.
        if layout.force_redraw || self.elapsed % 3600 == 0 {
            if self.elapsed >= 3600 {
                self.draw_digit_pair(
                    &mut stdout,
                    self.elapsed / 3600,
                    &layout.clock_hr,
                    layout.plain);

                // Draw colon.
                self.draw_colon(
                    &mut stdout,
                    &layout.clock_colon1,
                    layout.plain);
            }

            // Draw days.
            if self.days > 0 {
                let day_count = format!(
                    "+ {} {}",
                    self.days,
                    if self.days == 1 { "DAY" } else { "DAYS" });

                write!(stdout,
                    "{}{:>11}",
                    cursor::Goto(
                        layout.clock_days.col,
                        layout.clock_days.line,
                    ),
                    day_count)
                    .unwrap();
            }
        }

        // Draw minutes if necessary.
        if layout.force_redraw || self.elapsed % 60 == 0 {
            self.draw_digit_pair(
                &mut stdout,
                (self.elapsed % 3600) / 60,
                &layout.clock_min,
                layout.plain);
        }

        // Draw colon if necessary.
        if layout.force_redraw {
            self.draw_colon(
                &mut stdout,
                &layout.clock_colon0,
                layout.plain);
        }

        // Draw seconds.
        self.draw_digit_pair(
            &mut stdout,
            self.elapsed % 60,
            &layout.clock_sec,
            layout.plain);
    }

    fn draw_digit_pair<W: Write>(
        &self,
        stdout: &mut RawTerminal<W>,
        value: u32,
        pos: &Position,
        plain: bool,
    ) {
        if let Some(c) = self.color_index {
            write!(stdout,
                "{}{}",
                cursor::Goto(pos.col, pos.line),
                color::Fg(COLOR[c]))
                .unwrap();
        }

        for l in 0..DIGIT_HEIGHT {
            if plain {
                write!(stdout,
                    "{}{} {}",
                    cursor::Goto(pos.col, pos.line + l),
                    // First digit.
                    DIGITS_PLAIN[(value / 10) as usize][l as usize],
                    // Second digit.
                    DIGITS_PLAIN[(value % 10) as usize][l as usize])
                    .unwrap();
            } else {
                write!(stdout,
                    "{}{} {}",
                    cursor::Goto(pos.col, pos.line + l),
                    // First digit.
                    DIGITS[(value / 10) as usize][l as usize],
                    // Second digit.
                    DIGITS[(value % 10) as usize][l as usize])
                    .unwrap();
            }
        }

        if self.color_index != None {
            write!(stdout,
                "{}{}",
                cursor::Goto(pos.col + DIGIT_WIDTH + 1, pos.line + DIGIT_HEIGHT),
                color::Fg(color::Reset))
                .unwrap();
        }
    }

    fn draw_colon<W: Write>(
        &self,
        stdout: &mut RawTerminal<W>,
        pos: &Position,
        plain: bool,
    ) {
        let dot = if plain {'█'} else {'■'};

        match self.color_index {
            Some(c) =>  {
                write!(stdout,
                    "{}{}{}{}{}{}",
                    cursor::Goto(pos.col, pos.line + 1),
                    color::Fg(COLOR[c]),
                    dot,
                    cursor::Goto(pos.col, pos.line + 3),
                    dot,
                    color::Fg(color::Reset))
                    .unwrap();
            }
            None =>  {
                write!(stdout,
                    "{}{}{}{}",
                    cursor::Goto(pos.col, pos.line + 1),
                    dot,
                    cursor::Goto(pos.col, pos.line + 3),
                    dot)
                    .unwrap();
            }
        }
    }
}

