use std::io::Write;
use std::process::{Command, Stdio, Child};
use termion::{color, cursor, style};
use termion::raw::RawTerminal;
use unicode_width::UnicodeWidthStr;
use crate::Config;
use crate::clock::Clock;
use crate::layout::{Layout, Position};
use crate::utils::*;
use crate::consts::{COLOR, LABEL_SIZE_LIMIT};

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

    pub fn set_position(&mut self, position: Position) {
        self.position = Some(position);
    }

    pub fn has_position(&self) -> bool {
        self.position.is_some()
    }

    // Draw countdown.
    pub fn draw<W: Write>(&self, stdout: &mut RawTerminal<W>)
        -> Result<(), std::io::Error>
    {
        if let Some(pos) = &self.position {
            if self.value < 3600 {
                // Show minutes and seconds.
                write!(stdout,
                    "{}(-{:02}:{:02})",
                    cursor::Goto(pos.col, pos.line),
                    (self.value / 60) % 60,
                    self.value % 60)?;
                if self.value == 3599 {
                    // Write three additional spaces after switching from hour display to
                    // minute display.
                    write!(stdout, "   ")?;
                }
            } else {
                // Show hours, minutes and seconds.
                write!(stdout,
                    "{}(-{:02}:{:02}:{:02})",
                    cursor::Goto(pos.col, pos.line),
                    self.value / 3600,
                    (self.value / 60) % 60,
                    self.value % 60)?;
            }
        }
        Ok(())
    }
}

pub struct Alarm {
    time: u32,
    label: String,
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
}

impl AlarmRoster {
    pub fn new() -> AlarmRoster {
        AlarmRoster {
            list: Vec::new(),
        }
    }

    // Parse string and add as alarm.
    pub fn add(&mut self, input: &String) -> Result<(), &'static str> {
        let mut index = 0;
        let mut time: u32 = 0;
        let mut label: String;
        let time_str: &str;

        if let Some(i) = input.find(DELIMITER) {
            label = input[(i + 1)..].to_string();
            // Truncate label.
            unicode_truncate(&mut label, LABEL_SIZE_LIMIT);
            time_str = &input[..i].trim();
        } else {
            label = input.clone();
            time_str = &input.trim();
        }

        // Parse input into seconds.
        if time_str.contains(':') {
            for sub in time_str.rsplit(':') {
                match sub.parse::<u32>() {
                    // Valid.
                    Ok(d) if d < 60 && index < 3 => time += d * 60u32.pow(index),
                    // Passes as u32, but does not fit into time range.
                    Ok(_) => return Err("Could not parse value as time."),
                    // Ignore failure caused by an empty string.
                    // TODO: Match error kind when stable. See documentation
                    // for std::num::ParseIntError and
                    // https://github.com/rust-lang/rust/issues/22639
                    Err(_) if sub.is_empty() => (),
                    // Could not parse to u32.
                    Err(_) => return Err("Could not parse value as integer."),
                }
                index += 1;
            }
        } else {
            // Parse as seconds only.
            match time_str.parse::<u32>() {
                Ok(d) => time = d,
                Err(_) => return Err("Could not parse as integer."),
            }
        }

        // Skip if time is out of boundaries.
        if time == 0 { return Err("Evaluates to zero.") };
        if time >= 24 * 60 * 60 { return Err("Values >24h not supported.") };
        // Filter out double entries.
        if self.list.iter().any(|a| a.time == time) {
            return Err("Already exists.");
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
    pub fn drop_last(&mut self) -> bool {
        self.list.pop().is_some()
    }

    // Check for active alarms.
    pub fn idle(&self) -> bool {
        !self.list.iter().any(|a| !a.exceeded)
    }

    // Check for exceeded alarms.
    pub fn check(&mut self,
        clock: &mut Clock,
        layout: &Layout,
        countdown: &mut Countdown,
        force_redraw: bool,
    ) -> Option<(u32, &String)>
    {
        let mut ret = None;
        let size = self.list.len() as u16;

        for (index, alarm) in self.list.iter_mut().enumerate()
            // Ignore alarms marked exceeded.
            .filter(|(_, a)| !a.exceeded) {

            if alarm.time <= clock.elapsed {
                // Found alarm to raise.
                ret = Some((alarm.time, &alarm.label));
                alarm.exceeded = true;
                clock.color_index = Some(alarm.color_index);
                countdown.reset();
                // Skip ahead to the next one.
                continue;
            }
            // Reached the alarm to exceed next. Update countdown accordingly.
            countdown.set(alarm.time - clock.elapsed);
            if !countdown.has_position() || force_redraw {
                // Compute position.
                let mut col =
                    layout.roster.col
                    + 3
                    + UnicodeWidthStr::width(&alarm.label[..]) as u16;
                let mut line = layout.roster.line + index as u16;

                // Compensate for "hidden" items in the alarm roster.
                // TODO: Make this more elegant and robust.
                if let Some(offset) = size.checked_sub(layout.roster_height + 1) {
                    if index as u16 <= offset{
                        // Draw next to placeholder ("[...]").
                        line = layout.roster.line;
                        col = layout.roster.col + 6;
                    } else {
                        line = line.checked_sub(offset)
                            .unwrap_or(layout.roster.line);
                    }
                }
                countdown.set_position(Position { col, line });
            }
            // Ignore other alarms.
            break;
        }
        ret // Return value.
    }

    // Draw alarm roster according to layout.
    pub fn draw<W: Write>(
        &self,
        stdout: &mut RawTerminal<W>,
        layout: &mut Layout,
        config: &Config,
    ) -> Result<(), std::io::Error>
    {
        // Find first item to print in case we lack the space to print them
        // all. Final '-1' to take account for the input buffer.
        let mut offset = 0;

        if self.list.len() > layout.roster_height as usize {
            // Actually -1 (zero indexing) +1 (first line containing "[...]").
            offset = self.list.len() - layout.roster_height as usize;

            write!(stdout,
                "{}{}[...]{}",
                cursor::Goto(layout.roster.col, layout.roster.line),
                style::Faint,
                style::Reset,
            )?;
        }

        for (i, alarm) in self.list.iter().skip(offset).enumerate() {
            let line = if offset > 0 {
                // Add offset of one for "[...]".
                layout.roster.line + i as u16 + 1
            } else {
                layout.roster.line + i as u16
            };

            match alarm.exceeded {
                true if config.fancy => {
                    write!(stdout,
                        "{}{}{}{} {} {}🭬{}{}",
                        cursor::Goto(layout.roster.col, line),
                        color::Fg(COLOR[alarm.color_index]),
                        style::Bold,
                        style::Invert,
                        &alarm.label,
                        style::NoInvert,
                        color::Fg(color::Reset),
                        style::Reset,
                    )?;
                },
                false if config.fancy => {
                    write!(stdout,
                        "{}{}█🭬{}{}",
                        cursor::Goto(layout.roster.col, line),
                        color::Fg(COLOR[alarm.color_index]),
                        color::Fg(color::Reset),
                        &alarm.label,
                    )?;
                },
                true => {
                    write!(stdout,
                        "{}{}{}{} {} {}{}",
                        cursor::Goto(layout.roster.col, line),
                        color::Fg(COLOR[alarm.color_index]),
                        style::Bold,
                        style::Invert,
                        &alarm.label,
                        color::Fg(color::Reset),
                        style::Reset,
                    )?;
                },
                false => {
                    write!(stdout,
                        "{}{} {} {}",
                        cursor::Goto(layout.roster.col, line),
                        color::Bg(COLOR[alarm.color_index]),
                        color::Bg(color::Reset),
                        &alarm.label,
                    )?;
                },
            }
        }
        Ok(())
    }

    // Return width of roster.
    pub fn width(&self) -> u16 {
        let mut width: u16 = 0;
        for alarm in &self.list {
            let length = UnicodeWidthStr::width(&alarm.label[..]) as u16;
            if length > width { width = length };
        }
        // Actual width is 4 columns wider if it's not 0.
        if width == 0 { 0 } else { width.saturating_add(4) }
    }

    // Reset every alarm.
    pub fn reset_all(&mut self) {
        for alarm in &mut self.list {
            alarm.reset();
        }
    }

    // Call when time jumps backwards.
    pub fn time_travel(&mut self, time: u32) {
        for alarm in self.list.iter_mut() {
            alarm.exceeded = alarm.time <= time;
        }
    }

    // Read alarm times from stdin.
    pub fn from_stdin(&mut self, stdin: std::io::Stdin)
        -> Result<(), String>
    {
        let mut buffer = String::new();
        loop {
            match stdin.read_line(&mut buffer) {
                Ok(0) => break, // EOF.
                Ok(1) => continue, // Empty (newline only).
                Ok(_) => (),
                Err(e) => return Err(e.to_string()),
            }
            // Strip newline.
            buffer.retain(|c| c != '\n');
            // Ignore lines containing only white spaces.
            if buffer.contains(|c: char| !c.is_whitespace()) {
                if let Err(e) = self.add(&buffer) {
                    return Err(e.to_string());
                }
            }
            buffer.clear();
        }
        Ok(())
    }
}

// Execute the command given on the command line.
pub fn exec_command(command: &Vec<String>, elapsed: u32, label: &String) -> Option<Child> {
    let time = if elapsed < 3600 {
        format!("{:02}:{:02}", elapsed / 60, elapsed % 60)
    } else {
        format!("{:02}:{:02}:{:02}", elapsed /3600, (elapsed / 60) % 60, elapsed % 60)
    };

    let mut args: Vec<String> = Vec::new();
    // Build vector of command line arguments. Replace every occurrence of
    // "{t}" and "{l}".
    for s in command.iter().skip(1) {
        args.push(s.replace("{t}", &time).replace("{l}", &label));
    }

    match Command::new(&command[0])
        .args(args)
        .stdout(Stdio::null())
        .stdin(Stdio::null())
        .spawn() {
        Ok(child) => Some(child),
        Err(error) => {
            eprintln!("Error: Could not execute command. ({})", error);
            None
        }
    }
}

