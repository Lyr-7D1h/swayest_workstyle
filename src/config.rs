extern crate dirs;

use std::{fs::read_to_string, path::Path};

use log::{debug, warn};

use self::match_config::MatchConfig;

mod config_error;
mod match_config;

pub const DEFAULT_CONFIG: &'static [u8] = include_bytes!("default_config.toml");

pub struct Config {
    match_config: MatchConfig,
}

fn match_config<P: AsRef<Path>>(config_path: Option<P>) -> MatchConfig {
    if let Some(config_path) = config_path {
        match read_to_string(&config_path) {
            Ok(content) => return MatchConfig::from(content),
            Err(e) => debug!(
                "Could not create config from path: {:?} {e}",
                config_path.as_ref()
            ),
        }
    } else {
        warn!("Default config could not have been found")
    }

    return MatchConfig::default();
}

impl Config {
    pub fn new<P: AsRef<Path>>(config_path: Option<P>) -> Config {
        let match_config = match_config(config_path);

        Config { match_config }
    }

    pub fn fetch_icon(&self, exact_name: &String, generic_name: Option<&String>) -> String {
        self.match_config.fetch_icon(exact_name, generic_name)
    }
}
