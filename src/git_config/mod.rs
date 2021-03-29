mod git_config_entry;

pub use git_config_entry::{GitConfigEntry, GitRemoteRepo};

use regex::Regex;
use std::collections::HashMap;
use std::env;
#[cfg(test)]
use std::path::Path;
use std::process;

use lazy_static::lazy_static;

pub struct GitConfig {
    pub config: git2::Config,
    config_from_env_var: HashMap<String, String>,
    pub enabled: bool,
    pub repo: Option<git2::Repository>,
}

impl GitConfig {
    pub fn try_create() -> Option<Self> {
        let repo = match std::env::current_dir() {
            Ok(dir) => git2::Repository::discover(dir).ok(),
            _ => None,
        };
        let config = match &repo {
            Some(repo) => repo.config().ok(),
            None => git2::Config::open_default().ok(),
        };
        match config {
            Some(mut config) => {
                let config = config.snapshot().unwrap_or_else(|err| {
                    eprintln!("Failed to read git config: {}", err);
                    process::exit(1)
                });
                Some(Self {
                    config,
                    config_from_env_var: parse_config_from_env_var(),
                    repo,
                    enabled: true,
                })
            }
            None => None,
        }
    }

    #[cfg(test)]
    pub fn from_path(path: &Path, honor_env_var: bool) -> Self {
        Self {
            config: git2::Config::open(path).unwrap(),
            config_from_env_var: if honor_env_var {
                parse_config_from_env_var()
            } else {
                HashMap::new()
            },
            repo: None,
            enabled: true,
        }
    }

    pub fn get<T>(&self, key: &str) -> Option<T>
    where
        T: GitConfigGet,
    {
        if self.enabled {
            T::git_config_get(key, self)
        } else {
            None
        }
    }
}

fn parse_config_from_env_var() -> HashMap<String, String> {
    if let Ok(s) = env::var("GIT_CONFIG_PARAMETERS") {
        parse_config_from_env_var_value(&s)
    } else {
        HashMap::new()
    }
}

lazy_static! {
    static ref GIT_CONFIG_PARAMETERS_REGEX: Regex =
        Regex::new(r"'(delta\.[a-z-]+)=([^']+)'").unwrap();
}

fn parse_config_from_env_var_value(s: &str) -> HashMap<String, String> {
    GIT_CONFIG_PARAMETERS_REGEX
        .captures_iter(s)
        .map(|captures| (captures[1].to_string(), captures[2].to_string()))
        .collect()
}

pub trait GitConfigGet {
    fn git_config_get(key: &str, git_config: &GitConfig) -> Option<Self>
    where
        Self: Sized;
}

impl GitConfigGet for String {
    fn git_config_get(key: &str, git_config: &GitConfig) -> Option<Self> {
        match git_config.config_from_env_var.get(key) {
            Some(val) => Some(val.to_string()),
            None => git_config.config.get_string(key).ok(),
        }
    }
}

impl GitConfigGet for Option<String> {
    fn git_config_get(key: &str, git_config: &GitConfig) -> Option<Self> {
        match git_config.config_from_env_var.get(key) {
            Some(val) => Some(Some(val.to_string())),
            None => match git_config.config.get_string(key) {
                Ok(val) => Some(Some(val)),
                _ => None,
            },
        }
    }
}

impl GitConfigGet for bool {
    fn git_config_get(key: &str, git_config: &GitConfig) -> Option<Self> {
        match git_config.config_from_env_var.get(key).map(|s| s.as_str()) {
            Some("true") => Some(true),
            Some("false") => Some(false),
            _ => git_config.config.get_bool(key).ok(),
        }
    }
}

impl GitConfigGet for usize {
    fn git_config_get(key: &str, git_config: &GitConfig) -> Option<Self> {
        if let Some(s) = git_config.config_from_env_var.get(key) {
            if let Ok(n) = s.parse::<usize>() {
                return Some(n);
            }
        }
        match git_config.config.get_i64(key) {
            Ok(value) => Some(value as usize),
            _ => None,
        }
    }
}

impl GitConfigGet for f64 {
    fn git_config_get(key: &str, git_config: &GitConfig) -> Option<Self> {
        if let Some(s) = git_config.config_from_env_var.get(key) {
            if let Ok(n) = s.parse::<f64>() {
                return Some(n);
            }
        }
        match git_config.config.get_string(key) {
            Ok(value) => value.parse::<f64>().ok(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::parse_config_from_env_var_value;

    #[test]
    fn test_parse_config_from_env_var_value() {
        // To generate test cases, use git -c ... with
        // [core]
        //     pager = env | grep GIT_CONFIG_PARAMETERS

        let config = parse_config_from_env_var_value("'user.name=xxx'");
        assert!(config.is_empty());

        let config = parse_config_from_env_var_value("'delta.plus-style=green'");
        assert_eq!(config["delta.plus-style"], "green");

        let config = parse_config_from_env_var_value(
            r##"'user.name=xxx' 'delta.hunk-header-line-number-style=red "#067a00"'"##,
        );
        assert_eq!(
            config["delta.hunk-header-line-number-style"],
            r##"red "#067a00""##
        );

        let config =
            parse_config_from_env_var_value(r##"'user.name=xxx' 'delta.side-by-side=false'"##);
        assert_eq!(config["delta.side-by-side"], "false");

        let config = parse_config_from_env_var_value(
            r##"'delta.plus-style=green' 'delta.side-by-side=false' 'delta.hunk-header-line-number-style=red "#067a00"'"##,
        );
        assert_eq!(config["delta.plus-style"], "green");
        assert_eq!(config["delta.side-by-side"], "false");
        assert_eq!(
            config["delta.hunk-header-line-number-style"],
            r##"red "#067a00""##
        );
    }
}
