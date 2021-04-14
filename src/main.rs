use std::{env, process};
use kitchentimer::{Config, AlarmRoster, run};


fn main() {
    let args = env::args();
    let mut alarm_roster = AlarmRoster::new();
    // Parse command line arguments into config and alarm roster.
    let config = Config::new(args, &mut alarm_roster)
        .unwrap_or_else(|e| {
            eprintln!("{}", e);
            process::exit(1);
        });

    // Read alarm times from stdin if stdin not a tty.
    let stdin = std::io::stdin();
    if !termion::is_tty(&stdin) {
        stdin.lock();
        if let Err(e) = alarm_roster.from_stdin(stdin) {
            eprintln!("Error while reading alarm times from stdin. ({})", e);
            process::exit(1);
        }
    }

    // Holds spawned child process if any.
    let mut spawned: Option<process::Child> = None;

    // Run main loop.
    if let Err(e) = run(config, alarm_roster, &mut spawned) {
        eprintln!("Main loop exited with error: {}", e);
        process::exit(1);
    }

    // Wait for remaining spawned processes to exit.
    if let Some(ref mut child) = spawned {
        eprint!("Waiting for spawned process (PID {}) to exit ...", child.id());

        match child.wait() {
            Ok(status) => eprintln!(" ok ({})", status),
            Err(error) => eprintln!(" failed ({})", error),
        }
    }
}


