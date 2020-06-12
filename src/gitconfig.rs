use std::collections::HashMap;
use std::process;

use crate::cli;
use crate::preset::{self, GetValueFunctionFromBuiltinPreset};

// A type T implementing this trait gains a static method allowing an option value of type T to be
// sought, obeying delta's standard rules for looking up option values. It is implemented for T in
// {String, bool, i64}.
pub trait GetOptionValue {
    // If the value for option name n was not supplied on the command line, then a search is performed
    // as follows. The first value encountered is used:
    //
    // 1. For each preset p (moving right to left through the listed presets):
    //    1.1 The value of n under p interpreted as a user-supplied preset (i.e. git config value
    //        delta.$p.$n)
    //    1.2 The value for n under p interpreted as a builtin preset
    // 3. The value for n in the main git config section for delta (i.e. git config value delta.$n)
    fn get_option_value(
        option_name: &str,
        builtin_presets: &HashMap<String, preset::BuiltinPreset<String>>,
        opt: &cli::Opt,
        git_config: &mut Option<git2::Config>,
    ) -> Option<Self>
    where
        Self: Sized,
        Self: GitConfigGet,
        Self: GetValueFunctionFromBuiltinPreset,
    {
        if let Some(presets) = &opt.presets {
            for preset in presets.to_lowercase().split_whitespace().rev() {
                if let Some(value) = Self::get_option_value_for_preset(
                    option_name,
                    &preset,
                    &builtin_presets,
                    opt,
                    git_config,
                ) {
                    return Some(value);
                }
            }
        }
        if let Some(git_config) = git_config {
            let git_config = git_config.snapshot().unwrap_or_else(|err| {
                eprintln!("Failed to read git config: {}", err);
                process::exit(1)
            });
            if let Some(value) =
                git_config_get::<Self>(&format!("delta.{}", option_name), git_config)
            {
                return Some(value);
            }
        }
        None
    }

    fn get_option_value_for_preset(
        option_name: &str,
        preset: &str,
        builtin_presets: &HashMap<String, preset::BuiltinPreset<String>>,
        opt: &cli::Opt,
        git_config: &mut Option<git2::Config>,
    ) -> Option<Self>
    where
        Self: Sized,
        Self: GitConfigGet,
        Self: GetValueFunctionFromBuiltinPreset,
    {
        if let Some(git_config) = git_config {
            let git_config = git_config.snapshot().unwrap_or_else(|err| {
                eprintln!("Failed to read git config: {}", err);
                process::exit(1)
            });
            if let Some(value) =
                git_config_get::<Self>(&format!("delta.{}.{}", preset, option_name), git_config)
            {
                return Some(value);
            }
        }
        if let Some(builtin_preset) = builtin_presets.get(preset) {
            if let Some(value_function) =
                Self::get_value_function_from_builtin_preset(option_name, builtin_preset)
            {
                return Some(value_function(opt, &git_config));
            }
        }
        return None;
    }
}

impl GetOptionValue for String {}
impl GetOptionValue for bool {}
impl GetOptionValue for i64 {}

pub trait GitConfigGet {
    fn git_config_get(key: &str, git_config: &git2::Config) -> Option<Self>
    where
        Self: Sized;
}

impl GitConfigGet for String {
    fn git_config_get(key: &str, git_config: &git2::Config) -> Option<Self> {
        git_config.get_string(key).ok()
    }
}

impl GitConfigGet for bool {
    fn git_config_get(key: &str, git_config: &git2::Config) -> Option<Self> {
        git_config.get_bool(key).ok()
    }
}

impl GitConfigGet for i64 {
    fn git_config_get(key: &str, git_config: &git2::Config) -> Option<Self> {
        git_config.get_i64(key).ok()
    }
}

fn git_config_get<T>(key: &str, git_config: git2::Config) -> Option<T>
where
    T: GitConfigGet,
{
    T::git_config_get(key, &git_config)
}

#[macro_use]
mod set_options {
    // set_options<T> implementations

    macro_rules! set_options__string {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
            let builtin_presets = $crate::preset::make_builtin_presets(); // TODO: move up the stack
            $(
                 if !$crate::config::user_supplied_option($option_name, $arg_matches) {
                    if let Some(value) = String::get_option_value($option_name, &builtin_presets, $opt, $git_config) {
                        $opt.$field_ident = value;
                    }
                };
            )*
	    };
    }

    macro_rules! set_options__option_string {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
            let builtin_presets = $crate::preset::make_builtin_presets(); // TODO: move up the stack
            $(
                 if !$crate::config::user_supplied_option($option_name, $arg_matches) {
                    if let Some(value) = String::get_option_value($option_name, &builtin_presets, $opt, $git_config) {
                        $opt.$field_ident = Some(value);
                    }
                };
            )*
	    };
    }

    macro_rules! set_options__bool {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
            let builtin_presets = $crate::preset::make_builtin_presets(); // TODO: move up the stack
            $(
                 if !$crate::config::user_supplied_option($option_name, $arg_matches) {
                    if let Some(value) = bool::get_option_value($option_name, &builtin_presets, $opt, $git_config) {
                        $opt.$field_ident = value;
                    }
                };
            )*
	    };
    }

    macro_rules! set_options__f64 {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
            let builtin_presets = $crate::preset::make_builtin_presets(); // TODO: move up the stack
            $(
                 if !$crate::config::user_supplied_option($option_name, $arg_matches) {
                    if let Some(value) = String::get_option_value($option_name, &builtin_presets, $opt, $git_config) {
                        if let Some(value) = value.parse::<f64>().ok(){
                            $opt.$field_ident = value;
                        }
                    }
                };
            )*
	    };
    }

    macro_rules! set_options__usize {
	    ([$( ($option_name:expr, $field_ident:ident) ),* ],
         $opt:expr, $arg_matches:expr, $git_config:expr) => {
            let builtin_presets = $crate::preset::make_builtin_presets(); // TODO: move up the stack
            $(
                 if !$crate::config::user_supplied_option($option_name, $arg_matches) {
                    if let Some(value) = i64::get_option_value($option_name, &builtin_presets, $opt, $git_config) {
                        $opt.$field_ident = value as usize;
                    }
                };
            )*
	    };
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{remove_file, File};
    use std::io::Write;
    use std::path::Path;

    use git2;
    use itertools;

    use crate::config;
    use crate::style::{DecorationStyle, Style};

    #[test]
    fn test_main_section() {
        let git_config_contents = b"
[delta]
    minus-style = blue
";
        let git_config_path = "delta__test_main_section.gitconfig";

        // First check that it doesn't default to blue, because that's going to be used to signal
        // that gitconfig has set the style.
        assert_ne!(make_config(&[], None, None).minus_style, make_style("blue"));

        // Check that --minus-style is honored as we expect.
        assert_eq!(
            make_config(&["--minus-style", "red"], None, None).minus_style,
            make_style("red")
        );

        // Check that gitconfig does not override a command line argument
        assert_eq!(
            make_config(
                &["--minus-style", "red"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("red")
        );

        // Finally, check that gitconfig is honored when not overridden by a command line argument.
        assert_eq!(
            make_config(&[], Some(git_config_contents), Some(git_config_path)).minus_style,
            make_style("blue")
        );

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_preset() {
        let git_config_contents = b"
[delta]
    minus-style = blue

[delta \"my-preset\"]
    minus-style = green
";
        let git_config_path = "delta__test_preset.gitconfig";

        // Without --presets the main section takes effect
        assert_eq!(
            make_config(&[], Some(git_config_contents), Some(git_config_path)).minus_style,
            make_style("blue")
        );

        // With --presets the preset takes effect
        assert_eq!(
            make_config(
                &["--presets", "my-preset"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("green")
        );
        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_multiple_presets() {
        let git_config_contents = b"
[delta]
    minus-style = blue

[delta \"my-preset-1\"]
    minus-style = green

[delta \"my-preset-2\"]
    minus-style = yellow
";
        let git_config_path = "delta__test_multiple_presets.gitconfig";

        assert_eq!(
            make_config(
                &["--presets", "my-preset-1"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("green")
        );

        assert_eq!(
            make_config(
                &["--presets", "my-preset-1 my-preset-2"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("yellow")
        );

        assert_eq!(
            make_config(
                &["--presets", "my-preset-2 my-preset-1"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("green")
        );

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_invalid_presets() {
        let git_config_contents = b"
[delta]
    minus-style = blue

[delta \"my-preset-1\"]
    minus-style = green

[delta \"my-preset-2\"]
    minus-style = yellow
";
        let git_config_path = "delta__test_invalid_presets.gitconfig";

        assert_eq!(
            make_config(
                &["--presets", "my-preset-1"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("green")
        );

        assert_eq!(
            make_config(
                &["--presets", "my-preset-x"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("blue")
        );

        assert_eq!(
            make_config(
                &["--presets", "my-preset-1 my-preset-x"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("green")
        );

        assert_eq!(
            make_config(
                &["--presets", "my-preset-x my-preset-2 my-preset-x"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("yellow")
        );

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_diff_highlight_defaults() {
        let config = make_config(&["--presets", "diff-highlight"], None, None);

        assert_eq!(config.minus_style, make_style("red"));
        assert_eq!(config.minus_non_emph_style, make_style("red"));
        assert_eq!(config.minus_emph_style, make_emph_style("red reverse"));
        assert_eq!(config.zero_style, make_style(""));
        assert_eq!(config.plus_style, make_style("green"));
        assert_eq!(config.plus_non_emph_style, make_style("green"));
        assert_eq!(config.plus_emph_style, make_emph_style("green reverse"));
    }

    #[test]
    fn test_diff_highlight_respects_gitconfig() {
        let git_config_contents = b"
[color \"diff\"]
    old = red bold
    new = green bold

[color \"diff-highlight\"]
    oldNormal = ul red bold
    oldHighlight = red bold 52
    newNormal = ul green bold
    newHighlight = green bold 22
";
        let git_config_path = "delta__test_diff_highlight.gitconfig";

        let config = make_config(
            &["--presets", "diff-highlight"],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(config.minus_style, make_style("red bold"));
        assert_eq!(config.minus_non_emph_style, make_style("ul red bold"));
        assert_eq!(config.minus_emph_style, make_emph_style("red bold 52"));
        assert_eq!(config.zero_style, make_style(""));
        assert_eq!(config.plus_style, make_style("green bold"));
        assert_eq!(config.plus_non_emph_style, make_style("ul green bold"));
        assert_eq!(config.plus_emph_style, make_emph_style("green bold 22"));

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_diff_so_fancy_defaults() {
        let config = make_config(&["--presets", "diff-so-fancy"], None, None);

        assert_eq!(
            config.commit_style.ansi_term_style,
            make_style("bold yellow").ansi_term_style
        );
        assert_eq!(
            config.commit_style.decoration_style,
            make_decoration_style("none")
        );

        assert_eq!(
            config.file_style.ansi_term_style,
            make_style("11").ansi_term_style
        );
        assert_eq!(
            config.file_style.decoration_style,
            make_decoration_style("bold yellow ul ol")
        );

        assert_eq!(
            config.hunk_header_style.ansi_term_style,
            make_style("bold syntax").ansi_term_style
        );
        assert_eq!(
            config.hunk_header_style.decoration_style,
            make_decoration_style("magenta box")
        );
    }

    #[test]
    fn test_diff_so_fancy_respects_git_config() {
        let git_config_contents = b"
[color \"diff\"]
    meta = 11
    frag = magenta bold
    commit = yellow bold
    old = red bold
    new = green bold
    whitespace = red reverse
";
        let git_config_path = "delta__test_diff_so_fancy.gitconfig";

        let config = make_config(
            &["--presets", "diff-so-fancy some-other-preset"],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(
            config.commit_style.ansi_term_style,
            make_style("yellow bold").ansi_term_style
        );
        assert_eq!(
            config.file_style.ansi_term_style,
            make_style("11").ansi_term_style
        );
        assert_eq!(
            config.hunk_header_style.ansi_term_style,
            make_style("magenta bold").ansi_term_style
        );
        assert_eq!(
            config.commit_style.decoration_style,
            make_decoration_style("none")
        );
        assert_eq!(
            config.file_style.decoration_style,
            make_decoration_style("yellow bold ul ol")
        );
        assert_eq!(
            config.hunk_header_style.decoration_style,
            make_decoration_style("magenta box")
        );

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_diff_so_fancy_obeys_preset_precedence_rules() {
        let git_config_contents = b"
[color \"diff\"]
    meta = 11
    frag = magenta bold
    commit = yellow bold
    old = red bold
    new = green bold
    whitespace = red reverse

[delta \"decorations\"]
    commit-decoration-style = bold box ul
    file-style = bold 19 ul
    file-decoration-style = none
";
        let git_config_path = "delta__test_diff_so_fancy_obeys_preset_precedence_rules.gitconfig";

        let config = make_config(
            &["--presets", "decorations diff-so-fancy"],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(
            config.file_style.ansi_term_style,
            make_style("11").ansi_term_style
        );

        assert_eq!(
            config.file_style.decoration_style,
            make_decoration_style("yellow bold ul ol")
        );

        let config = make_config(
            &["--presets", "diff-so-fancy decorations"],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(
            config.file_style.ansi_term_style,
            make_style("ul bold 19").ansi_term_style
        );

        assert_eq!(
            config.file_style.decoration_style,
            make_decoration_style("none")
        );

        remove_file(git_config_path).unwrap();
    }

    fn make_style(s: &str) -> Style {
        _make_style(s, false)
    }

    fn make_emph_style(s: &str) -> Style {
        _make_style(s, true)
    }

    fn _make_style(s: &str, is_emph: bool) -> Style {
        Style::from_str(s, None, None, None, true, is_emph)
    }

    fn make_decoration_style(s: &str) -> DecorationStyle {
        DecorationStyle::from_str(s, true)
    }

    fn make_git_config(contents: &[u8], path: &str) -> git2::Config {
        let path = Path::new(path);
        let mut file = File::create(path).unwrap();
        file.write_all(contents).unwrap();
        git2::Config::open(&path).unwrap()
    }

    fn make_config<'a>(
        args: &[&str],
        git_config_contents: Option<&[u8]>,
        path: Option<&str>,
    ) -> config::Config<'a> {
        let args: Vec<&str> = itertools::chain(
            &["/dev/null", "/dev/null", "--24-bit-color", "always"],
            args,
        )
        .map(|s| *s)
        .collect();
        let mut git_config = match (git_config_contents, path) {
            (Some(contents), Some(path)) => Some(make_git_config(contents, path)),
            _ => None,
        };
        config::Config::from_args(&args, &mut git_config)
    }
}
