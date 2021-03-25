extern crate dirs;

use std::{
    collections::HashMap,
    fs::{create_dir_all, read_to_string, File},
    io::{Error, ErrorKind, Write},
    str::from_utf8,
};

use swayipc_async::Node;
use toml::Value;

#[derive(Clone)]
struct IconMap {
    exact: HashMap<String, String>,
    generic: HashMap<String, String>,
    fallback: String,
}

pub struct Config {
    icon_map: IconMap,
}

const DEFAULT_CONFIG: &'static [u8; 790] = include_bytes!("default_config.toml");

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
fn parse_content_to_icon_map(content: &String) -> Result<IconMap, Error> {
    let map: Value = toml::from_str(content).unwrap();

    let map_to_string = |k: (&String, &Value)| {
        if let Some(value) = k.1.as_str() {
            return Ok((k.0.to_string(), value.to_string()));
        }

        Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid config format: could not parse {}", k.1),
        ))
    };

    println!("{:?}", map);
    match map {
        Value::Table(root) => {
            let exact = root
                .get("exact")
                .ok_or(Error::new(
                    ErrorKind::InvalidData,
                    "Invalid config format: exact table not found",
                ))?
                .as_table()
                .ok_or(Error::new(
                    ErrorKind::InvalidData,
                    "Invalid config format: could not parse exact to table",
                ))?
                .iter()
                .map(map_to_string)
                .collect::<Result<Vec<(String, String)>, Error>>()?
                .into_iter()
                .collect();

            let generic = root
                .get("generic")
                .ok_or(Error::new(
                    ErrorKind::InvalidData,
                    "Invalid config format: generic table not found",
                ))?
                .as_table()
                .ok_or(Error::new(
                    ErrorKind::InvalidData,
                    "Invalid config format: could not parse generic",
                ))?
                .iter()
                .map(map_to_string)
                .collect::<Result<Vec<(String, String)>, Error>>()?
                .into_iter()
                .collect();

            let fallback: String = match root["fallback"].as_str() {
                Some(fallback) => fallback.to_string(),
                _ => {
                    println!("No fallback set");
                    "A".to_string()
                }
            };

            Ok(IconMap {
                exact,
                generic,
                fallback,
            })
        }
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            "Invalid config format: no root table",
        )),
    }
}

impl Config {
    pub fn new() -> Result<Config, Error> {
        let get_user_config = || -> Result<IconMap, Error> {
            let content = get_user_config_content()?;
            parse_content_to_icon_map(&content)
        };

        // Use default_icon_map if user config does not work
        let icon_map = match get_user_config() {
            Ok(im) => im,
            Err(e) => {
                println!("{}", e);
                let default_content = from_utf8(DEFAULT_CONFIG)
                    .map_err(|e| {
                        Error::new(
                            ErrorKind::Other,
                            format!("Failed to convert default content to string: {}", e),
                        )
                    })?
                    .to_string();
                parse_content_to_icon_map(&default_content)?
            }
        };

        Ok(Config { icon_map })
    }

    pub fn fetch_icon(&self, container: &Node) -> &String {
        // Wayland Exact app
        if let Some(app_id) = &container.app_id {
            if let Some(icon) = self.icon_map.exact.get(app_id) {
                return icon;
            }
        }

        // X11 Exact
        if let Some(window_props) = &container.window_properties {
            if let Some(class) = &window_props.class {
                if let Some(icon) = self.icon_map.exact.get(class) {
                    return icon;
                }
            }
        }

        // Generic matching
        if let Some(name) = &container.name {
            for key in self.icon_map.generic.keys() {
                if key.contains(name) {
                    return self.icon_map.generic.get(name).unwrap();
                }
            }
        }

        return &self.icon_map.fallback;
    }
}
