use syntect::highlighting::ThemeSet;

const LIGHT_THEMES: [&str; 5] = [
    "GitHub",
    "Monokai Extended Light",
    "OneHalfLight",
    "ansi-light",
    "Solarized (light)",
];

pub const DEFAULT_LIGHT_THEME: &str = "GitHub";
pub const DEFAULT_DARK_THEME: &str = "Monokai Extended";

pub fn is_light_theme(theme: &str) -> bool {
    LIGHT_THEMES.contains(&theme)
}

pub fn is_no_syntax_highlighting_theme_name(theme_name: &str) -> bool {
    theme_name.to_lowercase() == "none"
}

/// Return a (theme_name, is_light_mode) tuple.
/// theme_name == None in return value means syntax highlighting is disabled.
///
/// There are two types of color choices that have to be made:
/// 1. The choice of "theme". This is the language syntax highlighting theme; you have to make this choice when using `bat` also.
/// 2. The choice of "light vs dark mode". This determines whether the background colors should be chosen for a light or dark terminal background. (`bat` has no equivalent.)
///
/// Basically:
/// 1. The theme is specified by the `--theme` option. If this isn't supplied then it is specified by the `BAT_PAGER` environment variable.
/// 2. Light vs dark mode is specified by the `--light` or `--dark` options. If these aren't supplied then it is inferred from the chosen theme.
///
/// In the absence of other factors, the default assumes a dark terminal background.
///
/// Specifically, the rules are as follows:
///
/// | --theme    | $BAT_THEME | --light/--dark | Behavior                                                                   |
/// |------------|------------|----------------|----------------------------------------------------------------------------|
/// | -          | -          | -              | default dark theme, dark mode                                              |
/// | some_theme | (IGNORED)  | -              | some_theme with light/dark mode inferred accordingly                       |
/// | -          | BAT_THEME  | -              | BAT_THEME, with light/dark mode inferred accordingly                       |
/// | -          | -          | yes            | default light/dark theme, light/dark mode                                  |
/// | some_theme | (IGNORED)  | yes            | some_theme, light/dark mode (even if some_theme conflicts with light/dark) |
/// | -          | BAT_THEME  | yes            | BAT_THEME, light/dark mode (even if BAT_THEME conflicts with light/dark)   |
pub fn get_is_light_mode_and_theme_name(
    theme_arg: Option<&String>,
    bat_theme_env_var: Option<&String>,
    light_mode_arg: bool,
    theme_set: &ThemeSet,
) -> (bool, String) {
    let theme_arg = valid_theme_name_or_none(theme_arg, theme_set);
    let bat_theme_env_var = valid_theme_name_or_none(bat_theme_env_var, theme_set);
    match (theme_arg, bat_theme_env_var, light_mode_arg) {
        (None, None, false) => (false, DEFAULT_DARK_THEME.to_string()),
        (Some(theme_name), _, false) => (is_light_theme(&theme_name), theme_name),
        (None, Some(theme_name), false) => (is_light_theme(&theme_name), theme_name),
        (None, None, true) => (true, DEFAULT_LIGHT_THEME.to_string()),
        (Some(theme_name), _, is_light_mode) => (is_light_mode, theme_name),
        (None, Some(theme_name), is_light_mode) => (is_light_mode, theme_name),
    }
}

// At this stage the theme name is considered valid if it is either a real theme name or the special
// no-syntax-highlighting name.
fn valid_theme_name_or_none(theme_name: Option<&String>, theme_set: &ThemeSet) -> Option<String> {
    match theme_name {
        Some(name)
            if is_no_syntax_highlighting_theme_name(name)
                || theme_set.themes.contains_key(name) =>
        {
            Some(name.to_string())
        }
        _ => None,
    }
}
