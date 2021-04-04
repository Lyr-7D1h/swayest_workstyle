extern crate dirs;

use std::{
    fs::{create_dir_all, read_to_string, File},
    io::{Error, ErrorKind, Write},
    str::from_utf8,
};

use super::util::prettify_option;
use log::{error, info, warn};
use swayipc::reply::Node;
use toml::Value;

#[derive(Clone)]
enum MatchType {
    Exact,
    Generic,
}
#[derive(Clone)]
struct Match {
    pattern: String,
    value: String,
    match_type: MatchType,
}
#[derive(Clone)]
struct MatchConfig {
    matching: Vec<Match>,
    fallback: Option<String>,
}

pub struct Config {
    match_config: MatchConfig,
}

const DEFAULT_CONFIG: &'static [u8; 1151] = include_bytes!("default_config.toml");

/// Fetch user config content and create a config file if does not exist
fn get_user_config_content() -> Result<String, Error> {
    let sworkstyle_config_dir = dirs::config_dir()
        .ok_or(Error::new(
            ErrorKind::Other,
            "Missing default XDG Config directory",
        ))?
        .join("sworkstyle");

    create_dir_all(&sworkstyle_config_dir)?;

    let sworkstyle_config_path = sworkstyle_config_dir.join("config.toml");

    let content: String;
    if !sworkstyle_config_path.exists() {
        let mut config_file = File::create(sworkstyle_config_path)?;
        config_file.write_all(DEFAULT_CONFIG)?;
        content = from_utf8(DEFAULT_CONFIG)
            .map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("Failed to convert default content to string: {}", e),
                )
            })?
            .to_string()
    } else {
        content = read_to_string(sworkstyle_config_path)?;
    }

    Ok(content)
}

/// Parse toml config content to icon_map
fn parse_content_to_icon_map(content: &String) -> Result<MatchConfig, Error> {
    let map: Value = toml::from_str(content).unwrap();

    let map_to_match = |k: (&String, &Value)| {
        if let Some(value) = k.1.as_str() {
            return Ok(Match {
                pattern: k.0.to_string(),
                value: value.to_string(),
                match_type: MatchType::Exact,
            });
        }

        if let Some(table) = k.1.as_table() {
            let match_type = table
                .get("type")
                .ok_or(Error::new(
                    ErrorKind::InvalidData,
                    format!("could not parse: {}", k.0),
                ))?
                .as_str()
                .ok_or(Error::new(
                    ErrorKind::InvalidData,
                    format!("value of {} is not a string", k.0),
                ))?;

            let match_type = match &match_type[..] {
                "exact" => MatchType::Exact,
                "generic" => MatchType::Generic,
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "Invalid match type should be exact of generic but found {}",
                            match_type
                        ),
                    ))
                }
            };

            let value = table
                .get("value")
                .ok_or(Error::new(
                    ErrorKind::InvalidData,
                    format!("could not parse: {}", k.0),
                ))?
                .as_str()
                .ok_or(Error::new(
                    ErrorKind::InvalidData,
                    format!("value of {} is not a string", k.0),
                ))?
                .to_string();

            return Ok(Match {
                pattern: k.0.to_string(),
                value,
                match_type,
            });
        }

        Err(Error::new(
            ErrorKind::InvalidData,
            format!("could not parse {}", k.1),
        ))
    };

    match map {
        Value::Table(root) => {
            let matching: Vec<Match> = root
                .get("matching")
                .ok_or(Error::new(
                    ErrorKind::InvalidData,
                    "matching table not found",
                ))?
                .as_table()
                .ok_or(Error::new(
                    ErrorKind::InvalidData,
                    "could not parse exact to table",
                ))?
                .iter()
                .map(map_to_match)
                .collect::<Result<Vec<Match>, Error>>()?
                .into_iter()
                .collect();

            let fallback: Option<String> = match root.get("fallback") {
                Some(value) => {
                    let f = value.as_str().ok_or(Error::new(
                        ErrorKind::InvalidData,
                        "fallback is not a string",
                    ))?;
                    Some(f.to_string())
                }
                None => None,
            };

            Ok(MatchConfig { matching, fallback })
        }
        _ => Err(Error::new(ErrorKind::InvalidData, "no root table found")),
    }
}

fn get_match_config() -> MatchConfig {
    let get_user_config = || -> Result<MatchConfig, Error> {
        let content = get_user_config_content()?;
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
    pub fn new() -> Result<Config, Error> {
        let match_config = get_match_config();

        Ok(Config { match_config })
    }

    pub fn update(&mut self) {
        self.match_config = get_match_config();
    }

    pub fn fetch_icon(&mut self, node: &Node) -> String {
        // Ensure latest config version
        self.update();

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
                    if let MatchType::Generic = m.match_type {
                        if generic_name.to_lowercase().contains(&m.pattern) {
                            return m.value.clone();
                        }
                    } else if m.pattern == *exact_name {
                        return m.value.clone();
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
