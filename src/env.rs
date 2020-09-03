use std::env;

/// If `name` is set and, after trimming whitespace, is not empty string, then return that trimmed
/// string. Else None.
pub fn get_env_var(name: &str) -> Option<String> {
    match env::var(name).unwrap_or("".to_string()).trim() {
        "" => None,
        non_empty_string => Some(non_empty_string.to_string()),
    }
}

pub fn get_boolean_env_var(name: &str) -> bool {
    let val = get_env_var(name).map(|s| s.to_lowercase());
    match val.as_deref() {
        None => false,
        Some("false") => false,
        Some(_) => true,
    }
}
