extern crate signal_hook;
mod alarm;
mod clock;
mod consts;
mod kitchentimer;
mod layout;
mod utils;
#[cfg(test)]
mod tests;

use std::env;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use signal_hook::flag;
use clock::Clock;
use alarm::AlarmRoster;
use layout::{Layout, Position};
use consts::*;
use kitchentimer::kitchentimer;


pub struct Config {
    plain: bool,
    quit: bool,
    command: Option<Vec<String>>,
}


fn main() {
    let mut config = Config {
        plain: false,
        quit: false,
        command: None,
    };
    let mut alarm_roster = AlarmRoster::new();
    // Parse command line arguments into config and alarm roster.
    parse_args(&mut config, &mut alarm_roster);

    // Register signal handlers.
    let signal = Arc::new(AtomicUsize::new(0));
    let sigwinch = Arc::new(AtomicBool::new(true));
    register_signal_handlers(&signal, &sigwinch);
    // Spawned child process if any.
    let mut spawned: Option<std::process::Child> = None;

    // Runs main loop.
    kitchentimer(
        config,
        alarm_roster,
        signal,
        sigwinch,
        &mut spawned,
    );

    // Wait for remaining spawned processes to exit.
    if let Some(ref mut child) = spawned {
        print!("Waiting for spawned processes (PID {}) to exit ...", child.id());
        std::io::stdout().flush().unwrap();

        match child.wait() {
            Ok(status) => println!(" ok ({})", status),
            Err(error) => println!(" failed ({})", error),
        }
    }
}

// Print usage information and exit.
fn usage() {
    println!("{}", USAGE);
    std::process::exit(0);
}

// Parse command line arguments into "config".
fn parse_args(config: &mut Config, alarm_roster: &mut AlarmRoster) {
    let mut iter = env::args().skip(1);

    loop {
        if let Some(arg) = iter.next() {
            match arg.as_str() {
                "-h" | "--help" => usage(),
                "-v" | "--version" => {
                    println!("{} {}", NAME, VERSION);
                    std::process::exit(0);
                },
                "-p" | "--plain" => config.plain = true,
                "-q" | "--quit" => config.quit = true,
                "-e" | "--exec" => {
                    if let Some(e) = iter.next() {
                        config.command = Some(parse_to_command(&e));
                    } else {
                        println!("Missing parameter to \"{}\".", arg);
                        std::process::exit(1);
                    }
                },
                any if any.starts_with('-') => {
                    // Unrecognized flag.
                    println!("Unrecognized option: \"{}\"", any);
                    println!("Use \"-h\" or \"--help\" for a list of valid command line options");
                    std::process::exit(1);
                },
                any => {
                    // Alarm to add.
                    if let Err(error) = alarm_roster.add(&String::from(any)) {
                        println!("Error adding \"{}\" as alarm. ({})", any, error);
                        std::process::exit(1);
                    }
                },
            }
        } else { break; } // All command line parameters processed.
    }
}

// Parse command line argument to --command into a vector of strings suitable
// for process::Command::new().
fn parse_to_command(input: &str) -> Vec<String> {
    let mut command: Vec<String> = Vec::new();
    let mut buffer: String = String::new();
    let mut quoted = false;
    let mut escaped = false;

    for byte in input.chars() {
        match byte {
            '\\' if !escaped => {
                // Next char is escaped.
                escaped = true;
                continue;
            },
            ' ' if escaped || quoted => { &buffer.push(' '); },
            ' ' => {
                if !&buffer.is_empty() {
                    command.push(buffer.clone());
                    &buffer.clear();
                }
            },
            '"' | '\'' if !escaped => quoted = !quoted,
            _ => {
                if escaped { &buffer.push('\\'); }
                &buffer.push(byte);
            },
        }
        escaped = false;
    }
    command.push(buffer);
    command.shrink_to_fit();
    command
}

fn register_signal_handlers(
    signal: &Arc<AtomicUsize>,
    recalc_flag: &Arc<AtomicBool>,
) {
    flag::register_usize(SIGTSTP as i32, Arc::clone(&signal), SIGTSTP).unwrap();
    flag::register_usize(SIGCONT as i32, Arc::clone(&signal), SIGCONT).unwrap();
    flag::register_usize(SIGTERM as i32, Arc::clone(&signal), SIGTERM).unwrap();
    flag::register_usize(SIGINT as i32, Arc::clone(&signal), SIGINT).unwrap();
    flag::register_usize(SIGUSR1 as i32, Arc::clone(&signal), SIGUSR1).unwrap();
    flag::register_usize(SIGUSR2 as i32, Arc::clone(&signal), SIGUSR2).unwrap();
    // SIGWINCH sets "force_recalc" directly.
    flag::register(SIGWINCH as i32, Arc::clone(&recalc_flag)).unwrap();
}

