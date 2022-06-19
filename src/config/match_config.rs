use std::{convert::TryFrom, str::from_utf8};

use log::{error, info, warn};
use regex::Regex;
use toml::Value;

use crate::util::prettify_option;

use super::{config_error::ConfigError, DEFAULT_CONFIG};

/// Parse toml config content to icon_map
fn parse_content_to_icon_map(content: &String) -> Result<MatchConfig, ConfigError> {
    let map: Value = toml::from_str(content)?;

    let map_to_match = |k: (&String, &Value)| -> Result<Match, ConfigError> {
        if let Some(value) = k.1.as_str() {
            let value = value.to_string();
            let pattern = Pattern::try_from(k.0.to_string()).or(Err(ConfigError::new(format!(
                "Invalid pattern given: {}",
                k.0
            ))))?;

            match pattern {
                Pattern::Regex(_) => return Ok(Match::Generic { pattern, value }),
                Pattern::String(pattern) => return Ok(Match::Exact { pattern, value }),
            };
        }

        if let Some(table) = k.1.as_table() {
            let match_type = table
                .get("type")
                .ok_or(ConfigError::new(format!("Could not parse: {}", k.0)))?
                .as_str()
                .ok_or(ConfigError::new(format!(
                    "Value of {} is not a string",
                    k.0
                )))?;

            let value = table
                .get("value")
                .ok_or(ConfigError::new(format!("Could not parse: {}", k.0)))?
                .as_str()
                .ok_or(ConfigError::new(format!(
                    "Value of {} is not a string",
                    k.0
                )))?
                .to_string();

            let m = match &match_type[..] {
                "exact" => Match::Exact {
                    pattern: k.0.to_string(),
                    value,
                },
                "generic" => Match::Generic {
                    pattern: Pattern::try_from(k.0.to_string()).or(Err(ConfigError::new(
                        format!("Invalid pattern given: {}", k.0),
                    )))?,

                    value,
                },
                _ => return Err(ConfigError::new(format!("Invalid match type: {}", k.1))),
            };

            return Ok(m);
        }

        Err(ConfigError::new(format!(
            "{} could not be parsed as a table",
            k.1
        )))
    };

    match map {
        Value::Table(root) => {
            let matching: Vec<Match> = root
                .get("matching")
                .ok_or(ConfigError::new("Matching table not found"))?
                .as_table()
                .ok_or(ConfigError::new("Could not parse matching table"))?
                .iter()
                .map(map_to_match)
                .collect::<Result<Vec<Match>, ConfigError>>()?
                .into_iter()
                .collect();

            let fallback: Option<String> = match root.get("fallback") {
                Some(value) => {
                    let f = value
                        .as_str()
                        .ok_or(ConfigError::new("Fallback is not a string"))?;
                    Some(f.to_string())
                }
                None => None,
            };

            Ok(MatchConfig {
                matchings: matching,
                fallback,
            })
        }
        _ => Err(ConfigError::new("No root table found")),
    }
}

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
enum Match {
    Generic { pattern: Pattern, value: String },
    Exact { pattern: String, value: String },
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

impl<S: Into<String>> From<S> for MatchConfig {
    fn from(value: S) -> Self {
        let value = value.into();
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

#[test]
fn test_default() {
    let config = MatchConfig::default();
    assert_eq!(config.fallback.unwrap(), "")
}

#[test]
fn test_from_string() {
    let config = MatchConfig::from(
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

#[test]
fn test_parse_content_to_icon_map() {
    let no_match_table = parse_content_to_icon_map(&String::from("fallback = 'c'"));

    assert_eq!(
        no_match_table.unwrap_err().to_string(),
        "Matching table not found"
    );

    let content = "
    [matching]
    a = b
    ";
    let invalid_match = parse_content_to_icon_map(&content.to_string());
    assert!(invalid_match
        .unwrap_err()
        .to_string()
        .starts_with("invalid TOML value"));

    let content = "
    [matching]
    
    'fdsa' = 'a'
    '/asdf/' = 'b'
    test = { type = 'generic', value = 'c' }
    qwer = { type = 'exact', value = 'd' }
    ";
    let icon_map = parse_content_to_icon_map(&content.to_string()).unwrap();

    assert_eq!(
        icon_map.matchings[0],
        Match::Exact {
            value: "a".to_string(),
            pattern: "fdsa".to_string()
        }
    );
    assert_eq!(
        icon_map.matchings[1],
        Match::Generic {
            value: "b".to_string(),
            pattern: Pattern::Regex(Regex::new("asdf").unwrap())
        }
    );
    assert_eq!(
        icon_map.matchings[2],
        Match::Generic {
            value: "c".to_string(),
            pattern: Pattern::String("test".to_string())
        }
    );
    assert_eq!(
        icon_map.matchings[3],
        Match::Exact {
            value: "d".to_string(),
            pattern: "qwer".to_string()
        }
    );
}
