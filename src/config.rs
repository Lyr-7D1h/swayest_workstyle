use std::{
    convert::TryFrom,
    fs::read_to_string,
    path::{Path, PathBuf},
    str::from_utf8,
};

use log::{debug, error, info, warn};
use regex::Regex;

use crate::util::prettify_option;

mod config_error;

mod parse_content_to_config;
use parse_content_to_config::parse_content_to_config;

pub const DEFAULT_MATCH_CONFIG: &'static [u8] = include_bytes!("../default_config.toml");

#[derive(Clone, Debug)]
pub enum Pattern {
    Regex(Regex),
    String(String),
}

impl PartialEq for Pattern {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(r0), Self::Regex(l0)) => l0.to_string() == r0.to_string(),
            (Self::Regex(l0), Self::String(r0)) => l0.to_string() == r0.to_string(),
            (Self::Regex(l0), Self::Regex(r0)) => l0.to_string() == r0.to_string(),
            (Self::String(l0), Self::String(r0)) => l0 == r0,
        }
    }
}

impl TryFrom<String> for Pattern {
    type Error = regex::Error;

    fn try_from(mut value: String) -> Result<Self, Self::Error> {
        if value.starts_with("/") && value.ends_with("/") {
            value.remove(value.len() - 1);
            value.remove(0);
            let regex = Regex::new(&value)?;
            Ok(Pattern::Regex(regex))
        } else {
            Ok(Pattern::String(value))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Match {
    Generic { pattern: Pattern, value: String },
    Exact { pattern: String, value: String },
}

#[derive(Clone, Debug)]
pub struct Config {
    pub matchings: Vec<Match>,
    pub fallback: Option<String>,
}

impl Config {
    pub fn new<P: AsRef<Path>>(config_path: &Option<P>) -> Config {
        if let Some(config_path) = config_path {
            match read_to_string(&config_path) {
                Ok(content) => return Config::from(content),
                Err(e) => {
                    debug!(
                        "Could not create config from path: {:?} {e}",
                        config_path.as_ref()
                    )
                }
            }
        } else {
            warn!("Default config could not have been found")
        }

        return Config::default();
    }

    pub fn fetch_icon(&self, exact_name: &String, generic_name: Option<&String>) -> String {
        for m in &self.matchings {
            match m {
                Match::Generic { pattern, value } => {
                    if let Some(generic_name) = &generic_name {
                        match pattern {
                            Pattern::Regex(r) => {
                                if r.is_match(generic_name) {
                                    return value.clone();
                                }
                            }
                            Pattern::String(p) => {
                                if generic_name.to_lowercase().contains(&p.to_lowercase()) {
                                    return value.clone();
                                }
                            }
                        }
                    }
                }
                Match::Exact { pattern, value } => {
                    if exact_name == pattern {
                        return value.clone();
                    }
                }
            }
        }

        warn!(
            "No match for \"{}\" with title \"{}\"",
            exact_name,
            prettify_option(generic_name),
        );

        self.fallback()
    }

    pub fn fallback(&self) -> String {
        match &self.fallback {
            Some(fallback) => {
                info!("Using fallback: {}", fallback);
                fallback.clone()
            }
            None => {
                warn!("No fallback set using empty string");
                String::from("")
            }
        }
    }
}

impl<S: Into<String>> From<S> for Config {
    /// Parse a string to a config enriching it with the default config
    fn from(value: S) -> Self {
        let value = value.into();
        let mut default = Config::default();

        match parse_content_to_config(&value) {
            Ok(mut user_config) => {
                user_config.matchings.append(&mut default.matchings);

                if user_config.fallback.is_none() {
                    warn!(
                        "No fallback set using default: {}",
                        prettify_option(default.fallback.as_ref())
                    );
                    user_config.fallback = default.fallback
                }

                user_config
            }
            Err(e) => {
                error!("Invalid config format: {}", e);
                return default;
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let default_config_content = from_utf8(DEFAULT_MATCH_CONFIG).unwrap().to_string();
        return parse_content_to_config(&default_config_content).unwrap();
    }
}

#[test]
fn test_default() {
    let config = Config::default();
    assert_eq!(config.fallback.unwrap(), "")
}

#[test]
fn test_from_string() {
    let config = Config::from(
        "
    fallback = 'c'
    [matching]
    a = 'b'
    b = 'c'
    '/(?i)A title/' = 'd' 
    ",
    );

    assert_eq!(config.fallback(), "c");
    assert_eq!(
        config.fetch_icon(&String::from("application"), Some(&String::from("a title"))),
        "d"
    );
}
