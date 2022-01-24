use std::path::{Component, Path, PathBuf};

use crate::config::Config;

use super::process::calling_process;

// Infer absolute path to `relative_path`.
pub fn absolute_path(relative_path: &str, config: &Config) -> Option<PathBuf> {
    match (
        &config.cwd_of_delta_process,
        &config.cwd_of_user_shell_process,
        calling_process().paths_in_input_are_relative_to_cwd() || config.relative_paths,
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

/// Relativize path if delta config demands that and paths are not already relativized by git.
pub fn relativize_path_maybe(path: &str, config: &Config) -> Option<PathBuf> {
    if config.relative_paths && !calling_process().paths_in_input_are_relative_to_cwd() {
        if let Some(base) = config.cwd_relative_to_repo_root.as_deref() {
            pathdiff::diff_paths(&path, base)
        } else {
            None
        }
    } else {
        None
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
