use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum ConfigErrorKind {
    InvalidFile,
    TomlParseError,
    GenericParseError,
}

#[derive(Debug)]
pub struct ConfigError {
    kind: ConfigErrorKind,
    details: Option<toml::de::Error>,
    message: String,
}

impl ConfigError {
    pub fn new<S: Into<String>>(kind: ConfigErrorKind, message: S) -> ConfigError {
        ConfigError {
            kind,
            details: None,
            message: message.into(),
        }
    }

    pub fn details(&mut self, details: toml::de::Error) {
        self.details = Some(details)
    }
}

impl<'n> Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(details) = &self.details {
            return write!(f, "{}: {}", self.message, details);
        }

        return write!(f, "{}", self.message);
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        ConfigError::new(ConfigErrorKind::TomlParseError, e.to_string())
    }
}

impl Error for ConfigError {}
