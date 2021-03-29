use std::fmt::Display;

/// Map an option to a printable string
pub fn prettify_option<T: Display>(option: Option<T>) -> String {
    match option {
        Some(a) => a.to_string(),
        None => "-".to_string(),
    }
}
