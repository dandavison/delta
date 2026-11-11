use std::collections::HashMap;

use crate::cli;
use crate::features;
use crate::git_config::{self, GitConfigGet};
use crate::options::option_value::{OptionValue, ProvenancedOptionValue};
use ProvenancedOptionValue::*;

// Look up a value of type `T` associated with `option name`.
//
// A feature is enabled when the user's config or the command line turns it on, for example with
// side-by-side = 140 or features = side-by-side. A feature that another builtin feature brings in
// as a default, such as side-by-side's line numbers, can be in effect without being enabled. A
// feature is in effect when delta uses it for this output. delta decides that once per run, at
// the width it renders at (COLUMNS, or else the terminal's width), before it gathers the
// features: side-by-side = 140 enables side-by-side, but side-by-side is in effect, and gathered,
// only at 140 columns or more. So the feature list that this searches holds the features in
// effect.
//
// The search rules are:
//
// 1. If there is a value associated with `option_name` in the main [delta] git config
//    section, then stop searching and return that value.
//
// 2. Otherwise search the features in effect in these groups, and return the first value found:
//
//    2.1 The user's git config sections with the same name as a builtin feature that is in effect,
//        such as [delta "side-by-side"], in order of priority.
//
//    2.2 The other features in order of priority: for each, the user's git config section
//        [delta "feature"], or else the builtin feature's values, unless it is a switch feature.
//
//    2.3 The values of the switch features that are in effect, such as side-by-side, in order of
//        priority.
//
// So what a switch feature sets besides its own option, such as side-by-side's line-number
// formats, is a default that anything the user sets beats, however the switch feature was turned
// on; and a section with the same name as a builtin feature beats every other feature.
pub fn get_option_value<T>(
    option_name: &str,
    builtin_features: &HashMap<String, features::BuiltinFeature>,
    opt: &cli::Opt,
    git_config: &mut Option<git_config::GitConfig>,
) -> Option<T>
where
    T: GitConfigGet,
    T: GetOptionValue,
    T: From<OptionValue>,
    T: Into<OptionValue>,
{
    T::get_option_value(option_name, builtin_features, opt, git_config)
}

static GIT_CONFIG_THEME_REGEX: &str = r"^delta\.(.+)\.(light|dark)$";

pub fn get_themes(git_config: Option<git_config::GitConfig>) -> Vec<String> {
    let mut themes: Vec<String> = Vec::new();
    let git_config = git_config.unwrap();
    git_config.for_each(GIT_CONFIG_THEME_REGEX, |name, _| {
        if let Some(name) = name.strip_prefix("delta.") {
            if let Some((name, _)) = name.rsplit_once('.') {
                let name = name.to_owned();
                if !themes.contains(&name) {
                    themes.push(name);
                }
            }
        }
    });
    themes.sort_by_key(|a| a.to_lowercase());
    themes
}

pub trait GetOptionValue {
    fn get_option_value(
        option_name: &str,
        builtin_features: &HashMap<String, features::BuiltinFeature>,
        opt: &cli::Opt,
        git_config: &mut Option<git_config::GitConfig>,
    ) -> Option<Self>
    where
        Self: Sized,
        Self: GitConfigGet,
        Self: From<OptionValue>,
        Self: Into<OptionValue>,
    {
        if let Some(git_config) = git_config {
            if let Some(value) = git_config.get::<Self>(&format!("delta.{option_name}")) {
                return Some(value);
            }
        }
        let features: Vec<&str> = opt.features.as_ref()?.split_whitespace().rev().collect();
        let is_builtin = |feature: &&str| builtin_features.contains_key(*feature);
        let is_switch_feature = |feature: &&str| crate::options::set::is_switch_feature(feature);
        let user_section = |feature: &str| {
            git_config
                .as_ref()?
                .get::<Self>(&format!("delta.{feature}.{option_name}"))
                .map(Into::into)
        };
        let builtin = |feature: &str| {
            let value_function = builtin_features.get(feature)?.get(option_name)?;
            match value_function(opt, git_config) {
                GitConfigValue(value) | DefaultValue(value) => Some(value),
            }
        };
        let in_order = || features.iter().copied();
        in_order()
            .filter(is_builtin)
            .find_map(user_section)
            .or_else(|| {
                in_order().find_map(|feature| {
                    if !is_builtin(&feature) {
                        user_section(feature)
                    } else if !is_switch_feature(&feature) {
                        builtin(feature)
                    } else {
                        None
                    }
                })
            })
            .or_else(|| in_order().filter(is_switch_feature).find_map(builtin))
            .map(Into::into)
    }
}

impl GetOptionValue for Option<String> {}
impl GetOptionValue for String {}
impl GetOptionValue for bool {}
impl GetOptionValue for f64 {}
impl GetOptionValue for usize {}
impl GetOptionValue for cli::SideBySide {}

#[cfg(test)]
pub mod tests {
    use std::fs::remove_file;

    use crate::cli::Opt;
    use crate::env::DeltaEnv;
    use crate::options::get::get_themes;
    use crate::tests::integration_test_utils;

    // fn generic<T>(_s: SGen<T>) {}
    fn _test_env_var_overrides_git_config_generic(
        git_config_contents: &[u8],
        git_config_path: &str,
        env_value: String,
        fn_cmp_before: &dyn Fn(Opt),
        fn_cmp_after: &dyn Fn(Opt),
    ) {
        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        fn_cmp_before(opt);

        let opt = integration_test_utils::make_options_from_args_and_git_config_honoring_env_var_with_custom_env(
            DeltaEnv {
                git_config_parameters: Some(env_value),
                ..DeltaEnv::default()
            },
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        fn_cmp_after(opt);

        remove_file(git_config_path).unwrap();
    }
    #[test]
    fn test_env_var_overrides_git_config_simple_string() {
        let git_config_contents = b"
[delta]
    plus-style = blue
";
        let git_config_path = "delta__test_simple_string_env_var_overrides_git_config.gitconfig";
        _test_env_var_overrides_git_config_generic(
            git_config_contents,
            git_config_path,
            "'delta.plus-style=green'".into(),
            &|opt: Opt| assert_eq!(opt.plus_style, "blue"),
            &|opt: Opt| assert_eq!(opt.plus_style, "green"),
        );
    }

    #[test]
    fn test_env_var_overrides_git_config_complex_string() {
        let git_config_contents = br##"
[delta]
    minus-style = red bold ul "#ffeeee"
"##;
        let git_config_path = "delta__test_complex_string_env_var_overrides_git_config.gitconfig";
        _test_env_var_overrides_git_config_generic(
            git_config_contents,
            git_config_path,
            r##"'delta.minus-style=magenta italic ol "#aabbcc"'"##.into(),
            &|opt: Opt| assert_eq!(opt.minus_style, r##"red bold ul #ffeeee"##),
            &|opt: Opt| assert_eq!(opt.minus_style, r##"magenta italic ol "#aabbcc""##,),
        );
    }

    #[test]
    fn test_env_var_overrides_git_config_option_string() {
        let git_config_contents = b"
[delta]
    plus-style = blue
";
        let git_config_path = "delta__test_option_string_env_var_overrides_git_config.gitconfig";
        _test_env_var_overrides_git_config_generic(
            git_config_contents,
            git_config_path,
            "'delta.plus-style=green'".into(),
            &|opt: Opt| assert_eq!(opt.plus_style, "blue"),
            &|opt: Opt| assert_eq!(opt.plus_style, "green"),
        );
    }

    #[test]
    fn test_env_var_overrides_git_config_bool() {
        let git_config_contents = b"
[delta]
    side-by-side = true
";
        let git_config_path = "delta__test_bool_env_var_overrides_git_config.gitconfig";
        _test_env_var_overrides_git_config_generic(
            git_config_contents,
            git_config_path,
            "'delta.side-by-side=false'".into(),
            &|opt: Opt| assert!(opt.computed.side_by_side),
            &|opt: Opt| assert!(!opt.computed.side_by_side),
        );
    }

    #[test]
    fn test_env_var_overrides_git_config_int() {
        let git_config_contents = b"
[delta]
    max-line-length = 1
";
        let git_config_path = "delta__test_int_env_var_overrides_git_config.gitconfig";
        _test_env_var_overrides_git_config_generic(
            git_config_contents,
            git_config_path,
            "'delta.max-line-length=2'".into(),
            &|opt: Opt| assert_eq!(opt.max_line_length, 1),
            &|opt: Opt| assert_eq!(opt.max_line_length, 2),
        );
    }

    #[test]
    fn test_env_var_overrides_git_config_float() {
        let git_config_contents = b"
[delta]
    max-line-distance = 0.6
";
        let git_config_path = "delta__test_float_env_var_overrides_git_config.gitconfig";
        _test_env_var_overrides_git_config_generic(
            git_config_contents,
            git_config_path,
            "'delta.max-line-distance=0.7'".into(),
            &|opt: Opt| assert_eq!(opt.max_line_distance, 0.6),
            &|opt: Opt| assert_eq!(opt.max_line_distance, 0.7),
        );
    }

    #[test]
    fn test_side_by_side_value_in_git_config_takes_precedence_over_side_by_side_feature() {
        // The side-by-side feature sets side-by-side to "true", but like any option, a value in
        // the main [delta] section wins over the feature turned on in git config: booleans,
        // minimum widths and hunk alike. Adding side-by-side to the feature list on the command
        // line is the same as --side-by-side, which beats git config.
        for (i, value) in ["false", "99999", "hunk", "hunk,99999", "true"]
            .iter()
            .enumerate()
        {
            let git_config_contents = format!("[delta]\n    side-by-side = {value}\n");
            let with_feature = format!("{git_config_contents}    features = side-by-side\n");
            let git_config_path =
                format!("delta__test_side_by_side_feature_precedence_{i}.gitconfig");
            let side_by_side =
                |git_config_contents: &str, features: Option<&str>, args: &[&str]| {
                    let opt =
                    integration_test_utils::make_options_from_args_and_git_config_with_custom_env(
                        DeltaEnv {
                            features: features.map(str::to_string),
                            ..DeltaEnv::default()
                        },
                        args,
                        Some(git_config_contents.as_bytes()),
                        Some(&git_config_path),
                    );
                    (
                        opt.side_by_side,
                        opt.computed.side_by_side,
                        opt.computed.side_by_side_scope,
                    )
                };
            let without_feature = side_by_side(&git_config_contents, None, &[]);
            assert_eq!(without_feature.0, value.parse().unwrap());
            assert_eq!(side_by_side(&with_feature, None, &[]), without_feature);

            let with_flag = side_by_side(&git_config_contents, None, &["--side-by-side"]);
            assert_eq!(with_flag.0, "true".parse().unwrap());
            assert_eq!(
                side_by_side(&git_config_contents, Some("+side-by-side"), &[]),
                with_flag
            );
            assert_eq!(
                side_by_side(&git_config_contents, Some("side-by-side"), &[]),
                with_flag
            );
            remove_file(git_config_path).unwrap();
        }
    }

    #[test]
    fn test_delta_features_env_var() {
        let git_config_contents = b"
[delta]
    features = feature-from-gitconfig
";
        let git_config_path = "delta__test_delta_features_env_var.gitconfig";

        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert_eq!(opt.features.unwrap(), "feature-from-gitconfig");
        assert!(!opt.computed.side_by_side);

        let opt = integration_test_utils::make_options_from_args_and_git_config_with_custom_env(
            DeltaEnv {
                features: Some("side-by-side".into()),
                ..DeltaEnv::default()
            },
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        // `line-numbers` is a builtin feature induced by side-by-side
        assert_eq!(opt.features.unwrap(), "line-numbers side-by-side");
        assert!(opt.computed.side_by_side);

        let opt = integration_test_utils::make_options_from_args_and_git_config_with_custom_env(
            DeltaEnv {
                features: Some("+side-by-side".into()),
                ..DeltaEnv::default()
            },
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert_eq!(
            opt.features.unwrap(),
            "feature-from-gitconfig line-numbers side-by-side"
        );
        assert!(opt.computed.side_by_side);

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_get_themes_from_config() {
        let git_config_contents = r#"
[delta "dark-theme"]
    max-line-distance = 0.6
    dark = true

[delta "light-theme"]
    max-line-distance = 0.6
    light = true

[delta "light-and-dark-theme"]
    max-line-distance = 0.6
    light = true
    dark = true

[delta "Uppercase-Theme"]
    light = true

[delta "not-a-theme"]
    max-line-distance = 0.6
"#;
        let git_config_path = "delta__test_get_themes_git_config.gitconfig";

        let git_config = Some(integration_test_utils::make_git_config(
            &DeltaEnv::default(),
            git_config_contents.as_bytes(),
            git_config_path,
            false,
        ));

        let themes = get_themes(git_config);

        assert_eq!(
            themes,
            [
                "dark-theme",
                "light-and-dark-theme",
                "light-theme",
                "Uppercase-Theme"
            ]
        );

        remove_file(git_config_path).unwrap();
    }
}
