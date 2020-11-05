use std::env;

/// If `name` is set and, after trimming whitespace, is not empty string, then return that trimmed
/// string. Else None.
pub fn get_env_var(name: &str) -> Option<String> {
    match env::var(name).unwrap_or_else(|_| "".to_string()).trim() {
        "" => None,
        non_empty_string => Some(non_empty_string.to_string()),
    }
}

/// If `name` is set to any value at all (including "") then return true; else false.
pub fn get_boolean_env_var(name: &str) -> bool {
    env::var(name).ok().is_some()
}
