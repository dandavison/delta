use std::env;

/// If key is set and, after trimming whitespace, is not empty string, then return that trimmed
/// string. Else None.
pub fn get_env_var(key: &str) -> Option<String> {
    match env::var(key).unwrap_or("".to_string()).trim() {
        "" => None,
        non_empty_string => Some(non_empty_string.to_string()),
    }
}
