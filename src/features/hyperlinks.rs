use std::borrow::Cow;

use lazy_static::lazy_static;
use regex::{Captures, Regex};

use crate::config::Config;
use crate::features::OptionValueFunction;
use crate::git_config_entry::{GitConfigEntry, GitRemoteRepo};

#[cfg(not(tarpaulin_include))]
pub fn make_feature() -> Vec<(String, OptionValueFunction)> {
    builtin_feature!([
        (
            "hyperlinks",
            bool,
            None,
            _opt => true
        )
    ])
}

pub fn format_commit_line_with_osc8_commit_hyperlink<'a>(
    line: &'a str,
    config: &Config,
) -> Cow<'a, str> {
    if let Some(GitConfigEntry::GitRemote(GitRemoteRepo::GitHubRepo(repo))) =
        config.git_config_entries.get("remote.origin.url")
    {
        COMMIT_LINE_REGEX.replace(line, |captures: &Captures| {
            format_commit_line_captures_with_osc8_commit_hyperlink(captures, repo)
        })
    } else {
        Cow::from(line)
    }
}

/// Create a file hyperlink to `path`, displaying `text`.
pub fn format_osc8_file_hyperlink<'a>(
    relative_path: &'a str,
    line_number: Option<usize>,
    text: &str,
    config: &Config,
) -> Cow<'a, str> {
    if let Some(GitConfigEntry::Path(workdir)) = config.git_config_entries.get("delta.__workdir__")
    {
        let absolute_path = workdir.join(relative_path);
        let mut url = config
            .hyperlinks_file_link_format
            .replace("{path}", &absolute_path.to_string_lossy());
        if let Some(n) = line_number {
            url = url.replace("{line}", &format!("{}", n))
        } else {
            url = url.replace("{line}", "")
        };
        Cow::from(format!(
            "{osc}8;;{url}{st}{text}{osc}8;;{st}",
            url = url,
            text = text,
            osc = "\x1b]",
            st = "\x1b\\"
        ))
    } else {
        Cow::from(relative_path)
    }
}

lazy_static! {
    static ref COMMIT_LINE_REGEX: Regex = Regex::new("(.* )([0-9a-f]{40})(.*)").unwrap();
}

fn format_commit_line_captures_with_osc8_commit_hyperlink(
    captures: &Captures,
    github_repo: &str,
) -> String {
    let commit = captures.get(2).unwrap().as_str();
    format!(
        "{prefix}{osc}8;;{url}{st}{commit}{osc}8;;{st}{suffix}",
        url = format_github_commit_url(commit, github_repo),
        commit = commit,
        prefix = captures.get(1).unwrap().as_str(),
        suffix = captures.get(3).unwrap().as_str(),
        osc = "\x1b]",
        st = "\x1b\\"
    )
}

fn format_github_commit_url(commit: &str, github_repo: &str) -> String {
    format!("https://github.com/{}/commit/{}", github_repo, commit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::integration_test_utils::integration_test_utils::make_config_from_args_and_git_config;
    use std::fs::remove_file;
    use std::path::{Path, PathBuf};

    #[test]
    fn test_format_commit_line_with_osc8_commit_hyperlink() {
        let git_config_contents = b"
[remote \"origin\"]
    url = git@github.com:dandavison/delta.git
";
        let git_config_path = "delta__format_commit_line_with_osc8_commit_hyperlink.gitconfig";
        let config = make_config_from_args_and_git_config(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert_eq!(
            format_commit_line_with_osc8_commit_hyperlink(
                "commit e198c0d841d9fb660e59e0329235a8601b407c69 (HEAD -> master, origin/master)",
                &config
            ),
            "commit \u{1b}]8;;https://github.com/dandavison/delta/commit/e198c0d841d9fb660e59e0329235a8601b407c69\u{1b}\\e198c0d841d9fb660e59e0329235a8601b407c69\u{1b}]8;;\u{1b}\\ (HEAD -> master, origin/master)"
        );
        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_format_osc8_file_hyperlink() {
        let mut config = make_config_from_args_and_git_config(&[], None, None);
        config.git_config_entries.insert(
            "delta.__workdir__".to_string(),
            GitConfigEntry::Path(PathBuf::from("/working/directory")),
        );
        assert_eq!(
            format!(
                "\x1b]8;;file://{}\x1b\\link-text\x1b]8;;\x1b\\",
                Path::new("/working/directory/relative/path/file.rs").to_string_lossy()
            ),
            format_osc8_file_hyperlink("relative/path/file.rs", None, "link-text", &config)
        )
    }

    #[test]
    fn test_format_osc8_file_hyperlink_with_line_number() {
        let mut config = make_config_from_args_and_git_config(
            &["--hyperlinks-file-link-format", "file-line://{path}:{line}"],
            None,
            None,
        );
        config.git_config_entries.insert(
            "delta.__workdir__".to_string(),
            GitConfigEntry::Path(PathBuf::from("/working/directory")),
        );
        assert_eq!(
            format!(
                "\x1b]8;;file-line://{}:7\x1b\\link-text\x1b]8;;\x1b\\",
                Path::new("/working/directory/relative/path/file.rs").to_string_lossy()
            ),
            format_osc8_file_hyperlink("relative/path/file.rs", Some(7), "link-text", &config)
        )
    }

    #[test]
    fn test_format_github_commit_url() {
        assert_eq!(
            format_github_commit_url(
                "b9a76d4523949c09013f24ff555180da1d39e9e4",
                "dandavison/delta"
            ),
            "https://github.com/dandavison/delta/commit/b9a76d4523949c09013f24ff555180da1d39e9e4"
        )
    }
}
