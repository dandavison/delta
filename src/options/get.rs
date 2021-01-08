use std::collections::HashMap;

use crate::cli;
use crate::features;
use crate::git_config::{self, GitConfigGet};
use crate::options::option_value::{OptionValue, ProvenancedOptionValue};
use ProvenancedOptionValue::*;

/// Look up a value of type `T` associated with `option name`. The search rules are:
///
/// 1. If there is a value associated with `option_name` in the main [delta] git config
///    section, then stop searching and return that value.
///
/// 2. For each feature in the ordered list of enabled features:
///
///    2.1 Look-up the value, treating `feature` as a custom feature.
///        I.e., if there is a value associated with `option_name` in a git config section
///        named [delta "`feature`"] then stop searching and return that value.
///
///    2.2 Look-up the value, treating `feature` as a builtin feature.
///        I.e., if there is a value (not a default value) associated with `option_name` in a
///        builtin feature named `feature`, then stop searching and return that value.
///        Otherwise, record the default value and continue searching.
///
/// 3. Return the last default value that was encountered.
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
            if let Some(value) = git_config.get::<Self>(&format!("delta.{}", option_name)) {
                return Some(value);
            }
        }
        for feature in opt.features.to_lowercase().split_whitespace().rev() {
            match Self::get_provenanced_value_for_feature(
                option_name,
                &feature,
                &builtin_features,
                opt,
                git_config,
            ) {
                Some(GitConfigValue(value)) | Some(DefaultValue(value)) => {
                    return Some(value.into());
                }
                None => {}
            }
        }
        None
    }

    /// Return the value, or default value, associated with `option_name` under feature name
    /// `feature`. This may refer to a custom feature, or a builtin feature, or both. Only builtin
    /// features have defaults. See `GetOptionValue::get_option_value`.
    fn get_provenanced_value_for_feature(
        option_name: &str,
        feature: &str,
        builtin_features: &HashMap<String, features::BuiltinFeature>,
        opt: &cli::Opt,
        git_config: &mut Option<git_config::GitConfig>,
    ) -> Option<ProvenancedOptionValue>
    where
        Self: Sized,
        Self: GitConfigGet,
        Self: Into<OptionValue>,
    {
        if let Some(git_config) = git_config {
            if let Some(value) =
                git_config.get::<Self>(&format!("delta.{}.{}", feature, option_name))
            {
                return Some(GitConfigValue(value.into()));
            }
        }
        if let Some(builtin_feature) = builtin_features.get(feature) {
            if let Some(value_function) = builtin_feature.get(option_name) {
                return Some(value_function(opt, &git_config));
            }
        }
        None
    }
}

impl GetOptionValue for Option<String> {}
impl GetOptionValue for String {}
impl GetOptionValue for bool {}
impl GetOptionValue for f64 {}
impl GetOptionValue for usize {}

#[cfg(test)]
pub mod tests {
    use std::env;
    use std::fs::remove_file;

    use crate::tests::integration_test_utils::integration_test_utils;

    // TODO: the followig tests are collapsed into one since they all set the same env var and thus
    // could affect each other if allowed to run concurrently.

    #[test]
    fn test_env_var_overrides_git_config() {
        // ----------------------------------------------------------------------------------------
        // simple string
        let git_config_contents = b"
[delta]
    plus-style = blue
";
        let git_config_path = "delta__test_simple_string_env_var_overrides_git_config.gitconfig";

        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert_eq!(opt.plus_style, "blue");

        env::set_var("GIT_CONFIG_PARAMETERS", "'delta.plus-style=green'");
        let opt = integration_test_utils::make_options_from_args_and_git_config_honoring_env_var(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert_eq!(opt.plus_style, "green");

        remove_file(git_config_path).unwrap();

        // ----------------------------------------------------------------------------------------
        // complex string
        let git_config_contents = br##"
[delta]
    minus-style = red bold ul "#ffeeee"
"##;
        let git_config_path = "delta__test_complex_string_env_var_overrides_git_config.gitconfig";

        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert_eq!(opt.minus_style, r##"red bold ul #ffeeee"##);

        env::set_var(
            "GIT_CONFIG_PARAMETERS",
            r##"'delta.minus-style=magenta italic ol "#aabbcc"'"##,
        );
        let opt = integration_test_utils::make_options_from_args_and_git_config_honoring_env_var(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert_eq!(opt.minus_style, r##"magenta italic ol "#aabbcc""##,);

        remove_file(git_config_path).unwrap();

        // ----------------------------------------------------------------------------------------
        // option string
        let git_config_contents = b"
[delta]
    plus-style = blue
";
        let git_config_path = "delta__test_option_string_env_var_overrides_git_config.gitconfig";

        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert_eq!(opt.plus_style, "blue");

        env::set_var("GIT_CONFIG_PARAMETERS", "'delta.plus-style=green'");
        let opt = integration_test_utils::make_options_from_args_and_git_config_honoring_env_var(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert_eq!(opt.plus_style, "green");

        remove_file(git_config_path).unwrap();

        // ----------------------------------------------------------------------------------------
        // bool
        let git_config_contents = b"
[delta]
    side-by-side = true
";
        let git_config_path = "delta__test_bool_env_var_overrides_git_config.gitconfig";

        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert_eq!(opt.side_by_side, true);

        env::set_var("GIT_CONFIG_PARAMETERS", "'delta.side-by-side=false'");
        let opt = integration_test_utils::make_options_from_args_and_git_config_honoring_env_var(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert_eq!(opt.side_by_side, false);

        remove_file(git_config_path).unwrap();

        // ----------------------------------------------------------------------------------------
        // int
        let git_config_contents = b"
[delta]
    max-line-length = 1
";
        let git_config_path = "delta__test_int_env_var_overrides_git_config.gitconfig";

        let opt = integration_test_utils::make_options_from_args_and_git_config(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert_eq!(opt.max_line_length, 1);

        env::set_var("GIT_CONFIG_PARAMETERS", "'delta.max-line-length=2'");
        let opt = integration_test_utils::make_options_from_args_and_git_config_honoring_env_var(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert_eq!(opt.max_line_length, 2);

        remove_file(git_config_path).unwrap();

        // ----------------------------------------------------------------------------------------
        // float
        let git_config_contents = b"
[delta]
    max-line-distance = 0.6
";
        let git_config_path = "delta__test_float_env_var_overrides_git_config.gitconfig";

        let opt = integration_test_utils::make_options_from_args_and_git_config_honoring_env_var(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert_eq!(opt.max_line_distance, 0.6);

        env::set_var("GIT_CONFIG_PARAMETERS", "'delta.max-line-distance=0.7'");
        let opt = integration_test_utils::make_options_from_args_and_git_config_honoring_env_var(
            &[],
            Some(git_config_contents),
            Some(git_config_path),
        );
        assert_eq!(opt.max_line_distance, 0.7);

        remove_file(git_config_path).unwrap();
    }
}
