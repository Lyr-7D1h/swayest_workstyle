use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub struct ConfigError {
    message: String,
}

impl ConfigError {
    pub fn new<S: Into<String>>(message: S) -> ConfigError {
        ConfigError {
            message: message.into(),
        }
    }
}

impl<'n> Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{}", self.message);
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        ConfigError::new(e.to_string())
    }
}

impl Error for ConfigError {}
