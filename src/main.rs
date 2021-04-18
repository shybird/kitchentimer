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

use kitchentimer::{run, AlarmRoster, Config};
use std::{env, process};

fn main() {
    let args = env::args();
    let mut alarm_roster = AlarmRoster::new();
    // Parse command line arguments into config and alarm roster.
    let config = Config::new(args, &mut alarm_roster).unwrap_or_else(|e| {
        eprintln!("{}", e);
        process::exit(1);
    });

    // Read alarm times from stdin if stdin is not a tty.
    let stdin = std::io::stdin();
    if !termion::is_tty(&stdin) {
        if let Err(e) = alarm_roster.from_stdin(stdin) {
            eprintln!("Error while reading alarm times from stdin. ({})", e);
            process::exit(1);
        }
    }

    // Run main loop. Returns spawned child process if any.
    let child = match run(config, alarm_roster) {
        Ok(child) => child,
        Err(error) => {
            eprintln!("Main loop exited with error: {}", error);
            process::exit(1);
        }
    };

    // Wait for remaining spawned process to exit.
    if let Some(mut child) = child {
        eprint!(
            "Waiting for spawned process (PID {}) to finish ...",
            child.id()
        );

        match child.wait() {
            Ok(status) if status.success() => eprintln!(" ok"),
            // Unix only.
            Ok(status) if status.code().is_none() => eprintln!(" interrupted ({})", status),
            Ok(status) => eprintln!(" ok ({})", status),
            Err(error) => eprintln!(" failed ({})", error),
        }
    }
}
