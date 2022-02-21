use std::result::Result;
use std::str::FromStr;

use lazy_static::lazy_static;
use regex::Regex;

use crate::errors::*;

#[derive(Clone, Debug)]
pub enum GitConfigEntry {
    Style(String),
    GitRemote(GitRemoteRepo),
}

#[derive(Clone, Debug, PartialEq)]
pub enum GitRemoteRepo {
    GitHubRepo { repo_slug: String },
    GitLabRepo { repo_slug: String },
}

impl GitRemoteRepo {
    pub fn format_commit_url(&self, commit: &str) -> String {
        match self {
            Self::GitHubRepo { repo_slug } => {
                format!("https://github.com/{}/commit/{}", repo_slug, commit)
            }
            Self::GitLabRepo { repo_slug } => {
                format!("https://gitlab.com/{}/-/commit/{}", repo_slug, commit)
            }
        }
    }
}

lazy_static! {
    static ref GITHUB_REMOTE_URL: Regex = Regex::new(
        r"(?x)
        ^
        (?:https://|git@)? # Support both HTTPS and SSH URLs, SSH URLs optionally omitting the git@
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
    static ref GITLAB_REMOTE_URL: Regex = Regex::new(
        r"(?x)
        ^
        (?:https://|git@)? # Support both HTTPS and SSH URLs, SSH URLs optionally omitting the git@
        gitlab\.com
        [:/]              # This separator differs between SSH and HTTPS URLs
        ([^/]+)           # Capture the user/org name
        (/.*)?            # Capture group(s), if any
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
            Ok(Self::GitHubRepo {
                repo_slug: format!(
                    "{user}/{repo}",
                    user = caps.get(1).unwrap().as_str(),
                    repo = caps.get(2).unwrap().as_str()
                ),
            })
        } else if let Some(caps) = GITLAB_REMOTE_URL.captures(s) {
            Ok(Self::GitLabRepo {
                repo_slug: format!(
                    "{user}{groups}/{repo}",
                    user = caps.get(1).unwrap().as_str(),
                    groups = caps.get(2).map(|x| x.as_str()).unwrap_or_default(),
                    repo = caps.get(3).unwrap().as_str()
                ),
            })
        } else {
            Err("Not a GitHub or GitLab repo.".into())
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
            "github.com:dandavison/delta.git",
            "github.com:dandavison/delta",
        ];
        for url in urls {
            let parsed = GitRemoteRepo::from_str(url);
            assert!(parsed.is_ok());
            assert_eq!(
                parsed.unwrap(),
                GitRemoteRepo::GitHubRepo {
                    repo_slug: "dandavison/delta".to_string()
                }
            );
        }
    }

    #[test]
    fn test_format_github_commit_link() {
        let repo = GitRemoteRepo::GitHubRepo {
            repo_slug: "dandavison/delta".to_string(),
        };
        let commit_hash = "d3b07384d113edec49eaa6238ad5ff00";
        assert_eq!(
            repo.format_commit_url(commit_hash),
            format!("https://github.com/dandavison/delta/commit/{}", commit_hash)
        )
    }

    #[test]
    fn test_parse_gitlab_urls() {
        let urls = &[
            (
                "https://gitlab.com/proj/grp/subgrp/repo.git",
                "proj/grp/subgrp/repo",
            ),
            ("https://gitlab.com/proj/grp/repo.git", "proj/grp/repo"),
            ("https://gitlab.com/proj/repo.git", "proj/repo"),
            ("https://gitlab.com/proj/repo", "proj/repo"),
            (
                "git@gitlab.com:proj/grp/subgrp/repo.git",
                "proj/grp/subgrp/repo",
            ),
            ("git@gitlab.com:proj/repo.git", "proj/repo"),
            ("git@gitlab.com:proj/repo", "proj/repo"),
            ("gitlab.com:proj/grp/repo.git", "proj/grp/repo"),
            ("gitlab.com:proj/repo.git", "proj/repo"),
            ("gitlab.com:proj/repo", "proj/repo"),
        ];

        for (url, expected) in urls {
            let parsed = GitRemoteRepo::from_str(url);
            assert!(parsed.is_ok());
            assert_eq!(
                parsed.unwrap(),
                GitRemoteRepo::GitLabRepo {
                    repo_slug: expected.to_string()
                }
            );
        }
    }

    #[test]
    fn test_format_gitlab_commit_link() {
        let repo = GitRemoteRepo::GitLabRepo {
            repo_slug: "proj/grp/repo".to_string(),
        };
        let commit_hash = "d3b07384d113edec49eaa6238ad5ff00";
        assert_eq!(
            repo.format_commit_url(commit_hash),
            format!("https://gitlab.com/proj/grp/repo/-/commit/{}", commit_hash)
        )
    }
}
