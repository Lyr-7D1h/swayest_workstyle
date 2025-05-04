use std::process;
use sworkstyle::Sworkstyle;

use fslock::LockFile;
use log::{debug, error};
use simple_logger::SimpleLogger;

use std::{env, path::PathBuf};

use log::LevelFilter;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Args {
    pub log_level: LevelFilter,
    pub config_path: Option<PathBuf>,
    pub deduplicate: bool,
}

/// Get the xdg default config path
fn default_config_path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("sworkstyle/config.toml"))
}

impl Args {
    pub fn from_cli() -> Args {
        let mut log_level = LevelFilter::Warn;
        let mut config_path = default_config_path();
        let mut deduplicate = false;

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match &arg[..] {
                "-h" | "--help" => {
                    println!(
                        "Swayest Workstyle v{VERSION}
This tool will rename workspaces to the icons configured.
Config can be found in $HOME/.config/sworkstyle

SYNOPSIS
    sworkstyle [FLAGS]

FLAGS
    -h, --help
        Display a description of this program.
    
    -v, --version
        Print the current version

    -l, --log-level <level>
        Either \"error\", \"warn\", \"info\", \"debug\", \"off\". Uses \"warn\" by default
        
    -c, --config <file>
        Specifies the config file to use. Uses \"`XDG_CONFIG_HOME`/sworkstyle/config\" by default

    -d, --deduplicate
        Deduplicate the same icons in your workspace
        "
                    );
                    process::exit(0);
                }
                "-v" | "--version" => {
                    println!("{VERSION}");
                    process::exit(0)
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
                        let path = PathBuf::from(path);
                        if !path.exists() {
                            eprintln!("Config file does not exist or couldn't be accessed");
                            process::exit(1);
                        }
                        config_path = Some(path);
                    } else {
                        eprintln!("No path given");
                        process::exit(1);
                    }
                }
                "-d" | "--deduplicate" => {
                    deduplicate = true;
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
            deduplicate,
        };
    }
}
fn acquire_lock() {
    let mut file = match LockFile::open("/tmp/sworkstyle.lock") {
        Ok(f) => f,
        _ => return,
    };

    let locked = file.try_lock().unwrap();

    if locked == false {
        error!("Sworkstyle already running");
        process::exit(1)
    }

    ctrlc::set_handler(move || {
        debug!("Unlocking /tmp/sworkstyle.lock");
        file.unlock().unwrap();
        process::exit(0)
    })
    .expect("Could not set ctrlc handler")
}

#[async_std::main]
async fn main() {
    let args = Args::from_cli();

    SimpleLogger::new()
        .with_level(args.log_level)
        .init()
        .expect("Could not load simple logger");

    acquire_lock();

    if let Err(e) = Sworkstyle::new(args.config_path, args.deduplicate)
        .run()
        .await
    {
        error!("{e}");
        process::exit(1)
    }
}
