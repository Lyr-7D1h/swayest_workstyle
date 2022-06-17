use std::{env, process};

use log::LevelFilter;

pub struct Args {
    pub log_level: LevelFilter,
    pub config_path: Option<String>,
}

impl Args {
    pub fn from_cli() -> Args {
        let mut log_level = LevelFilter::Warn;
        let mut config_path = None;

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match &arg[..] {
                "-h" | "--help" => {
                    println!(
                        "Swayest Workstyle
This tool will rename workspaces to the icons configured.
Config can be found in $HOME/.config/sworkstyle

SYNOPSIS
    sworkstyle [FLAGS]

FLAGS
    -h, --help
        Display a description of this program.

    -l, --log-level
        Either \"error\", \"warn\", \"info\", \"debug\", \"off\". Uses \"warn\" by default
        
    -c, --config
        Specifies the config file to use.
        "
                    );
                    process::exit(0);
                }
                "-l" | "--log-level" => {
                    if let Some(level) = args.next() {
                        log_level = match &level[..] {
                            "error" => LevelFilter::Error,
                            "warn" => LevelFilter::Warn,
                            "info" => LevelFilter::Info,
                            "debug" => LevelFilter::Debug,
                            "off" => LevelFilter::Off,
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
                "-c" | "--config" => {
                    if let Some(path) = args.next() {
                        config_path = Some(String::from(path));
                    } else {
                        eprintln!("No path given");
                        process::exit(1);
                    }
                }
                _ => {
                    eprintln!("Did not recognize \"{}\" as an option", arg);
                    process::exit(1);
                }
            }
        }

        return Args {
            log_level,
            config_path,
        };
    }
}
