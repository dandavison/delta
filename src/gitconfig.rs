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
         $opt:expr, $git_config:expr, $arg_matches:expr) => {
            $(
                if $arg_matches.is_none() || !$crate::cli::user_supplied_option($opt_name, $arg_matches.unwrap()) {
                    $opt.$field_ident =
                        $crate::gitconfig::git_config_get::_string($keys, $git_config)
                        .unwrap_or_else(|| $default.to_string());
                };
            )*
        };
    }

    macro_rules! set_options__option_string {
        ([$( ($opt_name:expr, $field_ident:ident, $keys:expr, $default:expr) ),* ],
         $opt:expr, $git_config:expr, $arg_matches:expr) => {
            $(
                if $arg_matches.is_none() || !$crate::cli::user_supplied_option($opt_name, $arg_matches.unwrap()) {
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
         $opt:expr, $git_config:expr, $arg_matches:expr) => {
            $(
                if $arg_matches.is_none() || !$crate::cli::user_supplied_option($opt_name, $arg_matches.unwrap()) {
                    $opt.$field_ident =
                        $crate::gitconfig::git_config_get::_bool($keys, $git_config)
                        .unwrap_or_else(|| $default);
                };
            )*
        };
    }

    macro_rules! set_options__f64 {
        ([$( ($opt_name:expr, $field_ident:ident, $keys:expr, $default:expr) ),* ],
         $opt:expr, $git_config:expr, $arg_matches:expr) => {
            $(
                if $arg_matches.is_none() || !$crate::cli::user_supplied_option($opt_name, $arg_matches.unwrap()) {
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
         $opt:expr, $git_config:expr, $arg_matches:expr) => {
            $(
                if $arg_matches.is_none() || !$crate::cli::user_supplied_option($opt_name, $arg_matches.unwrap()) {
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
         $opt:expr, $git_config:expr, $arg_matches:expr) => {
		    set_options__string!([
                $(
                    ($option_name,
                     $field_ident,
                     $crate::gitconfig::make_git_config_keys_for_delta($option_name, $opt.preset.as_deref()),
                     &$opt.$field_ident)
                ),*
            ],
            $opt,
            $git_config,
            $arg_matches);
	    };
    }

    macro_rules! set_delta_options__option_string {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $git_config:expr, $arg_matches:expr) => {
		    set_options__option_string!([
                $(
                    ($option_name,
                     $field_ident,
                     $crate::gitconfig::make_git_config_keys_for_delta($option_name, $opt.preset.as_deref()),
                     $opt.$field_ident.as_deref())
                ),*
            ],
            $opt,
            $git_config,
            $arg_matches);
	    };
    }

    macro_rules! set_delta_options__bool {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $git_config:expr, $arg_matches:expr) => {
		    set_options__bool!([
                $(
                    ($option_name,
                     $field_ident,
                     $crate::gitconfig::make_git_config_keys_for_delta($option_name, $opt.preset.as_deref()),
                     $opt.$field_ident)
                ),*
            ],
            $opt,
            $git_config,
            $arg_matches);
	    };
    }

    macro_rules! set_delta_options__f64 {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $git_config:expr, $arg_matches:expr) => {
		    set_options__f64!([
                $(
                    ($option_name,
                     $field_ident,
                     $crate::gitconfig::make_git_config_keys_for_delta($option_name, $opt.preset.as_deref()),
                     $opt.$field_ident)
                ),*
            ],
            $opt,
            $git_config,
            $arg_matches);
	    };
    }

    macro_rules! set_delta_options__usize {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $git_config:expr, $arg_matches:expr) => {
		    set_options__usize!([
                $(
                    ($option_name,
                     $field_ident,
                     $crate::gitconfig::make_git_config_keys_for_delta($option_name, $opt.preset.as_deref()),
                     $opt.$field_ident)
                ),*
            ],
            $opt,
            $git_config,
            $arg_matches);
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
