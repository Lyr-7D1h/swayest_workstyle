use std::{convert::TryFrom, str::from_utf8};

use log::{error, info, warn};
use regex::Regex;
use toml::Value;

use crate::util::prettify_option;

use super::{
    config_error::{ConfigError, ConfigErrorKind},
    DEFAULT_CONFIG,
};

/// Parse toml config content to icon_map
fn parse_content_to_icon_map(content: &String) -> Result<MatchConfig, ConfigError> {
    let map: Value = toml::from_str(content)?;

    let map_to_match = |k: (&String, &Value)| -> Result<Match, ConfigError> {
        if let Some(value) = k.1.as_str() {
            let value = value.to_string();
            let pattern = Pattern::try_from(k.0.to_string()).or(Err(ConfigError::new(
                ConfigErrorKind::GenericParseError,
                format!("Invalid pattern given: {}", k.0),
            )))?;

            match pattern {
                Pattern::Regex(_) => return Ok(Match::Generic(GenericMatch { pattern, value })),
                Pattern::String(pattern) => return Ok(Match::Exact(ExactMatch { pattern, value })),
            };
        }

        if let Some(table) = k.1.as_table() {
            let match_type = table
                .get("type")
                .ok_or(ConfigError::new(
                    ConfigErrorKind::GenericParseError,
                    format!("Could not parse: {}", k.0),
                ))?
                .as_str()
                .ok_or(ConfigError::new(
                    ConfigErrorKind::GenericParseError,
                    format!("Value of {} is not a string", k.0),
                ))?;

            let value = table
                .get("value")
                .ok_or(ConfigError::new(
                    ConfigErrorKind::GenericParseError,
                    format!("Could not parse: {}", k.0),
                ))?
                .as_str()
                .ok_or(ConfigError::new(
                    ConfigErrorKind::GenericParseError,
                    format!("Value of {} is not a string", k.0),
                ))?
                .to_string();

            let m = match &match_type[..] {
                "exact" => Match::Exact(ExactMatch {
                    pattern: k.0.to_string(),
                    value,
                }),
                "generic" => Match::Generic(GenericMatch {
                    pattern: Pattern::try_from(k.0.to_string()).or(Err(ConfigError::new(
                        ConfigErrorKind::GenericParseError,
                        format!("Invalid pattern given: {}", k.0),
                    )))?,

                    value,
                }),
                _ => {
                    return Err(ConfigError::new(
                        ConfigErrorKind::GenericParseError,
                        format!("Invalid match type: {}", k.1),
                    ))
                }
            };

            return Ok(m);
        }

        Err(ConfigError::new(
            ConfigErrorKind::GenericParseError,
            format!("{} could not be parsed as a table", k.1),
        ))
    };

    match map {
        Value::Table(root) => {
            let matching: Vec<Match> = root
                .get("matching")
                .ok_or(ConfigError::new(
                    ConfigErrorKind::GenericParseError,
                    "Matching table not found",
                ))?
                .as_table()
                .ok_or(ConfigError::new(
                    ConfigErrorKind::GenericParseError,
                    "Could not parse matching table",
                ))?
                .iter()
                .map(map_to_match)
                .collect::<Result<Vec<Match>, ConfigError>>()?
                .into_iter()
                .collect();

            let fallback: Option<String> = match root.get("fallback") {
                Some(value) => {
                    let f = value.as_str().ok_or(ConfigError::new(
                        ConfigErrorKind::GenericParseError,
                        "Fallback is not a string",
                    ))?;
                    Some(f.to_string())
                }
                None => None,
            };

            Ok(MatchConfig {
                matchings: matching,
                fallback,
            })
        }
        _ => Err(ConfigError::new(
            ConfigErrorKind::GenericParseError,
            "No root table found",
        )),
    }
}

#[derive(Clone, Debug)]
pub enum Pattern {
    Regex(Regex),
    String(String),
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

#[derive(Clone, Debug)]
struct GenericMatch {
    pattern: Pattern,
    value: String,
}

#[derive(Clone, Debug)]
struct ExactMatch {
    pattern: String,
    value: String,
}

#[derive(Clone, Debug)]
enum Match {
    Generic(GenericMatch),
    Exact(ExactMatch),
}

#[derive(Clone, Debug)]
pub struct MatchConfig {
    matchings: Vec<Match>,
    fallback: Option<String>,
}

impl MatchConfig {
    pub fn fetch_icon(&self, exact_name: &String, generic_name: Option<&String>) -> String {
        for m in &self.matchings {
            match m {
                Match::Generic(m) => {
                    if let Some(generic_name) = &generic_name {
                        match &m.pattern {
                            Pattern::Regex(r) => {
                                if r.is_match(generic_name) {
                                    return m.value.clone();
                                }
                            }
                            Pattern::String(p) => {
                                if generic_name.to_lowercase().contains(&p.to_lowercase()) {
                                    return m.value.clone();
                                }
                            }
                        }
                    }
                }
                Match::Exact(m) => {
                    if exact_name == &m.pattern {
                        return m.value.clone();
                    }
                }
            }
        }

        warn!(
            "No match for \"{}\" with title \"{}\"",
            exact_name,
            prettify_option(generic_name),
        );

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

impl From<String> for MatchConfig {
    fn from(value: String) -> Self {
        let mut default = MatchConfig::default();

        match parse_content_to_icon_map(&value) {
            Ok(mut user_config) => {
                user_config.matchings.append(&mut default.matchings);

                if user_config.fallback.is_none() {
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
impl Default for MatchConfig {
    fn default() -> Self {
        let default_config_content = from_utf8(DEFAULT_CONFIG).unwrap().to_string();
        return parse_content_to_icon_map(&default_config_content).unwrap();
    }
}
