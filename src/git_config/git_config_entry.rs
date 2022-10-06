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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitRemoteRepo {
    GitHub { slug: String },
    GitLab { slug: String },
    SourceHut { slug: String },
    Codeberg { slug: String },
}

impl GitRemoteRepo {
    pub fn format_commit_url(&self, commit: &str) -> String {
        match self {
            Self::GitHub { slug } => {
                format!("https://github.com/{}/commit/{}", slug, commit)
            }
            Self::GitLab { slug } => {
                format!("https://gitlab.com/{}/-/commit/{}", slug, commit)
            }
            Self::SourceHut { slug } => {
                format!("https://git.sr.ht/{}/commit/{}", slug, commit)
            }
            Self::Codeberg { slug } => {
                format!("https://codeberg.org/{}/commit/{}", slug, commit)
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
    static ref SOURCEHUT_REMOTE_URL: Regex = Regex::new(
        r"(?x)
        ^
        (?:https://|git@)? # Support both HTTPS and SSH URLs, SSH URLs optionally omitting the git@
        git\.sr\.ht
        [:/]              # This separator differs between SSH and HTTPS URLs
        ~([^/]+)          # Capture the username
        /
        (.+)             # Capture the repo name
        $
        "
    )
    .unwrap();
    static ref CODEBERG_REMOTE_URL: Regex = Regex::new(
        r"(?x)
        ^
        (?:https://|git@)? # Support both HTTPS and SSH URLs, SSH URLs optionally omitting the git@
        codeberg\.org
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
            Ok(Self::GitHub {
                slug: format!(
                    "{user}/{repo}",
                    user = caps.get(1).unwrap().as_str(),
                    repo = caps.get(2).unwrap().as_str()
                ),
            })
        } else if let Some(caps) = GITLAB_REMOTE_URL.captures(s) {
            Ok(Self::GitLab {
                slug: format!(
                    "{user}{groups}/{repo}",
                    user = caps.get(1).unwrap().as_str(),
                    groups = caps.get(2).map(|x| x.as_str()).unwrap_or_default(),
                    repo = caps.get(3).unwrap().as_str()
                ),
            })
        } else if let Some(caps) = SOURCEHUT_REMOTE_URL.captures(s) {
            Ok(Self::SourceHut {
                slug: format!(
                    "~{user}/{repo}",
                    user = caps.get(1).unwrap().as_str(),
                    repo = caps.get(2).unwrap().as_str()
                ),
            })
        } else if let Some(caps) = CODEBERG_REMOTE_URL.captures(s) {
            Ok(Self::Codeberg {
                slug: format!(
                    "{user}/{repo}",
                    user = caps.get(1).unwrap().as_str(),
                    repo = caps.get(2).unwrap().as_str()
                ),
            })
        } else {
            Err("Not a GitHub, GitLab, SourceHut or Codeberg repo.".into())
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
                GitRemoteRepo::GitHub {
                    slug: "dandavison/delta".to_string()
                }
            );
        }
    }

    #[test]
    fn test_format_github_commit_link() {
        let repo = GitRemoteRepo::GitHub {
            slug: "dandavison/delta".to_string(),
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
                GitRemoteRepo::GitLab {
                    slug: expected.to_string()
                }
            );
        }
    }

    #[test]
    fn test_format_gitlab_commit_link() {
        let repo = GitRemoteRepo::GitLab {
            slug: "proj/grp/repo".to_string(),
        };
        let commit_hash = "d3b07384d113edec49eaa6238ad5ff00";
        assert_eq!(
            repo.format_commit_url(commit_hash),
            format!("https://gitlab.com/proj/grp/repo/-/commit/{}", commit_hash)
        )
    }

    #[test]
    fn test_parse_sourcehut_urls() {
        let urls = &[
            "https://git.sr.ht/~someuser/somerepo",
            "git@git.sr.ht:~someuser/somerepo",
            "git.sr.ht:~someuser/somerepo",
        ];
        for url in urls {
            let parsed = GitRemoteRepo::from_str(url);
            assert!(parsed.is_ok());
            assert_eq!(
                parsed.unwrap(),
                GitRemoteRepo::SourceHut {
                    slug: "~someuser/somerepo".to_string()
                }
            );
        }
    }

    #[test]
    fn test_format_sourcehut_commit_link() {
        let repo = GitRemoteRepo::SourceHut {
            slug: "~someuser/somerepo".to_string(),
        };
        let commit_hash = "df41ac86f08a40e64c76062fd67e238522c14990";
        assert_eq!(
            repo.format_commit_url(commit_hash),
            format!(
                "https://git.sr.ht/~someuser/somerepo/commit/{}",
                commit_hash
            )
        )
    }

    #[test]
    fn test_parse_codeberg_urls() {
        let urls = &[
            "https://codeberg.org/someuser/somerepo.git",
            "https://codeberg.org/someuser/somerepo",
            "git@codeberg.org:someuser/somerepo.git",
            "git@codeberg.org:someuser/somerepo",
            "codeberg.org:someuser/somerepo.git",
            "codeberg.org:someuser/somerepo",
        ];
        for url in urls {
            let parsed = GitRemoteRepo::from_str(url);
            assert!(parsed.is_ok());
            assert_eq!(
                parsed.unwrap(),
                GitRemoteRepo::Codeberg {
                    slug: "someuser/somerepo".to_string()
                }
            );
        }
    }

    #[test]
    fn test_format_codeberg_commit_link() {
        let repo = GitRemoteRepo::Codeberg {
            slug: "dnkl/foot".to_string(),
        };
        let commit_hash = "1c072856ebf12419378c5098ad543c497197c6da";
        assert_eq!(
            repo.format_commit_url(commit_hash),
            format!("https://codeberg.org/dnkl/foot/commit/{}", commit_hash)
        )
    }
}
