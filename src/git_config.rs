#[cfg(test)]
use std::path::Path;
use std::process;

use git2;

pub struct GitConfig {
    config: git2::Config,
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
                    repo,
                    enabled: true,
                })
            }
            None => None,
        }
    }

    #[cfg(test)]
    pub fn from_path(path: &Path) -> Self {
        Self {
            config: git2::Config::open(path).unwrap(),
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

pub trait GitConfigGet {
    fn git_config_get(key: &str, git_config: &GitConfig) -> Option<Self>
    where
        Self: Sized;
}

impl GitConfigGet for String {
    fn git_config_get(key: &str, git_config: &GitConfig) -> Option<Self> {
        git_config.config.get_string(key).ok()
    }
}

impl GitConfigGet for Option<String> {
    fn git_config_get(key: &str, git_config: &GitConfig) -> Option<Self> {
        match git_config.config.get_string(key) {
            Ok(value) => Some(Some(value)),
            _ => None,
        }
    }
}

impl GitConfigGet for bool {
    fn git_config_get(key: &str, git_config: &GitConfig) -> Option<Self> {
        git_config.config.get_bool(key).ok()
    }
}

impl GitConfigGet for usize {
    fn git_config_get(key: &str, git_config: &GitConfig) -> Option<Self> {
        match git_config.config.get_i64(key) {
            Ok(value) => Some(value as usize),
            _ => None,
        }
    }
}

impl GitConfigGet for f64 {
    fn git_config_get(key: &str, git_config: &GitConfig) -> Option<Self> {
        match git_config.config.get_string(key) {
            Ok(value) => value.parse::<f64>().ok(),
            _ => None,
        }
    }
}
