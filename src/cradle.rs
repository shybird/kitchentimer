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

use std::process::{self, Command, Stdio};

// Manages spawned child processes.
pub struct Cradle {
    commands: Vec<Vec<String>>,
    children: Vec<process::Child>,
}

impl Drop for Cradle {
    fn drop(&mut self) {
        for child in self.children.iter_mut() {
            eprint!(
                "Waiting for spawned process (PID {}) to finish ...",
                child.id()
            );

            match child.wait() {
                Ok(status) if status.success() => eprintln!(" ok"),
                Ok(status) if status.code().is_none() => eprintln!(" interrupted ({})", status),
                Ok(status) => eprintln!(" ok ({})", status),
                Err(error) => eprintln!(" failed ({})", error),
            }
        }
    }
}

impl Cradle {
    pub fn new() -> Cradle {
        Cradle {
            commands: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn add(&mut self, mut command: Vec<String>) {
        // Vector will never change from here on.
        command.shrink_to_fit();
        self.commands.push(command);
        self.children.reserve(self.commands.len());
    }

    pub fn run_all(&mut self, time: u32, label: &String) {
        // Do nothing if there are still running child processes.
        if !self.children.is_empty() { return; }

        let time = if time < 3600 {
            format!("{:02}:{:02}", time / 60, time % 60)
        } else {
            format!(
                "{:02}:{:02}:{:02}",
                time / 3600,
                (time / 60) % 60,
                time % 60
            )
        };

        for command in self.commands.iter() {
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
                .spawn()
            {
                Ok(child) => self.children.push(child),
                Err(error) => eprintln!("Error: Could not execute command. ({})", error),
            }
        }
    }

    pub fn tend(&mut self) {
        while let Some(mut child) = self.children.pop() {
            match child.try_wait() {
                // Process exited successfully.
                Ok(Some(status)) if status.success() => (),
                // Abnormal exit.
                Ok(Some(status)) => eprintln!("Spawned process terminated with non-zero exit status. ({})", status),
                // Process is still running. Put back child and return.
                // Leaving any other children unattended, which shouldn't be
                // a problem, as we will not spawn any further commands, as
                // long as self.children isn't empty.
                Ok(None) => {
                    self.children.push(child);
                    break;
                }
                // Other error.
                Err(error) => eprintln!("Error executing command. ({})", error),
            }
        }
    }

    // Parse command line argument to --command into a vector of strings suitable
    // for process::Command::new().
    pub fn parse(input: &str) -> Vec<String> {
        let mut command: Vec<String> = Vec::new();
        let mut segment: String = String::new();
        let mut quoted = false;
        let mut escaped = false;

        for c in input.chars() {
            match c {
                '\\' if !escaped => {
                    // Next char is escaped. (If not escaped itself.)
                    escaped = true;
                    continue;
                }
                // Keep spaces when escaped or quoted.
                ' ' if escaped || quoted => {
                    &segment.push(' ');
                }
                // Otherwise end the current segment.
                ' ' => {
                    if !&segment.is_empty() {
                        command.push(segment.clone());
                        &segment.clear();
                    }
                }
                // Quotation marks toggle quote.
                '"' | '\'' if !escaped => quoted = !quoted,
                // Carry everything else. Escape if found escaped.
                _ => {
                    if escaped {
                        &segment.push('\\');
                    }
                    &segment.push(c);
                }
            }
            escaped = false;
        }
        command.push(segment);
        // Vector will not change from here on.
        for segment in command.iter_mut() {
            segment.shrink_to_fit();
        }
        command.shrink_to_fit();
        command
    }
}

