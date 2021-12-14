extern crate dirs;

use std::{fs::read_to_string, str::from_utf8};

use super::util::prettify_option;
use anyhow::{bail, Context, Error};
use log::{error, info, warn};
use regex::Regex;
use swayipc::reply::Node;
use toml::Value;

const DEFAULT_CONFIG: &'static [u8] = include_bytes!("default_config.toml");

#[derive(Clone, Debug)]
enum Pattern {
    Regex(Regex),
    String(String),
}

impl Pattern {
    fn from_string(mut string: String) -> anyhow::Result<Pattern> {
        if string.starts_with("/") && string.ends_with("/") {
            string.remove(string.len() - 1);
            string.remove(0);
            let regex = Regex::new(&string).with_context(|| "Invalid regex")?;
            Ok(Pattern::Regex(regex))
        } else {
            Ok(Pattern::String(string))
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
struct MatchConfig {
    matching: Vec<Match>,
    fallback: Option<String>,
}

pub struct Config {
    match_config: MatchConfig,
}

/// Fetch user config content and create a config file if does not exist
fn get_user_config_content(config_path: Option<String>) -> anyhow::Result<Option<String>> {
    if let Some(str_path) = config_path {
        match read_to_string(&str_path) {
            Ok(content) => return Ok(Some(content)),
            Err(e) => error!("Could not open config path: {}, {}", &str_path, e),
        }
    }
    let sworkstyle_config_dir = match dirs::config_dir() {
        Some(dir) => dir.join("sworkstyle"),
        None => bail!("Could not find config directory"),
    };

    let sworkstyle_config_path = sworkstyle_config_dir.join("config.toml");

    if !sworkstyle_config_path.exists() {
        return Ok(None);
    }

    match read_to_string(sworkstyle_config_path) {
        Ok(content) => Ok(Some(content)),
        Err(e) => Err(Error::new(e)),
    }
}

/// Parse toml config content to icon_map
fn parse_content_to_icon_map(content: &String) -> anyhow::Result<MatchConfig> {
    let map: Value = toml::from_str(content)?; //.with_context(|| "Could not parse config content")?;

    let map_to_match = |k: (&String, &Value)| -> anyhow::Result<Match> {
        if let Some(value) = k.1.as_str() {
            let value = value.to_string();
            let pattern = Pattern::from_string(k.0.to_string())
                .with_context(|| format!("Invalid pattern given: {}", k.0))?;

            match pattern {
                Pattern::Regex(_) => return Ok(Match::Generic(GenericMatch { pattern, value })),
                Pattern::String(pattern) => return Ok(Match::Exact(ExactMatch { pattern, value })),
            };
        }

        if let Some(table) = k.1.as_table() {
            let match_type = table
                .get("type")
                .with_context(|| format!("Could not parse: {}", k.0))?
                .as_str()
                .with_context(|| format!("Value of {} is not a string", k.0))?;

            let value = table
                .get("value")
                .with_context(|| format!("Could not parse: {}", k.0))?
                .as_str()
                .with_context(|| format!("Value of {} is not a string", k.0))?
                .to_string();

            let m = match &match_type[..] {
                "exact" => Match::Exact(ExactMatch {
                    pattern: k.0.to_string(),
                    value,
                }),
                "generic" => Match::Generic(GenericMatch {
                    pattern: Pattern::from_string(k.0.to_string())
                        .with_context(|| format!("Failed to parse pattern: {}", k.0))?,
                    value,
                }),
                _ => bail!("Invalid match type: {}", k.1),
            };

            return Ok(m);
        }

        bail!("Could not parse {}", k.1)
    };

    match map {
        Value::Table(root) => {
            let matching: Vec<Match> = root
                .get("matching")
                .with_context(|| "Matching table not found")?
                .as_table()
                .with_context(|| "Could not parse matching table")?
                .iter()
                .map(map_to_match)
                .collect::<anyhow::Result<Vec<Match>>>()?
                .into_iter()
                .collect();

            let fallback: Option<String> = match root.get("fallback") {
                Some(value) => {
                    let f = value.as_str().with_context(|| "Fallback is not a string")?;
                    Some(f.to_string())
                }
                None => None,
            };

            Ok(MatchConfig { matching, fallback })
        }
        _ => bail!("No root table found"),
    }
}

fn get_match_config(config_path: Option<String>) -> MatchConfig {
    let default_config_content = from_utf8(DEFAULT_CONFIG).unwrap().to_string();
    let mut default_config = parse_content_to_icon_map(&default_config_content).unwrap();

    let user_config_content = match get_user_config_content(config_path) {
        Ok(user_config_content) => user_config_content,
        Err(e) => {
            error!("Could not read config: {}", e);
            return default_config;
        }
    };

    match user_config_content {
        Some(user_config_content) => match parse_content_to_icon_map(&user_config_content) {
            Ok(mut user_config) => {
                user_config.matching.append(&mut default_config.matching);

                if user_config.fallback.is_none() {
                    user_config.fallback = default_config.fallback
                }

                user_config
            }
            Err(e) => {
                error!("Invalid config format: {}", e);
                return default_config;
            }
        },
        None => {
            info!("No user config found, using default");
            default_config
        }
    }
}

impl Config {
    pub fn new(config_path: Option<String>) -> Config {
        let match_config = get_match_config(config_path);

        Config { match_config }
    }

    pub fn fetch_icon(&mut self, node: &Node) -> String {
        let mut exact_name: Option<&String> = None;

        // Wayland Exact app
        if let Some(app_id) = &node.app_id {
            exact_name = Some(app_id);
        }

        // X11 Exact
        if let Some(window_props) = &node.window_properties {
            if let Some(class) = &window_props.class {
                exact_name = Some(class);
            }
        }

        for m in &self.match_config.matching {
            match m {
                Match::Generic(m) => {
                    if let Some(generic_name) = &node.name {
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
                    if let Some(exact_name) = exact_name {
                        if exact_name == &m.pattern {
                            return m.value.clone();
                        }
                    }
                }
            }
        }

        warn!(
            "No match for \"{}\" with title \"{}\"",
            prettify_option(exact_name),
            prettify_option(node.name.clone()),
        );

        match &self.match_config.fallback {
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
