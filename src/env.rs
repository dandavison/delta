use std::env;

const COLORTERM: &str = "COLORTERM";
const BAT_THEME: &str = "BAT_THEME";
const GIT_CONFIG_PARAMETERS: &str = "GIT_CONFIG_PARAMETERS";
const GIT_PREFIX: &str = "GIT_PREFIX";
const DELTA_FEATURES: &str = "DELTA_FEATURES";
const DELTA_NAVIGATE: &str = "DELTA_NAVIGATE";
const DELTA_EXPERIMENTAL_MAX_LINE_DISTANCE_FOR_NAIVELY_PAIRED_LINES: &str =
    "DELTA_EXPERIMENTAL_MAX_LINE_DISTANCE_FOR_NAIVELY_PAIRED_LINES";
const DELTA_PAGER: &str = "DELTA_PAGER";
const BAT_PAGER: &str = "BAT_PAGER";
const PAGER: &str = "PAGER";

#[derive(Default, Clone)]
pub struct DeltaEnv {
    pub bat_theme: Option<String>,
    pub colorterm: Option<String>,
    pub current_dir: Option<std::path::PathBuf>,
    pub experimental_max_line_distance_for_naively_paired_lines: Option<String>,
    pub features: Option<String>,
    pub git_config_parameters: Option<String>,
    pub git_prefix: Option<String>,
    pub navigate: Option<String>,
    pub pagers: (Option<String>, Option<String>, Option<String>),
}

impl DeltaEnv {
    /// Create a structure with current environment variable
    pub fn init() -> Self {
        let bat_theme = env_var(BAT_THEME);
        let colorterm = env_var(COLORTERM);
        let experimental_max_line_distance_for_naively_paired_lines =
            env_var(DELTA_EXPERIMENTAL_MAX_LINE_DISTANCE_FOR_NAIVELY_PAIRED_LINES);
        let features = env_var(DELTA_FEATURES);
        let git_config_parameters = env_var(GIT_CONFIG_PARAMETERS);
        let git_prefix = env_var(GIT_PREFIX);
        let navigate = env_var(DELTA_NAVIGATE);

        let current_dir = env::current_dir().ok();
        let pagers = (env_var(DELTA_PAGER), env_var(BAT_PAGER), env_var(PAGER));

        Self {
            bat_theme,
            colorterm,
            current_dir,
            experimental_max_line_distance_for_naively_paired_lines,
            features,
            git_config_parameters,
            git_prefix,
            navigate,
            pagers,
        }
    }
}

/// If `name` is set and, after trimming whitespace, is not empty string, then return that trimmed
/// string. Else None.
fn env_var(name: &str) -> Option<String> {
    match env::var(name).unwrap_or_else(|_| "".to_string()).trim() {
        "" => None,
        s => Some(s.to_string()),
    }
}

#[cfg(test)]
pub mod tests {
    use super::DeltaEnv;
    use std::env;

    #[test]
    fn test_env_parsing() {
        let feature = "Awesome Feature";
        env::set_var("DELTA_FEATURES", feature);
        let env = DeltaEnv::init();
        assert_eq!(env.features, Some(feature.into()));
    }
}
