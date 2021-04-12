use std::{env, process};
use std::io::Write;
use kitchentimer::{Config, AlarmRoster, run};


fn main() {
    let args = env::args();
    let mut alarm_roster = AlarmRoster::new();
    // Parse command line arguments into config and alarm roster.
    let config = Config::new(args, &mut alarm_roster)
        .unwrap_or_else(|e| {
            println!("{}", e);
            process::exit(1);
        });

    // Holds spawned child process if any.
    let mut spawned: Option<process::Child> = None;

    // Run main loop.
    if let Err(e) = run(config, alarm_roster, &mut spawned) {
        println!("Main loop exited with error: {}", e);
        process::exit(1);
    }

    // Wait for remaining spawned processes to exit.
    if let Some(ref mut child) = spawned {
        print!("Waiting for spawned process (PID {}) to exit ...", child.id());
        std::io::stdout().flush().unwrap();

        match child.wait() {
            Ok(status) => println!(" ok ({})", status),
            Err(error) => println!(" failed ({})", error),
        }
    }
}


