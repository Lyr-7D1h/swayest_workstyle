use std::convert::TryFrom;

use toml::Value;

use super::{config_error::ConfigError, Config, Match, Pattern};

/// Parse toml config content to icon_map
pub fn parse_content_to_config(content: &String) -> Result<Config, ConfigError> {
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

            let separator: Option<String> = match root.get("separator") {
                Some(value) => {
                    let f = value
                        .as_str()
                        .ok_or(ConfigError::new("Separator is not a string"))?;
                    Some(f.to_string())
                }
                None => None,
            };

            Ok(Config {
                matchings: matching,
                fallback,
                separator,
            })
        }
        _ => Err(ConfigError::new("No root table found")),
    }
}

#[test]
fn test_parse_content_to_config() {
    use regex::Regex;

    let no_match_table = parse_content_to_config(&String::from("fallback = 'c'"));

    assert_eq!(
        no_match_table.unwrap_err().to_string(),
        "Matching table not found"
    );

    let content = "
    [matching]
    a = b
    ";
    let invalid_match = parse_content_to_config(&content.to_string());
    let e = invalid_match.unwrap_err();
    assert!(
        e.to_string().starts_with("TOML parse error"),
        "error message not as expected: {e:?}"
    );

    let content = "
    [matching]
    
    'fdsa' = 'a'
    '/asdf/' = 'b'
    test = { type = 'generic', value = 'c' }
    qwer = { type = 'exact', value = 'd' }
    ";
    let icon_map = parse_content_to_config(&content.to_string()).unwrap();

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
