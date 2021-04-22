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

use crate::clock::Clock;
use crate::consts::{COLOR, LABEL_SIZE_LIMIT};
use crate::layout::{Layout, Position};
use crate::utils::*;
use std::io::BufRead;
use std::io::Write;
use termion::raw::RawTerminal;
use termion::{color, cursor, style};
use unicode_width::UnicodeWidthStr;

// Delimiter between time and label. Remember to update usage information in
// consts.rs when changing this.
const DELIMITER: char = '/';

pub struct Countdown {
    pub value: u32,
    position: Option<Position>,
}

impl Countdown {
    pub fn new() -> Countdown {
        Countdown {
            value: 0,
            position: None,
        }
    }

    pub fn reset(&mut self) {
        self.value = 0;
        self.position = None;
    }

    pub fn set(&mut self, value: u32) {
        self.value = value;
    }

    pub fn has_position(&self) -> bool {
        self.position.is_some()
    }

    // Draw countdown.
    pub fn draw<W: Write>(
        &self,
        stdout: &mut RawTerminal<W>
    ) -> Result<(), std::io::Error> {
        if let Some(pos) = &self.position {
            if self.value < 3600 {
                // Show minutes and seconds.
                write!(
                    stdout,
                    "{}(-{:02}:{:02})",
                    cursor::Goto(pos.col, pos.line),
                    (self.value / 60) % 60,
                    self.value % 60
                )?;
                if self.value == 3599 {
                    // Write three additional spaces after switching from hour display to
                    // minute display.
                    write!(stdout, "   ")?;
                }
            } else {
                // Show hours, minutes and seconds.
                write!(
                    stdout,
                    "{}(-{:02}:{:02}:{:02})",
                    cursor::Goto(pos.col, pos.line),
                    self.value / 3600,
                    (self.value / 60) % 60,
                    self.value % 60
                )?;
            }
        }
        Ok(())
    }

    // Compute position.
    pub fn place(
        &mut self,
        layout: &Layout,
        alarm: &Alarm,
        offset: usize,
        index: usize
    ) {
        let mut col = layout.roster.col + 3 + UnicodeWidthStr::width(alarm.label.as_str()) as u16;
        let mut line = layout.roster.line + index as u16;

        // Compensate for "hidden" items in the alarm roster.
        if offset > 0 {
            if index <= offset {
                // Draw next to upper placeholder.
                line = layout.roster.line;
                col = layout.roster.col + 6;
            } else {
                // Should be no problem as index > offset and
                // line is x + index.
                line -= offset as u16;
            }
        }
        if line > layout.roster_height.saturating_add(2) {
            // Draw next to lower placeholder.
            line = layout.roster.line + layout.roster_height;
            col = layout.roster.col + 6;
        }
        self.position = Some(Position { col, line });
    }
}

pub struct Alarm {
    pub time: u32,
    pub label: String,
    color_index: usize,
    exceeded: bool,
}

impl Alarm {
    fn reset(&mut self) {
        self.exceeded = false;
    }
}

pub struct AlarmRoster {
    list: Vec<Alarm>,
    offset: usize,
    hints_shown: bool,
}

impl AlarmRoster {
    pub fn new() -> AlarmRoster {
        AlarmRoster {
            list: Vec::new(),
            // Scrolling offset.
            offset: 0,
            // Scrolling hint.
            hints_shown: false,
        }
    }

    // Parse string and add as alarm.
    pub fn add(&mut self, input: &String) -> Result<(), &'static str> {
        let mut time: u32 = 0;
        let mut label: String;
        let time_str: &str;

        if let Some(i) = input.find(DELIMITER) {
            label = input[(i + 1)..].to_string();
            // Truncate label.
            grapheme_truncate(&mut label, LABEL_SIZE_LIMIT, '…');
            time_str = &input[..i].trim();
        } else {
            label = input.clone();
            time_str = &input.trim();
        }

        // Parse input into seconds.
        for (i, sub) in time_str.rsplit(':').enumerate() {
            match sub.parse::<u32>() {
                // Too many segments.
                Ok(_) if i > 2 => return Err("Too many segments to parse as time."),
                // Valid.
                Ok(d) => time += d * 60u32.pow(i as u32),
                // Ignore failure caused by an empty string.
                // TODO: Match error kind when stable. See documentation
                // for std::num::ParseIntError and
                // https://github.com/rust-lang/rust/issues/22639
                Err(_) if sub.is_empty() => (),
                // Could not parse to u32.
                Err(_) => return Err("Could not parse value as integer."),
            }
        }

        // Skip if time is out of boundaries.
        if time == 0 {
            return Err("Evaluates to zero.");
        };
        if time >= 24 * 60 * 60 {
            return Err("Values >24h not supported.");
        };
        // Filter out duplicate entries.
        if self.list.iter().any(|a| a.time == time) {
            return Err("Already exists. Duplicate entries not supported.");
        }

        // Label will never change from now on.
        label.shrink_to_fit();
        let alarm = Alarm {
            label,
            time,
            color_index: (self.list.len() % COLOR.len()),
            exceeded: false,
        };

        // Add to list, insert based on alarm time.
        if let Some(i) = self.list.iter().position(|a| a.time > time) {
            self.list.insert(i, alarm);
        } else {
            self.list.push(alarm);
        }
        Ok(())
    }

    // Remove last alarm.
    pub fn pop(&mut self) -> Option<Alarm> {
        self.list.pop()
    }

    // Offset ceiling according to layout information.
    fn adjust_offset(&mut self, layout: &Layout) {
        self.offset = self.offset.min(
            self.list
                .len()
                .saturating_sub(layout.roster_height as usize),
        );
    }

    // Check for active alarms.
    pub fn idle(&self) -> bool {
        !self.list.iter().any(|a| !a.exceeded)
    }

    pub fn scroll_up(&mut self, layout: &Layout) {
        let excess = self
            .list
            .len()
            .saturating_sub(layout.roster_height as usize);
        self.offset = excess.min(self.offset.saturating_sub(1));
    }

    pub fn scroll_down(&mut self, layout: &Layout) {
        let excess = self
            .list
            .len()
            .saturating_sub(layout.roster_height as usize);
        self.offset = excess.min(self.offset.saturating_add(1));
    }

    // Find and process exceeded alarms.
    pub fn check(
        &mut self,
        clock: &mut Clock,
        layout: &Layout,
        countdown: &mut Countdown,
        force_redraw: bool,
    ) -> Option<&Alarm> {
        let mut ret = None;

        for (index, alarm) in self
            .list
            .iter_mut()
            .enumerate()
            // Ignore alarms marked exceeded.
            .filter(|(_, a)| !a.exceeded)
        {
            if alarm.time <= clock.elapsed {
                // Found alarm to raise.
                alarm.exceeded = true;
                clock.color_index = Some(alarm.color_index);
                countdown.reset();
                ret = Some(&*alarm);
                // Skip ahead to the next one.
                continue;
            }
            // Reached the alarm to exceed next. Update countdown accordingly.
            countdown.set(alarm.time - clock.elapsed);
            if !countdown.has_position() || force_redraw {
                countdown.place(&layout, &alarm, self.offset, index);
            }
            // Ignore other alarms.
            break;
        }
        ret // Return value.
    }

    // Draw alarm roster according to layout.
    pub fn draw<W: Write>(
        &mut self,
        stdout: &mut RawTerminal<W>,
        layout: &mut Layout,
    ) -> Result<(), std::io::Error> {
        // Adjust offset in case something changed, e.g. the terminal size.
        self.adjust_offset(&layout);

        for (i, alarm) in self.list.iter().skip(self.offset).enumerate() {
            let line = layout.roster.line + i as u16;

            if self.offset > 0 && i == 0 {
                // Indicate hidden items at top.
                write!(
                    stdout,
                    "{}{}[ ^ ]{}",
                    cursor::Goto(layout.roster.col, line),
                    style::Faint,
                    style::Reset,
                )?;
                continue;
            } else if i as u16 == layout.roster_height {
                // Indicate hidden items at bottom.
                write!(
                    stdout,
                    "{}{}[ v ]{}{}",
                    cursor::Goto(layout.roster.col, line),
                    style::Faint,
                    if !self.hints_shown {
                        self.hints_shown = true;
                        " [Page Up/Down]"
                    } else {
                        ""
                    },
                    style::Reset,
                )?;
                break;
            }

            match alarm.exceeded {
                true => {
                    write!(
                        stdout,
                        "{}{}{}{} {} {}{}",
                        cursor::Goto(layout.roster.col, line),
                        color::Fg(COLOR[alarm.color_index]),
                        style::Bold,
                        style::Invert,
                        &alarm.label,
                        style::Reset,
                        color::Fg(color::Reset),
                    )?;
                }
                false => {
                    write!(
                        stdout,
                        "{}{} {} {}",
                        cursor::Goto(layout.roster.col, line),
                        color::Bg(COLOR[alarm.color_index]),
                        color::Bg(color::Reset),
                        &alarm.label,
                    )?;
                }
            }
        }
        Ok(())
    }

    // Return width of roster.
    pub fn width(&self) -> u16 {
        let mut width: u16 = 0;
        for alarm in &self.list {
            let length = UnicodeWidthStr::width(alarm.label.as_str()) as u16;
            if length > width {
                width = length
            };
        }
        // Actual width is 4 columns wider if it's not 0.
        if width == 0 {
            0
        } else {
            width.saturating_add(4)
        }
    }

    // Reset every alarm.
    pub fn reset_all(&mut self) {
        for alarm in &mut self.list {
            alarm.reset();
        }
    }

    // Call when time jumps backwards.
    pub fn time_travel(&mut self, clock: &mut Clock) {
        clock.color_index = None;

        for alarm in self.list.iter_mut() {
            if alarm.time <= clock.elapsed {
                alarm.exceeded = true;
                clock.color_index = Some(alarm.color_index);
            } else {
                alarm.exceeded = false;
            }
        }
    }

    // Read alarm times from stdin.
    pub fn from_stdin(&mut self, stdin: std::io::Stdin) -> Result<(), String> {
        for line in stdin.lock().lines() {
            match line {
                Ok(line) if !line.starts_with('#') && !line.trim().is_empty() => {
                    if let Err(e) = self.add(&line) {
                        return Err(format!("Value \"{}\": {}", line, e));
                    }
                }
                Ok(_) => (), // Discard comments and empty lines.
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(())
    }
}
