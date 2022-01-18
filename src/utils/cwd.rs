use std::path::PathBuf;

use crate::config::Config;

/// Return current working directory of the user's shell process. I.e. the directory which they are
/// in when delta exits. This is the directory relative to which the file paths in delta output are
/// constructed if they are using either (a) delta's relative-paths option or (b) git's --relative
/// flag.
pub fn cwd_of_user_shell_process(config: &Config) -> Option<PathBuf> {
    match (&config.cwd, &config.cwd_relative_to_repo_root) {
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
