use std::io::Write;
use std::process::{Command, Stdio};
use termion::{color, cursor, style};
use termion::raw::RawTerminal;
use crate::{Clock, Config, Layout, Position};
use crate::common::COLOR;


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
    }

    // Draw countdown.
    pub fn draw<W: Write>(&self, stdout: &mut RawTerminal<W>) {
        if let Some(pos) = &self.position {
            if self.value < 3600 {
                // Show minutes and seconds.
                write!(stdout,
                    "{}(-{:02}:{:02})",
                    cursor::Goto(pos.col, pos.line),
                    (self.value / 60) % 60,
                    self.value % 60)
                    .unwrap();
                if self.value == 3599 {
                    // Write three additional spaces after switching from hour display to
                    // minute display.
                    write!(stdout, "   ").unwrap();
                }
            } else {
                // Show hours, minutes and seconds.
                write!(stdout,
                    "{}(-{:02}:{:02}:{:02})",
                    cursor::Goto(pos.col, pos.line),
                    self.value / 3600,
                    (self.value / 60) % 60,
                    self.value % 60)
                    .unwrap();
            }
        }
    }
}

pub struct Alarm {
    time: u32,
    display: String,
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

    pub fn add(&mut self, buffer: &String)
        -> Result<(), &'static str> {

        let mut index = 0;
        let mut time: u32 = 0;

        // Parse input into seconds.
        for sub in buffer.rsplit(':') {
            if sub.len() > 0 {
                let d = sub.parse::<u32>();
                match d {
                    Ok(d) => time += d * 60u32.pow(index),
                    Err(_) => return Err("Could not parse number as <u32>."),
                }
            }
            index += 1;

            // More than 3 fields are an error.
            if index > 3 { return Err("Too many colons to parse.") };
        }

        // Skip if time evaluated to zero.
        if time == 0 { return Err("Evaluates to zero.") };
        if time >= 24 * 60 * 60 { return Err("Values >24h not supported.") };

        let alarm = Alarm {
            display: buffer.clone(),
            time,
            color_index: (self.list.len() % COLOR.len()),
            exceeded: false,
        };

        // Add to list, insert based on alarm time. Filter out double entries.
        let mut i = self.list.len();
        if i == 0 {
            self.list.push(alarm);
        } else {
            while i > 0 {
                // Filter out double entries.
                if self.list[i - 1].time == time {
                    return Err("Already exists.");
                } else if self.list[i - 1].time < time {
                    break;
                }
                i -= 1;
            }
            self.list.insert(i, alarm);
        }
        Ok(())
    }

    pub fn pop(&mut self) -> Option<Alarm> {
        self.list.pop()
    }

    // Check for exceeded alarms.
    pub fn check(&mut self,
        clock: &mut Clock,
        layout: &Layout,
        countdown: &mut Countdown) -> bool {

        let mut hit = false;
        let mut index = 0;

        for alarm in &mut self.list {
            // Ignore alarms already marked exceeded.
            if !alarm.exceeded {
                if alarm.time <= clock.elapsed {
                    // Found alarm to raise.
                    hit = true;
                    alarm.exceeded = true;
                    clock.color_index = Some(alarm.color_index);
                    countdown.value = 0;
                    countdown.position = None;
                    // Skip ahead to the next one.
                    index += 1;
                    continue;
                }
                // Reached the alarm to exceed next. Update countdown
                // accordingly.
                countdown.value = alarm.time - clock.elapsed;
                if countdown.position.is_none() || layout.force_redraw {
                    // Compute position.
                    let mut col =
                        layout.roster.col
                        + 3
                        + alarm.display.len() as u16;
                    let mut line = layout.roster.line + index;

                    // Compensate for "hidden" items in the alarm roster.
                    // TODO: Make this more elegant and robust.
                    if let Some(offset) = (self.list.len() as u16)
                        .checked_sub(layout.roster_height + 1) {

                        if index <= offset{
                            // Draw next to placeholder ("[...]").
                            line = layout.roster.line;
                            col = layout.roster.col + 6;
                        } else {
                            line = line.checked_sub(offset).unwrap_or(layout.roster.line);
                        }
                    }
                    countdown.position = Some(Position { col, line, });
                }
                // Ignore other alarms.
                break;
            }
            index += 1;
        }
        hit // Return value.
    }

    // Draw alarm roster according to layout.
    pub fn draw<W: Write>(&self, stdout: &mut RawTerminal<W>, layout: &mut Layout) {
        let mut width: u16 = 0;
        let mut index = 0;

        // Find first item to print in case we lack space to print them all.
        // Final '-1' to take account for the input buffer.
        let mut first = 0;

        if self.list.len() > layout.roster_height as usize {
            // Actually -1 (zero indexing) +1 (first line containing "...").
            first = self.list.len() - layout.roster_height as usize;
            index += 1;

            write!(stdout,
                "{}{}[...]{}",
                cursor::Goto(layout.roster.col, layout.roster.line),
                style::Faint,
                style::Reset).unwrap();
        }

        for alarm in &self.list[first..] {
            if alarm.exceeded {
                write!(stdout,
                    "{}{} {}{} {}!{}",
                    cursor::Goto(layout.roster.col, layout.roster.line + index),
                    color::Bg(COLOR[alarm.color_index]),
                    color::Bg(color::Reset),
                    style::Bold,
                    alarm.display,
                    style::Reset)
                    .unwrap();
            } else {
                write!(stdout,
                    "{}{} {} {}",
                    cursor::Goto(layout.roster.col, layout.roster.line + index),
                    color::Bg(COLOR[alarm.color_index]),
                    color::Bg(color::Reset),
                    alarm.display)
                    .unwrap();
            }
            index += 1;
            // Calculate roster width. Actual display width is 3 chars wider.
            if 3 + alarm.display.len() as u16 > width {
                width = 3 + alarm.display.len() as u16;
            }
        }
        // Update layout information.
        if layout.roster_width != width {
            layout.roster_width = width;
            layout.force_recalc = true;
        }
    }

    // Reset every alarm.
    pub fn reset_all(&mut self) {
        for a in &mut self.list {
            a.reset();
        }
    }
}

// Execute the command given on the command line.
pub fn alarm_exec(config: &Config, elapsed: u32) {
    let mut args: Vec<String> = Vec::new();
    let time: String;

    if elapsed < 3600 {
        time = format!("{:02}:{:02}", elapsed / 60, elapsed % 60);
    } else {
        time = format!("{:02}:{:02}:{:02}", elapsed /3600, (elapsed / 60) % 60, elapsed % 60);
    }

    if let Some(exec) = &config.alarm_exec {
        // Replace every occurrence of "%s".
        for s in exec {
            args.push(s.replace("%s", &time));
        }

        if Command::new(&exec[0])
            .args(&args[1..])
            .stdout(Stdio::null())
            .stdin(Stdio::null())
            .spawn().is_err() {

            eprintln!("Error: Could not execute command");
        }
    }
}

