#[cfg(test)]
use std::path::Path;

use std::process;

use git2;

pub struct GitConfig {
    config: git2::Config,
}

impl GitConfig {
    pub fn try_create() -> Option<Self> {
        match std::env::current_dir() {
            Ok(dir) => match git2::Repository::discover(dir) {
                Ok(repo) => match repo.config() {
                    Ok(mut config) => {
                        let config = config.snapshot().unwrap_or_else(|err| {
                            eprintln!("Failed to read git config: {}", err);
                            process::exit(1)
                        });
                        Some(Self { config })
                    }
                    Err(_) => None,
                },
                Err(_) => None,
            },
            Err(_) => None,
        }
    }

    #[cfg(test)]
    pub fn from_path(path: &Path) -> Self {
        Self {
            config: git2::Config::open(path).unwrap(),
        }
    }

    pub fn get<T>(&self, key: &str) -> Option<T>
    where
        T: GitConfigGet,
    {
        T::git_config_get(key, self)
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

impl GitConfigGet for bool {
    fn git_config_get(key: &str, git_config: &GitConfig) -> Option<Self> {
        git_config.config.get_bool(key).ok()
    }
}

impl GitConfigGet for i64 {
    fn git_config_get(key: &str, git_config: &GitConfig) -> Option<Self> {
        git_config.config.get_i64(key).ok()
    }
}
