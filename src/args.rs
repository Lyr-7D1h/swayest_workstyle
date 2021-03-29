use std::{env, process};

use log::LevelFilter;
use simple_logger::SimpleLogger;

/// Setup the program with the needed options
pub fn setup() {
    let mut log_level: LevelFilter = LevelFilter::Warn;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match &arg[..] {
            "-h" | "--help" => {
                println!("Swayest Workstyle.\nThis tool will rename workspaces to the icons configured.\nConfig can be found in $HOME/.config/sworkstyle");
                // #TODO add verbosity
                process::exit(0);
            }
            "--log-level" => {
                if let Some(level) = args.next() {
                    log_level = match &level[..] {
                        "error" => LevelFilter::Error,
                        "warn" => LevelFilter::Warn,
                        "info" => LevelFilter::Info,
                        "debug" => LevelFilter::Debug,
                        _ => {
                            eprintln!("Invalid logging option: {}", level);
                            process::exit(1);
                        }
                    }
                } else {
                    eprintln!("No logging option given");
                    process::exit(1);
                }
            }
            _ => {
                eprintln!("Did not recognize \"{}\" as an option", arg);
                process::exit(1);
            }
        }
    }

    SimpleLogger::new().with_level(log_level).init().unwrap();
}
