use toml::Value;

pub const CARGO: &'static str = include_str!("../Cargo.toml");

/// Parse cargo file and return package version
pub fn version() -> String {
    let value: Value = toml::from_str(CARGO).unwrap();

    value
        .get("package")
        .unwrap()
        .get("version")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string()
}
