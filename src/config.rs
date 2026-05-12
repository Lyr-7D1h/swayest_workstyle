use std::{convert::TryFrom, fs::read_to_string, path::Path, str::from_utf8};

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
    Window {
        app_id: Option<Pattern>,
        class: Option<Pattern>,
        title: Option<Pattern>,
        value: String,
    },
}

#[derive(Clone, Debug)]
pub struct Config {
    pub matchings: Vec<Match>,
    pub fallback: Option<String>,
    pub separator: Option<String>,
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

    fn matches_generic_pattern(pattern: &Pattern, value: &String) -> bool {
        match pattern {
            Pattern::Regex(r) => r.is_match(value),
            Pattern::String(p) => value.to_lowercase().contains(&p.to_lowercase()),
        }
    }

    fn matches_exact_pattern(pattern: &Pattern, value: &String) -> bool {
        match pattern {
            Pattern::Regex(r) => r.is_match(value),
            Pattern::String(p) => value == p,
        }
    }

    pub fn fetch_icon(
        &self,
        app_id: Option<&String>,
        class: Option<&String>,
        title: Option<&String>,
    ) -> String {
        for m in &self.matchings {
            match m {
                Match::Generic { pattern, value } => {
                    if let Some(title) = &title {
                        if Self::matches_generic_pattern(pattern, title) {
                            return value.clone();
                        }
                    }
                }
                Match::Exact { pattern, value } => {
                    if let Some(class_name) = class {
                        if class_name == pattern {
                            return value.clone();
                        }
                    }
                    if let Some(app_id_name) = app_id {
                        if app_id_name == pattern {
                            return value.clone();
                        }
                    }
                }
                Match::Window {
                    app_id: app_id_pattern,
                    class: class_pattern,
                    title: title_pattern,
                    value,
                } => {
                    let app_id_matches = app_id_pattern.as_ref().map_or(true, |pattern| {
                        app_id
                            .as_ref()
                            .is_some_and(|app_id| Self::matches_exact_pattern(pattern, app_id))
                    });
                    let class_matches = class_pattern.as_ref().map_or(true, |pattern| {
                        class
                            .as_ref()
                            .is_some_and(|class| Self::matches_exact_pattern(pattern, class))
                    });
                    let title_matches = title_pattern.as_ref().map_or(true, |pattern| {
                        title
                            .as_ref()
                            .is_some_and(|title| Self::matches_generic_pattern(pattern, title))
                    });

                    if app_id_matches && class_matches && title_matches {
                        return value.clone();
                    }
                }
            }
        }

        warn!(
            "No match for app_id=\"{}\" class=\"{}\" title=\"{}\"",
            prettify_option(app_id),
            prettify_option(class),
            prettify_option(title),
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

                if user_config.separator.is_none() {
                    user_config.separator = default.separator
                }

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
        config.fetch_icon(
            Some(&String::from("application")),
            None,
            Some(&String::from("a title"))
        ),
        "d"
    );
}

#[test]
fn test_window_match() {
    let config = Config::from(
        "
    fallback = 'x'
    [matching]
    steam_eve = { app_id = 'steam', title = '/Eve/', value = 's' }
    ",
    );

    assert_eq!(
        config.fetch_icon(
            Some(&String::from("steam")),
            None,
            Some(&String::from("Eve Online"))
        ),
        "s"
    );
    assert_eq!(
        config.fetch_icon(
            Some(&String::from("steam")),
            None,
            Some(&String::from("Counter-Strike"))
        ),
        "x"
    );
}
