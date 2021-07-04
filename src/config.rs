extern crate dirs;

use std::{
    error::Error,
    fs::{create_dir_all, read_to_string, File},
    io::Write,
    str::from_utf8,
};

use super::util::prettify_option;
use anyhow::{bail, Context};
use log::{error, info, warn};
use regex::Regex;
use swayipc::reply::Node;
use toml::Value;

const DEFAULT_CONFIG: &'static [u8; 1291] = include_bytes!("default_config.toml");

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
fn get_user_config_content() -> anyhow::Result<String> {
    let sworkstyle_config_dir = match dirs::config_dir() {
        Some(dir) => dir.join("sworkstyle"),
        None => bail!("Could not find config dir"),
    };

    create_dir_all(&sworkstyle_config_dir)?;

    let sworkstyle_config_path = sworkstyle_config_dir.join("config.toml");

    let content: String;
    if !sworkstyle_config_path.exists() {
        let mut config_file = File::create(sworkstyle_config_path)?;
        config_file.write_all(DEFAULT_CONFIG)?;
        content = from_utf8(DEFAULT_CONFIG)
            .with_context(|| "Failed to convert default content to string")?
            .to_string()
    } else {
        content = read_to_string(sworkstyle_config_path)?;
    }

    Ok(content)
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

fn get_match_config() -> MatchConfig {
    let get_user_config = || -> anyhow::Result<MatchConfig> {
        let content =
            get_user_config_content().with_context(|| "Could not get user config content")?;
        parse_content_to_icon_map(&content)
    };

    // Use default_icon_map if user config does not work
    match get_user_config() {
        Ok(im) => im,
        Err(e) => {
            error!("Invalid config format: {}", e);
            info!("Using default config");
            let default_content = from_utf8(DEFAULT_CONFIG).unwrap().to_string();
            parse_content_to_icon_map(&default_content).unwrap()
        }
    }
}

impl Config {
    pub fn new() -> Result<Config, Box<dyn Error>> {
        let match_config = get_match_config();

        Ok(Config { match_config })
    }

    pub fn update(&mut self) {
        self.match_config = get_match_config();
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

        if let Some(exact_name) = exact_name {
            if let Some(generic_name) = &node.name {
                for m in &self.match_config.matching {
                    match m {
                        Match::Generic(m) => match &m.pattern {
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
                        },
                        Match::Exact(m) => {
                            if exact_name == &m.pattern {
                                return m.value.clone();
                            }
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
