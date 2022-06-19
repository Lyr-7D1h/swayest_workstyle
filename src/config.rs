extern crate dirs;

use std::{fs::read_to_string, path::Path};

use log::{debug, error};

use self::{config_error::ConfigError, match_config::MatchConfig};

mod config_error;
mod match_config;

pub const DEFAULT_CONFIG: &'static [u8] = include_bytes!("default_config.toml");

pub struct Config {
    match_config: MatchConfig,
}

/// Fetch user config content at path given otherwise return config from default location
///
/// returns `None` if there is no config at the default location
fn get_user_config_content<P: AsRef<Path>>(
    config_path: Option<P>,
) -> Result<Option<String>, ConfigError> {
    if let Some(path) = config_path {
        let path = path.as_ref();
        match read_to_string(&path) {
            Ok(content) => return Ok(Some(content)),
            Err(e) => error!(
                "Could not open config path: {}, {}",
                path.to_string_lossy(),
                e
            ),
        }
    }

    let sworkstyle_config_dir = match dirs::config_dir() {
        Some(dir) => dir.join("sworkstyle"),
        None => return Err(ConfigError::new("Could not find config directory")),
    };

    let sworkstyle_config_path = sworkstyle_config_dir.join("config.toml");

    if !sworkstyle_config_path.exists() {
        return Ok(None);
    }

    match read_to_string(&sworkstyle_config_path) {
        Ok(content) => Ok(Some(content)),
        Err(e) => Err(ConfigError::new(&format!(
            "Could not open {sworkstyle_config_path:?}: {e}"
        ))),
    }
}

impl Config {
    pub fn new<P: AsRef<Path>>(config_path: Option<P>) -> Config {
        let match_config = match get_user_config_content(config_path) {
            Ok(c) => {
                if let Some(c) = c {
                    MatchConfig::from(c)
                } else {
                    debug!("No config found, using default");
                    MatchConfig::default()
                }
            }
            Err(e) => {
                error!("Failed to create config: {e}");
                MatchConfig::default()
            }
        };

        Config { match_config }
    }

    pub fn fetch_icon(&self, exact_name: &String, generic_name: Option<&String>) -> String {
        self.match_config.fetch_icon(exact_name, generic_name)
    }
}
