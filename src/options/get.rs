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
        return None;
    }
}

impl GetOptionValue for Option<String> {}
impl GetOptionValue for String {}
impl GetOptionValue for bool {}
impl GetOptionValue for f64 {}
impl GetOptionValue for usize {}
