// TODO: Add tests and parameterize over types more cleanly.

#[macro_use]
mod set_options {
    /// If `opt_name` was not supplied on the command line, then change its value to one of the
    /// following in order of precedence:
    /// 1. The entry for it in the section of gitconfig corresponding to the selected preset, if there is
    ///    one.
    /// 2. The entry for it in the main delta section of gitconfig, if there is one.
    /// 3. The default value passed to this macro (which may be the current value).

    macro_rules! set_options__string {
        ([$( ($opt_name:expr, $field_ident:ident, $keys:expr, $default:expr) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
            $(
                if !$crate::cli::user_supplied_option($opt_name, $arg_matches) {
                    $opt.$field_ident =
                        $crate::gitconfig::git_config_get::_string($keys, $git_config)
                        .unwrap_or_else(|| $default.to_string());
                };
            )*
        };
    }

    macro_rules! set_options__option_string {
        ([$( ($opt_name:expr, $field_ident:ident, $keys:expr, $default:expr) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
            $(
                if !$crate::cli::user_supplied_option($opt_name, $arg_matches) {
                    $opt.$field_ident = match ($crate::gitconfig::git_config_get::_string($keys, $git_config), $default) {
                        (Some(s), _) => Some(s),
                        (None, Some(default)) => Some(default.to_string()),
                        (None, None) => None,
                    }
                };
            )*
        };
    }

    macro_rules! set_options__bool {
        ([$( ($opt_name:expr, $field_ident:ident, $keys:expr, $default:expr) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
            $(
                if !$crate::cli::user_supplied_option($opt_name, $arg_matches) {
                    $opt.$field_ident =
                        $crate::gitconfig::git_config_get::_bool($keys, $git_config)
                        .unwrap_or_else(|| $default);
                };
            )*
        };
    }

    macro_rules! set_options__f64 {
        ([$( ($opt_name:expr, $field_ident:ident, $keys:expr, $default:expr) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
            $(
                if !$crate::cli::user_supplied_option($opt_name, $arg_matches) {
                    $opt.$field_ident = match $crate::gitconfig::git_config_get::_string($keys, $git_config) {
                        Some(s) => s.parse::<f64>().unwrap_or($default),
                        None => $default,
                    }
                };
            )*
        };
    }

    macro_rules! set_options__usize {
        ([$( ($opt_name:expr, $field_ident:ident, $keys:expr, $default:expr) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
            $(
                if !$crate::cli::user_supplied_option($opt_name, $arg_matches) {
                    $opt.$field_ident = match $crate::gitconfig::git_config_get::_i64($keys, $git_config) {
                        Some(int) => int as usize,
                        None => $default,
                    }
                };
            )*
        };
    }
}

#[macro_use]
mod set_delta_options {
    // set_delta_options<T> implementations

    macro_rules! set_delta_options__string {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
		    set_options__string!([
                $(
                    ($option_name,
                     $field_ident,
                     $crate::gitconfig::make_git_config_keys_for_delta($option_name, $opt.preset.as_deref()),
                     &$opt.$field_ident)
                ),*
            ],
            $opt,
            $arg_matches,
            $git_config);
	    };
    }

    macro_rules! set_delta_options__option_string {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
		    set_options__option_string!([
                $(
                    ($option_name,
                     $field_ident,
                     $crate::gitconfig::make_git_config_keys_for_delta($option_name, $opt.preset.as_deref()),
                     $opt.$field_ident.as_deref())
                ),*
            ],
            $opt,
            $arg_matches,
            $git_config);
	    };
    }

    macro_rules! set_delta_options__bool {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
		    set_options__bool!([
                $(
                    ($option_name,
                     $field_ident,
                     $crate::gitconfig::make_git_config_keys_for_delta($option_name, $opt.preset.as_deref()),
                     $opt.$field_ident)
                ),*
            ],
            $opt,
            $arg_matches,
            $git_config);
	    };
    }

    macro_rules! set_delta_options__f64 {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
		    set_options__f64!([
                $(
                    ($option_name,
                     $field_ident,
                     $crate::gitconfig::make_git_config_keys_for_delta($option_name, $opt.preset.as_deref()),
                     $opt.$field_ident)
                ),*
            ],
            $opt,
            $arg_matches,
            $git_config);
	    };
    }

    macro_rules! set_delta_options__usize {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
		    set_options__usize!([
                $(
                    ($option_name,
                     $field_ident,
                     $crate::gitconfig::make_git_config_keys_for_delta($option_name, $opt.preset.as_deref()),
                     $opt.$field_ident)
                ),*
            ],
            $opt,
            $arg_matches,
            $git_config);
	    };
    }
}

pub mod git_config_get {
    use git2;

    /// Get String value from gitconfig
    pub fn _string(keys: Vec<String>, git_config: &mut Option<git2::Config>) -> Option<String> {
        match git_config {
            Some(git_config) => {
                let git_config = git_config.snapshot().unwrap();
                for key in keys {
                    let entry = git_config.get_str(&key);
                    if let Ok(entry) = entry {
                        return Some(entry.to_string());
                    }
                }
                return None;
            }
            None => None,
        }
    }

    /// Get bool value from gitconfig
    pub fn _bool(keys: Vec<String>, git_config: &mut Option<git2::Config>) -> Option<bool> {
        match git_config {
            Some(git_config) => {
                let git_config = git_config.snapshot().unwrap();
                for key in keys {
                    let entry = git_config.get_bool(&key);
                    if let Ok(entry) = entry {
                        return Some(entry);
                    }
                }
                return None;
            }
            None => None,
        }
    }

    /// Get i64 value from gitconfig
    pub fn _i64(keys: Vec<String>, git_config: &mut Option<git2::Config>) -> Option<i64> {
        match git_config {
            Some(git_config) => {
                let git_config = git_config.snapshot().unwrap();
                for key in keys {
                    let entry = git_config.get_i64(&key);
                    if let Ok(entry) = entry {
                        return Some(entry);
                    }
                }
                return None;
            }
            None => None,
        }
    }
}

pub fn make_git_config_keys_for_delta(key: &str, preset: Option<&str>) -> Vec<String> {
    match preset {
        Some(preset) => vec![
            format!("delta.{}.{}", preset, key),
            format!("delta.{}", key),
        ],
        None => vec![format!("delta.{}", key)],
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{remove_file, File};
    use std::io::Write;
    use std::path::Path;

    use git2;
    use structopt::StructOpt;

    use crate::cli;
    use crate::config;
    use crate::style::Style;

    const TEST_GIT_CONFIG_FILE: &str = "/tmp/delta-test-gitconfig";

    #[test]
    fn test_delta_main_section_is_honored() {
        // First check that it doesn't default to blue, because that's going to be used to signal
        // that gitconfig has set the style.
        let config = make_config(
            &[
                "delta",
                "/dev/null",
                "/dev/null",
                "--24-bit-color",
                "always",
            ],
            None,
        );
        assert_ne!(config.minus_style, make_style("blue"));

        // Check that --minus-style is honored as we expect.
        let config = make_config(
            &[
                "/dev/null",
                "/dev/null",
                "--24-bit-color",
                "always",
                "--minus-style",
                "red",
            ],
            None,
        );
        assert_eq!(config.minus_style, make_style("red"));

        // Check that gitconfig does not override a command line argument
        let config = make_config(
            &[
                "/dev/null",
                "/dev/null",
                "--24-bit-color",
                "always",
                "--minus-style",
                "red",
            ],
            Some(
                b"
[delta]
    minus-style = blue
",
            ),
        );
        assert_eq!(config.minus_style, make_style("red"));

        // Finally, check that gitconfig is honored when not overridden by a command line argument.
        let config = make_config(
            &["/dev/null", "/dev/null", "--24-bit-color", "always"],
            Some(
                b"
[delta]
    minus-style = blue
",
            ),
        );
        assert_eq!(config.minus_style, make_style("blue"));

        remove_file(TEST_GIT_CONFIG_FILE).unwrap();
    }

    fn make_style(s: &str) -> Style {
        Style::from_str(s, None, None, None, true, false)
    }

    fn make_git_config(contents: &[u8]) -> git2::Config {
        let path = Path::new(TEST_GIT_CONFIG_FILE);
        let mut file = File::create(path).unwrap();
        file.write_all(contents).unwrap();
        git2::Config::open(&path).unwrap()
    }

    fn make_config<'a>(args: &[&str], git_config_contents: Option<&[u8]>) -> config::Config<'a> {
        let mut git_config = match git_config_contents {
            Some(contents) => Some(make_git_config(contents)),
            None => None,
        };
        cli::process_command_line_arguments(
            cli::Opt::clap().get_matches_from(args),
            &mut git_config,
        )
    }
}
