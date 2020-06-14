pub fn git_config_get<T>(key: &str, git_config: git2::Config) -> Option<T>
where
    T: GitConfigGet,
{
    T::git_config_get(key, &git_config)
}

pub trait GitConfigGet {
    fn git_config_get(key: &str, git_config: &git2::Config) -> Option<Self>
    where
        Self: Sized;
}

impl GitConfigGet for String {
    fn git_config_get(key: &str, git_config: &git2::Config) -> Option<Self> {
        git_config.get_string(key).ok()
    }
}

impl GitConfigGet for bool {
    fn git_config_get(key: &str, git_config: &git2::Config) -> Option<Self> {
        git_config.get_bool(key).ok()
    }
}

impl GitConfigGet for i64 {
    fn git_config_get(key: &str, git_config: &git2::Config) -> Option<Self> {
        git_config.get_i64(key).ok()
    }
}
