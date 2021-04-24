use std::path::PathBuf;
use std::result::Result;
use std::str::FromStr;

use lazy_static::lazy_static;
use regex::Regex;

use crate::errors::*;

#[derive(Clone, Debug)]
pub enum GitConfigEntry {
    Style(String),
    GitRemote(GitRemoteRepo),
    Path(PathBuf),
}

#[derive(Clone, Debug, PartialEq)]
pub enum GitRemoteRepo {
    GitHubRepo(String),
}

lazy_static! {
    static ref GITHUB_REMOTE_URL: Regex = Regex::new(
        r"(?x)
        ^
        (?:https://|git@) # Support both HTTPS and SSH URLs
        github\.com
        [:/]              # This separator differs between SSH and HTTPS URLs
        ([^/]+)           # Capture the user/org name
        /
        (.+?)             # Capture the repo name (lazy to avoid consuming '.git' if present)
        (?:\.git)?        # Non-capturing group to consume '.git' if present
        $
        "
    )
    .unwrap();
}

impl FromStr for GitRemoteRepo {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(caps) = GITHUB_REMOTE_URL.captures(s) {
            Ok(Self::GitHubRepo(format!(
                "{user}/{repo}",
                user = caps.get(1).unwrap().as_str(),
                repo = caps.get(2).unwrap().as_str()
            )))
        } else {
            Err("Not a GitHub repo.".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_urls() {
        let urls = &[
            "https://github.com/dandavison/delta.git",
            "https://github.com/dandavison/delta",
            "git@github.com:dandavison/delta.git",
            "git@github.com:dandavison/delta",
        ];
        for url in urls {
            let parsed = GitRemoteRepo::from_str(url);
            assert!(parsed.is_ok());
            assert_eq!(
                parsed.unwrap(),
                GitRemoteRepo::GitHubRepo("dandavison/delta".to_string())
            );
        }
    }
}
