/// Delta doesn't have a formal concept of a "theme". What it has is
/// (a) the choice of syntax-highlighting theme
/// (b) the choice of light-background-mode vs dark-background-mode, which determine certain
///     default color choices
/// This module sets those options. If the light/dark background mode choice is not made explicitly
/// by the user, it is determined by the classification of the syntax theme into light-background
/// vs dark-background syntax themes. If the user didn't choose a syntax theme, a dark-background
/// default is selected.
use bat;
use bat::assets::HighlightingAssets;

use crate::cli;

#[allow(non_snake_case)]
pub fn set__is_light_mode__syntax_theme__syntax_set(
    opt: &mut cli::Opt,
    assets: HighlightingAssets,
) {
    let syntax_theme_name_from_bat_theme = &opt.env.bat_theme;
    let (is_light_mode, syntax_theme_name) = get_is_light_mode_and_syntax_theme_name(
        opt.syntax_theme.as_ref(),
        syntax_theme_name_from_bat_theme.as_ref(),
        opt.light,
    );
    opt.computed.is_light_mode = is_light_mode;

    opt.computed.syntax_theme = if is_no_syntax_highlighting_syntax_theme_name(&syntax_theme_name) {
        None
    } else {
        Some(assets.get_theme(&syntax_theme_name).clone())
    };
    opt.computed.syntax_set = assets.get_syntax_set().unwrap().clone();
}

pub fn is_light_syntax_theme(theme: &str) -> bool {
    LIGHT_SYNTAX_THEMES.contains(&theme) || theme.to_lowercase().contains("light")
}

const LIGHT_SYNTAX_THEMES: [&str; 6] = [
    "GitHub",
    "gruvbox-light",
    "gruvbox-white",
    "Monokai Extended Light",
    "OneHalfLight",
    "Solarized (light)",
];

const DEFAULT_LIGHT_SYNTAX_THEME: &str = "GitHub";
const DEFAULT_DARK_SYNTAX_THEME: &str = "Monokai Extended";

fn is_no_syntax_highlighting_syntax_theme_name(theme_name: &str) -> bool {
    theme_name.to_lowercase() == "none"
}

/// Return a (theme_name, is_light_mode) tuple.
/// theme_name == None in return value means syntax highlighting is disabled.
///
/// There are two types of color choices that have to be made:

/// 1. The choice of "theme". This is the language syntax highlighting theme; you have to make this
///    choice when using `bat` also.
/// 2. The choice of "light vs dark mode". This determines whether the background colors should be
///    chosen for a light or dark terminal background. (`bat` has no equivalent.)
///
/// Basically:
/// 1. The theme is specified by the `--syntax-theme` option. If this isn't supplied then it is specified
///    by the `BAT_THEME` environment variable.
/// 2. Light vs dark mode is specified by the `--light` or `--dark` options. If these aren't
///    supplied then it is inferred from the chosen theme.
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
fn get_is_light_mode_and_syntax_theme_name(
    theme_arg: Option<&String>,
    bat_theme_env_var: Option<&String>,
    light_mode_arg: bool,
) -> (bool, String) {
    match (theme_arg, bat_theme_env_var, light_mode_arg) {
        (None, None, false) => (false, DEFAULT_DARK_SYNTAX_THEME.to_string()),
        (Some(theme_name), _, false) => (is_light_syntax_theme(theme_name), theme_name.to_string()),
        (None, Some(theme_name), false) => {
            (is_light_syntax_theme(theme_name), theme_name.to_string())
        }
        (None, None, true) => (true, DEFAULT_LIGHT_SYNTAX_THEME.to_string()),
        (Some(theme_name), _, is_light_mode) => (is_light_mode, theme_name.to_string()),
        (None, Some(theme_name), is_light_mode) => (is_light_mode, theme_name.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color;
    use crate::tests::integration_test_utils;

    // TODO: Test influence of BAT_THEME env var. E.g. see utils::process::tests::FakeParentArgs.
    #[test]
    fn test_syntax_theme_selection() {
        #[derive(PartialEq)]
        enum Mode {
            Light,
            Dark,
        }
        for (
            syntax_theme,
            mode, // (--light, --dark)
            expected_syntax_theme,
            expected_mode,
        ) in vec![
            (None, None, DEFAULT_DARK_SYNTAX_THEME, Mode::Dark),
            (Some("GitHub"), None, "GitHub", Mode::Light),
            (Some("GitHub"), None, "GitHub", Mode::Light),
            (
                None,
                Some(Mode::Light),
                DEFAULT_LIGHT_SYNTAX_THEME,
                Mode::Light,
            ),
            (
                None,
                Some(Mode::Dark),
                DEFAULT_DARK_SYNTAX_THEME,
                Mode::Dark,
            ),
            (
                None,
                Some(Mode::Light),
                DEFAULT_LIGHT_SYNTAX_THEME,
                Mode::Light,
            ),
            (None, Some(Mode::Light), "GitHub", Mode::Light),
            (Some("none"), None, "none", Mode::Dark),
            (Some("None"), Some(Mode::Light), "none", Mode::Light),
        ] {
            let mut args = vec![];
            if let Some(syntax_theme) = syntax_theme {
                args.push("--syntax-theme");
                args.push(syntax_theme);
            }
            let is_true_color = true;
            if is_true_color {
                args.push("--true-color");
                args.push("always");
            } else {
                args.push("--true-color");
                args.push("never");
            }
            match mode {
                Some(Mode::Light) => {
                    args.push("--light");
                }
                Some(Mode::Dark) => {
                    args.push("--dark");
                }
                None => {}
            }
            let config = integration_test_utils::make_config_from_args(&args);
            assert_eq!(
                &config
                    .syntax_theme
                    .clone()
                    .map(|t| t.name.unwrap())
                    .unwrap_or("none".to_string()),
                expected_syntax_theme
            );
            if is_no_syntax_highlighting_syntax_theme_name(expected_syntax_theme) {
                assert!(config.syntax_theme.is_none())
            } else {
                assert_eq!(
                    config.syntax_theme.unwrap().name.as_ref().unwrap(),
                    expected_syntax_theme
                );
            }
            assert_eq!(
                config.minus_style.ansi_term_style.background.unwrap(),
                color::get_minus_background_color_default(
                    expected_mode == Mode::Light,
                    is_true_color
                )
            );
            assert_eq!(
                config.minus_emph_style.ansi_term_style.background.unwrap(),
                color::get_minus_emph_background_color_default(
                    expected_mode == Mode::Light,
                    is_true_color
                )
            );
            assert_eq!(
                config.plus_style.ansi_term_style.background.unwrap(),
                color::get_plus_background_color_default(
                    expected_mode == Mode::Light,
                    is_true_color
                )
            );
            assert_eq!(
                config.plus_emph_style.ansi_term_style.background.unwrap(),
                color::get_plus_emph_background_color_default(
                    expected_mode == Mode::Light,
                    is_true_color
                )
            );
        }
    }
}
