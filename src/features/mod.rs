use std::collections::HashMap;

use crate::cli;
use crate::git_config::GitConfig;
use crate::option_value::ProvenancedOptionValue;
use ProvenancedOptionValue::*;

/// A custom feature is a named set of command line (option, value) pairs, supplied in a git config
/// file. I.e. it might look like
///
/// [delta "decorations"]
///     commit-decoration-style = bold box ul
///     file-style = bold 19 ul
///     file-decoration-style = none
///
/// A builtin feature is a named set of command line (option, value) pairs that is built-in to
/// delta. The valueof a builtin feature is a function. This function is passed the current set of
/// all command-line option-value pairs, and GitConfig, and returns either a GitConfigValue, or a
/// DefaultValue. (It may use the set of all option-value pairs when computing its default).
pub type BuiltinFeature = HashMap<String, OptionValueFunction>;

type OptionValueFunction = Box<dyn Fn(&cli::Opt, &Option<GitConfig>) -> ProvenancedOptionValue>;

// Construct a 2-level hash map: (feature name) -> (option name) -> (value function). A value
// function is a function that takes an Opt struct, and a git Config struct, and returns the value
// for the option.
pub fn make_builtin_features() -> HashMap<String, BuiltinFeature> {
    vec![
        (
            "diff-highlight".to_string(),
            diff_highlight::make_feature().into_iter().collect(),
        ),
        (
            "diff-so-fancy".to_string(),
            diff_so_fancy::make_feature().into_iter().collect(),
        ),
        (
            "navigate".to_string(),
            navigate::make_feature().into_iter().collect(),
        ),
    ]
    .into_iter()
    .collect()
}

/// The macro permits the values of a builtin feature to be specified as either (a) a git config
/// entry or (b) a value, which may be computed from the other command line options (cli::Opt).
macro_rules! builtin_feature {
    ([$( ($option_name:expr, $type:ty, $git_config_key:expr, $opt:ident => $value:expr) ),*]) => {
        vec![$(
            (
                $option_name.to_string(),
                Box::new(move |$opt: &$crate::cli::Opt, git_config: &Option<$crate::git_config::GitConfig>| {
                    match (git_config, $git_config_key, $opt.no_gitconfig) {
                        (Some(git_config), Some(git_config_key), false) => match git_config.get::<$type>(git_config_key) {
                            Some(value) => Some($crate::features::GitConfigValue(value.into())),
                            _ => None,
                        },
                        _ => None,
                    }
                    .unwrap_or_else(|| $crate::features::DefaultValue($value.into()))
                }) as OptionValueFunction
            )
        ),*]
    }
}

pub mod diff_highlight;
pub mod diff_so_fancy;
pub mod navigate;

#[cfg(test)]
pub mod tests {
    use std::fs::{remove_file, File};
    use std::io::Write;
    use std::path::Path;

    use itertools;

    use crate::config;
    use crate::git_config::GitConfig;
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
    fn test_feature() {
        let git_config_contents = b"
[delta]


[delta \"my-feature\"]
    minus-style = green
";
        let git_config_path = "delta__test_feature.gitconfig";

        assert_eq!(
            make_config(
                &["--features", "my-feature"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("green")
        );
        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_main_section_overrides_feature() {
        let git_config_contents = b"
[delta]
    minus-style = blue

[delta \"my-feature-1\"]
    minus-style = green
";
        let git_config_path = "delta__test_main_section_overrides_feature.gitconfig";

        // Without --features the main section takes effect
        assert_eq!(
            make_config(&[], Some(git_config_contents), Some(git_config_path)).minus_style,
            make_style("blue")
        );

        // Event with --features the main section overrides the feature.
        assert_eq!(
            make_config(
                &["--features", "my-feature-1"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("blue")
        );
        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_multiple_features() {
        let git_config_contents = b"
[delta]


[delta \"my-feature-1\"]
    minus-style = green

[delta \"my-feature-2\"]
    minus-style = yellow
";
        let git_config_path = "delta__test_multiple_features.gitconfig";

        assert_eq!(
            make_config(
                &["--features", "my-feature-1 my-feature-2"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("yellow")
        );

        assert_eq!(
            make_config(
                &["--features", "my-feature-2 my-feature-1"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("green")
        );

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_invalid_features() {
        let git_config_contents = b"
[delta \"my-feature-1\"]
    minus-style = green

[delta \"my-feature-2\"]
    minus-style = yellow
";
        let git_config_path = "delta__test_invalid_features.gitconfig";

        let default = make_config(&[], None, None).minus_style;
        assert_ne!(default, make_style("green"));
        assert_ne!(default, make_style("yellow"));

        assert_eq!(
            make_config(
                &["--features", "my-feature-1"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("green")
        );

        assert_eq!(
            make_config(
                &["--features", "my-feature-x"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            default
        );

        assert_eq!(
            make_config(
                &["--features", "my-feature-1 my-feature-x"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("green")
        );

        assert_eq!(
            make_config(
                &["--features", "my-feature-x my-feature-2 my-feature-x"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("yellow")
        );

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_whitespace_error_style() {
        let git_config_contents = b"
[color \"diff\"]
    whitespace = yellow dim ul magenta
";
        let git_config_path = "delta__test_whitespace_error_style.gitconfig";

        // Git config disabled: hard-coded delta default wins
        assert_eq!(
            make_config(&[], None, None).whitespace_error_style,
            make_style("magenta reverse")
        );

        // Unspecified by user: color.diff.whitespace wins
        assert_eq!(
            make_config(&[], Some(git_config_contents), Some(git_config_path))
                .whitespace_error_style,
            make_style("yellow dim ul magenta")
        );

        // Command line argument wins
        assert_eq!(
            make_config(
                &["--whitespace-error-style", "red reverse"],
                Some(git_config_contents),
                Some(git_config_path)
            )
            .whitespace_error_style,
            make_style("reverse red")
        );

        let git_config_contents = b"
[color \"diff\"]
    whitespace = yellow dim ul magenta

[delta]
    whitespace-error-style = blue reverse

[delta \"my-whitespace-error-style-feature\"]
    whitespace-error-style = green reverse
";

        // Command line argument wins
        assert_eq!(
            make_config(
                &["--whitespace-error-style", "red reverse"],
                Some(git_config_contents),
                Some(git_config_path)
            )
            .whitespace_error_style,
            make_style("reverse red")
        );

        // No command line argument or features; main [delta] section wins
        assert_eq!(
            make_config(&[], Some(git_config_contents), Some(git_config_path))
                .whitespace_error_style,
            make_style("blue reverse")
        );

        // Feature contains key, but main [delta] section still wins.
        // This is equivalent to
        //
        // [delta]
        //     features = my-whitespace-error-style-feature
        //     whitespace-error-style = blue reverse
        //
        // In this situation, the value from the feature is overridden.
        assert_eq!(
            make_config(
                &["--features", "my-whitespace-error-style-feature"],
                Some(git_config_contents),
                Some(git_config_path)
            )
            .whitespace_error_style,
            make_style("reverse blue")
        );

        remove_file(git_config_path).unwrap();
    }

    pub fn make_style(s: &str) -> Style {
        _make_style(s, false)
    }

    pub fn make_emph_style(s: &str) -> Style {
        _make_style(s, true)
    }

    fn _make_style(s: &str, is_emph: bool) -> Style {
        Style::from_str(s, None, None, None, true, is_emph)
    }

    pub fn make_decoration_style(s: &str) -> DecorationStyle {
        DecorationStyle::from_str(s, true)
    }

    fn make_git_config(contents: &[u8], path: &str) -> GitConfig {
        let path = Path::new(path);
        let mut file = File::create(path).unwrap();
        file.write_all(contents).unwrap();
        GitConfig::from_path(&path)
    }

    pub fn make_config(
        args: &[&str],
        git_config_contents: Option<&[u8]>,
        path: Option<&str>,
    ) -> config::Config {
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
