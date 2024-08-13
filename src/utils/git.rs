use std::process::Command;

pub fn retrieve_git_version() -> Option<(usize, usize)> {
    if let Ok(git_path) = grep_cli::resolve_binary("git") {
        let cmd = Command::new(git_path).arg("--version").output().ok()?;
        parse_git_version(&cmd.stdout)
    } else {
        None
    }
}

fn parse_git_version(output: &[u8]) -> Option<(usize, usize)> {
    let mut parts = output.strip_prefix(b"git version ")?.split(|&b| b == b'.');
    let major = std::str::from_utf8(parts.next()?).ok()?.parse().ok()?;
    let minor = std::str::from_utf8(parts.next()?).ok()?.parse().ok()?;
    Some((major, minor))
}

#[cfg(test)]
mod tests {
    use super::parse_git_version;
    use rstest::rstest;

    #[rstest]
    #[case(b"git version 2.46.0", Some((2, 46)))]
    #[case(b"git version 2.39.3 (Apple Git-146)", Some((2, 39)))]
    #[case(b"", None)]
    fn test_parse_git_version(#[case] input: &[u8], #[case] expected: Option<(usize, usize)>) {
        assert_eq!(parse_git_version(input), expected);
    }
}
