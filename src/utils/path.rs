use std::path::{Component, Path, PathBuf, MAIN_SEPARATOR_STR};

use crate::config::Config;

use super::process::calling_process;

// Infer absolute path to `relative_path`.
pub fn absolute_path(relative_path: &str, config: &Config) -> Option<PathBuf> {
    let caller = calling_process();
    if let Some(path) = root_relative_path(relative_path, config) {
        return Some(normalize_path(path));
    }
    match (
        &config.cwd_of_delta_process,
        &config.cwd_of_user_shell_process,
        caller.paths_in_input_are_relative_to_cwd() || config.relative_paths,
    ) {
        // Note that if we were invoked by git then cwd_of_delta_process == repo_root
        (Some(cwd_of_delta_process), _, false) => Some(cwd_of_delta_process.join(relative_path)),
        (_, Some(cwd_of_user_shell_process), true) => {
            Some(cwd_of_user_shell_process.join(relative_path))
        }
        (Some(cwd_of_delta_process), None, true) => {
            // This might occur when piping from git to delta?
            Some(cwd_of_delta_process.join(relative_path))
        }
        _ => None,
    }
    .map(normalize_path)
}

/// `git diff --no-index` strips the leading separator from absolute path arguments, so the paths
/// it emits for those arguments are relative to the filesystem root, not to the cwd. Return the
/// intended absolute path if `path` is such a path. See #1928.
fn root_relative_path(path: &str, config: &Config) -> Option<PathBuf> {
    let candidate = PathBuf::from(MAIN_SEPARATOR_STR).join(path);
    if !candidate.is_absolute() {
        // E.g. on Windows, where `git diff --no-index` does not emit such paths.
        return None;
    }
    // Only when delta ran `git diff --no-index` itself, so the two files are known exactly.
    // Guessing from the path alone would rewrite paths in ordinary diffs.
    (config.minus_file.as_ref() == Some(&candidate)
        || config.plus_file.as_ref() == Some(&candidate))
    .then_some(candidate)
}

#[allow(clippy::needless_borrows_for_generic_args)] // Lint has known problems, &path != path
/// Relativize `path` if delta `config` demands that and paths are not already relativized by git.
pub fn relativize_path_maybe(path: &mut String, config: &Config) {
    let mut inner_relativize = || -> Option<()> {
        let base = config.cwd_relative_to_repo_root.as_deref()?;
        let relative_path = pathdiff::diff_paths(&path, base)?;
        if relative_path.is_relative() {
            #[cfg(target_os = "windows")]
            // '/dev/null' is converted to '\dev\null' and considered relative. Work
            // around that by leaving all paths like that untouched:
            if relative_path.starts_with(Path::new(r"\")) {
                return None;
            }
            *path = relative_path.to_string_lossy().into_owned();
        }
        Some(())
    };
    if config.relative_paths && !calling_process().paths_in_input_are_relative_to_cwd() {
        let _ = inner_relativize();
    }
}

/// Return current working directory of the user's shell process. I.e. the directory which they are
/// in when delta exits. This is the directory relative to which the file paths in delta output are
/// constructed if they are using either (a) delta's relative-paths option or (b) git's --relative
/// flag.
pub fn cwd_of_user_shell_process(
    cwd_of_delta_process: Option<&PathBuf>,
    cwd_relative_to_repo_root: Option<&str>,
) -> Option<PathBuf> {
    match (cwd_of_delta_process, cwd_relative_to_repo_root) {
        (Some(cwd), None) => {
            // We are not a child process of git
            Some(PathBuf::from(cwd))
        }
        (Some(repo_root), Some(cwd_relative_to_repo_root)) => {
            // We are a child process of git; git spawned us from repo_root and preserved the user's
            // original cwd in the GIT_PREFIX env var (available as config.cwd_relative_to_repo_root)
            Some(PathBuf::from(repo_root).join(cwd_relative_to_repo_root))
        }
        (None, _) => {
            // Unexpected
            None
        }
    }
}

// Copied from
// https://github.com/rust-lang/cargo/blob/c6745a3d7fcea3a949c3e13e682b8ddcbd213add/crates/cargo-util/src/paths.rs#L73-L106
// as suggested by matklad: https://www.reddit.com/r/rust/comments/hkkquy/comment/fwtw53s/?utm_source=share&utm_medium=web2x&context=3
fn normalize_path<P>(path: P) -> PathBuf
where
    P: AsRef<Path>,
{
    let mut components = path.as_ref().components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

#[cfg(test)]
pub fn fake_delta_cwd_for_tests() -> PathBuf {
    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from("/fake/delta/cwd")
    }
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(r"C:\fake\delta\cwd")
    }
}

#[cfg(not(target_os = "windows"))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::integration_test_utils::make_config_from_args;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_absolute_path_of_root_relative_path_from_delta_diff() {
        // `delta /tmp/a.txt /tmp/b.txt` runs `git diff --no-index`, which emits the paths with
        // their leading separator stripped, i.e. relative to the filesystem root. The cwd must not
        // be prepended to them. https://github.com/dandavison/delta/issues/1928
        let mut config = make_config_from_args(&[]);
        config.minus_file = Some(PathBuf::from("/tmp/a.txt"));
        config.plus_file = Some(PathBuf::from("/tmp/b.txt"));

        assert_eq!(
            absolute_path("tmp/a.txt", &config),
            Some(PathBuf::from("/tmp/a.txt"))
        );
        assert_eq!(
            absolute_path("tmp/b.txt", &config),
            Some(PathBuf::from("/tmp/b.txt"))
        );
        // A path which is not one of the files being diffed is still resolved relative to the cwd.
        assert_eq!(
            absolute_path("tmp/c.txt", &config),
            Some(fake_delta_cwd_for_tests().join("tmp/c.txt"))
        );
    }

    #[test]
    fn test_absolute_path_of_repo_relative_path() {
        // When delta is invoked by git in a repo, paths are relative to the repo root, which is
        // delta's cwd, so they must be joined to it.
        let config = make_config_from_args(&[]);
        assert_eq!(
            absolute_path("src/main.rs", &config),
            Some(fake_delta_cwd_for_tests().join("src/main.rs"))
        );
    }
}
