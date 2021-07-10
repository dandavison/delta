use std::collections::HashMap;

use crate::cli;
use crate::git_config::GitConfig;
use crate::options::option_value::ProvenancedOptionValue;
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
            "color-only".to_string(),
            color_only::make_feature().into_iter().collect(),
        ),
        (
            "diff-highlight".to_string(),
            diff_highlight::make_feature().into_iter().collect(),
        ),
        (
            "diff-so-fancy".to_string(),
            diff_so_fancy::make_feature().into_iter().collect(),
        ),
        (
            "hyperlinks".to_string(),
            hyperlinks::make_feature().into_iter().collect(),
        ),
        (
            "line-numbers".to_string(),
            line_numbers::make_feature().into_iter().collect(),
        ),
        (
            "navigate".to_string(),
            navigate::make_feature().into_iter().collect(),
        ),
        ("raw".to_string(), raw::make_feature().into_iter().collect()),
        (
            "side-by-side".to_string(),
            side_by_side::make_feature().into_iter().collect(),
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
                    match (git_config, $git_config_key) {
                        (Some(git_config), Some(git_config_key)) => git_config.get::<$type>(git_config_key).map(|value| $crate::features::GitConfigValue(value.into())),
                        _ => None,
                    }
                    .unwrap_or_else(|| $crate::features::DefaultValue($value.into()))
                }) as OptionValueFunction
            )
        ),*]
    }
}

pub mod color_only;
pub mod diff_highlight;
pub mod diff_so_fancy;
pub mod hyperlinks;
pub mod line_numbers;
pub mod navigate;
pub mod raw;
pub mod side_by_side;

#[cfg(test)]
pub mod tests {
    use std::collections::HashSet;
    use std::fs::remove_file;

    use crate::cli;
    use crate::features::make_builtin_features;
    use crate::tests::integration_test_utils::make_options_from_args_and_git_config;

    #[test]
    fn test_builtin_features_have_flags_and_these_set_features() {
        let builtin_features = make_builtin_features();
        let mut args = vec!["delta".to_string()];
        args.extend(builtin_features.keys().map(|s| format!("--{}", s)));
        let opt = cli::Opt::from_iter_and_git_config(args, &mut None);
        let features: HashSet<&str> = opt.features.split_whitespace().collect();
        for feature in builtin_features.keys() {
            assert!(features.contains(feature.as_str()))
        }
    }

    #[test]
    fn test_builtin_feature_from_gitconfig() {
        let git_config_contents = b"
[delta]
    navigate = true
";
        let git_config_path = "delta__test_builtin_feature_from_gitconfig.gitconfig";

        assert_eq!(
            make_options_from_args_and_git_config(
                &[],
                Some(git_config_contents),
                Some(git_config_path)
            )
            .features,
            "navigate"
        );

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_features_on_command_line_replace_features_in_gitconfig() {
        let git_config_contents = b"
[delta]
    features = my-feature
";
        let git_config_path =
            "delta__test_features_on_command_line_replace_features_in_gitconfig.gitconfig";

        assert_eq!(
            make_options_from_args_and_git_config(
                &["--features", "navigate raw"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .features,
            "navigate raw"
        );
        assert_eq!(
            make_options_from_args_and_git_config(
                &["--navigate", "--features", "raw"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .features,
            "navigate raw"
        );

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_feature_flag_on_command_line_does_not_replace_features_in_gitconfig() {
        let git_config_contents = b"
[delta]
    features = my-feature
";
        let git_config_path =
            "delta__test_feature_flag_on_command_line_does_not_replace_features_in_gitconfig.gitconfig";
        assert_eq!(
            make_options_from_args_and_git_config(
                &["--navigate", "--raw"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .features,
            "my-feature navigate raw"
        );

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_recursive_feature_gathering_1() {
        let git_config_contents = b"
[delta]
    features = h g

[delta \"a\"]
    features = c b
    diff-highlight = true

[delta \"d\"]
    features = f e
    diff-so-fancy = true
";
        let git_config_path = "delta__test_feature_collection.gitconfig";

        assert_eq!(
            make_options_from_args_and_git_config(
                &["--raw", "--features", "d a"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .features,
            "raw diff-so-fancy f e d diff-highlight c b a"
        );

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_recursive_feature_gathering_2() {
        let git_config_contents = b"
[delta]
    features = feature-1

[delta \"feature-1\"]
    features = feature-2 feature-3

[delta \"feature-2\"]
    features = feature-4

[delta \"feature-4\"]
    minus-style = blue
";
        let git_config_path = "delta__test_recursive_features.gitconfig";
        let opt = make_options_from_args_and_git_config(
            &["delta"],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert_eq!(opt.features, "feature-4 feature-2 feature-3 feature-1");

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_main_section() {
        let git_config_contents = b"
[delta]
    minus-style = blue
";
        let git_config_path = "delta__test_main_section.gitconfig";

        // First check that it doesn't default to blue, because that's going to be used to signal
        // that gitconfig has set the style.
        assert_ne!(
            make_options_from_args_and_git_config(&[], None, None).minus_style,
            "blue"
        );

        // Check that --minus-style is honored as we expect.
        assert_eq!(
            make_options_from_args_and_git_config(&["--minus-style", "red"], None, None)
                .minus_style,
            "red"
        );

        // Check that gitconfig does not override a command line argument
        assert_eq!(
            make_options_from_args_and_git_config(
                &["--minus-style", "red"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            "red"
        );

        // Finally, check that gitconfig is honored when not overridden by a command line argument.
        assert_eq!(
            make_options_from_args_and_git_config(
                &[],
                Some(git_config_contents),
                Some(git_config_path)
            )
            .minus_style,
            "blue"
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
            make_options_from_args_and_git_config(
                &["--features", "my-feature"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            "green"
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
            make_options_from_args_and_git_config(
                &[],
                Some(git_config_contents),
                Some(git_config_path)
            )
            .minus_style,
            "blue"
        );

        // Event with --features the main section overrides the feature.
        assert_eq!(
            make_options_from_args_and_git_config(
                &["--features", "my-feature-1"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            "blue"
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
            make_options_from_args_and_git_config(
                &["--features", "my-feature-1 my-feature-2"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            "yellow"
        );

        assert_eq!(
            make_options_from_args_and_git_config(
                &["--features", "my-feature-2 my-feature-1"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            "green"
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

        let default = make_options_from_args_and_git_config(&[], None, None).minus_style;
        assert_ne!(default, "green");
        assert_ne!(default, "yellow");

        assert_eq!(
            make_options_from_args_and_git_config(
                &["--features", "my-feature-1"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            "green"
        );

        assert_eq!(
            make_options_from_args_and_git_config(
                &["--features", "my-feature-x"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            default
        );

        assert_eq!(
            make_options_from_args_and_git_config(
                &["--features", "my-feature-1 my-feature-x"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            "green"
        );

        assert_eq!(
            make_options_from_args_and_git_config(
                &["--features", "my-feature-x my-feature-2 my-feature-x"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            "yellow"
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
            make_options_from_args_and_git_config(&[], None, None).whitespace_error_style,
            "magenta reverse"
        );

        // Unspecified by user: color.diff.whitespace wins
        assert_eq!(
            make_options_from_args_and_git_config(
                &[],
                Some(git_config_contents),
                Some(git_config_path)
            )
            .whitespace_error_style,
            "yellow dim ul magenta"
        );

        // Command line argument wins
        assert_eq!(
            make_options_from_args_and_git_config(
                &["--whitespace-error-style", "red reverse"],
                Some(git_config_contents),
                Some(git_config_path)
            )
            .whitespace_error_style,
            "red reverse"
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
            make_options_from_args_and_git_config(
                &["--whitespace-error-style", "red reverse"],
                Some(git_config_contents),
                Some(git_config_path)
            )
            .whitespace_error_style,
            "red reverse"
        );

        // No command line argument or features; main [delta] section wins
        assert_eq!(
            make_options_from_args_and_git_config(
                &[],
                Some(git_config_contents),
                Some(git_config_path)
            )
            .whitespace_error_style,
            "blue reverse"
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
            make_options_from_args_and_git_config(
                &["--features", "my-whitespace-error-style-feature"],
                Some(git_config_contents),
                Some(git_config_path)
            )
            .whitespace_error_style,
            "blue reverse"
        );

        remove_file(git_config_path).unwrap();
    }
}
