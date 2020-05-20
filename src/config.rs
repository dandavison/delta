use std::process;
use std::str::FromStr;

use bit_set::BitSet;
use syntect::highlighting::{Color, Style, StyleModifier, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;

use crate::bat::output::PagingMode;
use crate::cli;
use crate::delta::State;
use crate::env;
use crate::paint;
use crate::style;

pub struct Config<'a> {
    pub theme: Option<Theme>,
    pub theme_name: String,
    pub dummy_theme: Theme,
    pub max_line_distance: f64,
    pub max_line_distance_for_naively_paired_lines: f64,
    pub minus_style_modifier: StyleModifier,
    pub minus_emph_style_modifier: StyleModifier,
    pub plus_style_modifier: StyleModifier,
    pub plus_emph_style_modifier: StyleModifier,
    pub minus_line_marker: &'a str,
    pub plus_line_marker: &'a str,
    pub lines_to_be_syntax_highlighted: BitSet,
    pub commit_style: cli::SectionStyle,
    pub commit_color: Color,
    pub file_style: cli::SectionStyle,
    pub file_color: Color,
    pub hunk_style: cli::SectionStyle,
    pub hunk_color: Color,
    pub syntax_set: SyntaxSet,
    pub terminal_width: usize,
    pub true_color: bool,
    pub background_color_extends_to_terminal_width: bool,
    pub tab_width: usize,
    pub no_style: Style,
    pub max_buffered_lines: usize,
    pub paging_mode: PagingMode,
}

#[allow(dead_code)]
pub enum ColorLayer {
    Background,
    Foreground,
}
use ColorLayer::*;
use State::*;

impl<'a> Config<'a> {
    #[allow(dead_code)]
    pub fn get_color(&self, state: &State, layer: ColorLayer) -> Option<Color> {
        let modifier = match state {
            HunkMinus => Some(self.minus_style_modifier),
            HunkZero => None,
            HunkPlus => Some(self.plus_style_modifier),
            _ => panic!("Invalid"),
        };
        match (modifier, layer) {
            (Some(modifier), Background) => modifier.background,
            (Some(modifier), Foreground) => modifier.foreground,
            (None, _) => None,
        }
    }

    #[allow(dead_code)]
    pub fn should_syntax_highlight(&self, state: &State) -> bool {
        self.lines_to_be_syntax_highlighted
            .contains((*state).clone() as usize)
    }
}

pub fn get_config<'a>(
    opt: cli::Opt,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    true_color: bool,
    terminal_width: usize,
    paging_mode: PagingMode,
    lines_to_be_syntax_highlighted: BitSet,
) -> Config<'a> {
    // Implement --color-only
    let keep_plus_minus_markers = if opt.color_only {
        true
    } else {
        opt.keep_plus_minus_markers
    };
    let background_color_extends_to_terminal_width = opt.width != Some("variable".to_string());
    let tab_width = if opt.color_only { 0 } else { opt.tab_width };
    let (commit_style, file_style, hunk_style) = if opt.color_only {
        (
            cli::SectionStyle::Plain,
            cli::SectionStyle::Plain,
            cli::SectionStyle::Plain,
        )
    } else {
        (opt.commit_style, opt.file_style, opt.hunk_style)
    };

    let theme_name_from_bat_pager = env::get_env_var("BAT_THEME");
    let (is_light_mode, theme_name) = get_is_light_mode_and_theme_name(
        opt.theme.as_ref(),
        theme_name_from_bat_pager.as_ref(),
        opt.light,
        &theme_set,
    );

    let (
        minus_style_modifier,
        minus_emph_style_modifier,
        plus_style_modifier,
        plus_emph_style_modifier,
    ) = make_styles(&opt, is_light_mode, true_color);

    let theme = if style::is_no_syntax_highlighting_theme_name(&theme_name) {
        None
    } else {
        Some(theme_set.themes[&theme_name].clone())
    };
    let dummy_theme = theme_set.themes.values().next().unwrap().clone();

    let minus_line_marker = if keep_plus_minus_markers { "-" } else { " " };
    let plus_line_marker = if keep_plus_minus_markers { "+" } else { " " };

    let max_line_distance_for_naively_paired_lines =
        env::get_env_var("DELTA_EXPERIMENTAL_MAX_LINE_DISTANCE_FOR_NAIVELY_PAIRED_LINES")
            .map(|s| s.parse::<f64>().unwrap_or(0.0))
            .unwrap_or(0.0);

    Config {
        theme,
        theme_name,
        dummy_theme,
        max_line_distance: opt.max_line_distance,
        max_line_distance_for_naively_paired_lines,
        minus_style_modifier,
        minus_emph_style_modifier,
        plus_style_modifier,
        plus_emph_style_modifier,
        lines_to_be_syntax_highlighted,
        minus_line_marker,
        plus_line_marker,
        commit_style,
        commit_color: color_from_rgb_or_ansi_code(&opt.commit_color),
        file_style,
        file_color: color_from_rgb_or_ansi_code(&opt.file_color),
        hunk_style,
        hunk_color: color_from_rgb_or_ansi_code(&opt.hunk_color),
        true_color,
        terminal_width,
        background_color_extends_to_terminal_width,
        tab_width,
        syntax_set,
        no_style: style::get_no_style(),
        max_buffered_lines: 32,
        paging_mode,
    }
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
fn get_is_light_mode_and_theme_name(
    theme_arg: Option<&String>,
    bat_theme_env_var: Option<&String>,
    light_mode_arg: bool,
    theme_set: &ThemeSet,
) -> (bool, String) {
    let theme_arg = valid_theme_name_or_none(theme_arg, theme_set);
    let bat_theme_env_var = valid_theme_name_or_none(bat_theme_env_var, theme_set);
    match (theme_arg, bat_theme_env_var, light_mode_arg) {
        (None, None, false) => (false, style::DEFAULT_DARK_THEME.to_string()),
        (Some(theme_name), _, false) => (style::is_light_theme(&theme_name), theme_name),
        (None, Some(theme_name), false) => (style::is_light_theme(&theme_name), theme_name),
        (None, None, true) => (true, style::DEFAULT_LIGHT_THEME.to_string()),
        (Some(theme_name), _, is_light_mode) => (is_light_mode, theme_name),
        (None, Some(theme_name), is_light_mode) => (is_light_mode, theme_name),
    }
}

// At this stage the theme name is considered valid if it is either a real theme name or the special
// no-syntax-highlighting name.
fn valid_theme_name_or_none(theme_name: Option<&String>, theme_set: &ThemeSet) -> Option<String> {
    match theme_name {
        Some(name)
            if style::is_no_syntax_highlighting_theme_name(name)
                || theme_set.themes.contains_key(name) =>
        {
            Some(name.to_string())
        }
        _ => None,
    }
}

fn make_styles<'a>(
    opt: &'a cli::Opt,
    is_light_mode: bool,
    true_color: bool,
) -> (StyleModifier, StyleModifier, StyleModifier, StyleModifier) {
    let minus_style = make_style(
        opt.minus_color.as_deref(),
        Some(style::get_minus_color_default(is_light_mode, true_color)),
        opt.minus_foreground_color.as_deref(),
        None,
    );

    let minus_emph_style = make_style(
        opt.minus_emph_color.as_deref(),
        Some(style::get_minus_emph_color_default(
            is_light_mode,
            true_color,
        )),
        opt.minus_emph_foreground_color.as_deref(),
        minus_style.foreground,
    );

    let plus_style = make_style(
        opt.plus_color.as_deref(),
        Some(style::get_plus_color_default(is_light_mode, true_color)),
        opt.plus_foreground_color.as_deref(),
        None,
    );

    let plus_emph_style = make_style(
        opt.plus_emph_color.as_deref(),
        Some(style::get_plus_emph_color_default(
            is_light_mode,
            true_color,
        )),
        opt.plus_emph_foreground_color.as_deref(),
        plus_style.foreground,
    );

    (minus_style, minus_emph_style, plus_style, plus_emph_style)
}

fn make_style(
    background: Option<&str>,
    background_default: Option<Color>,
    foreground: Option<&str>,
    foreground_default: Option<Color>,
) -> StyleModifier {
    StyleModifier {
        background: color_from_rgb_or_ansi_code_with_default(background, background_default),
        foreground: color_from_rgb_or_ansi_code_with_default(foreground, foreground_default),
        font_style: None,
    }
}

fn color_from_rgb_or_ansi_code(s: &str) -> Color {
    let die = || {
        eprintln!("Invalid color: {}", s);
        process::exit(1);
    };
    if s.starts_with("#") {
        Color::from_str(s).unwrap_or_else(|_| die())
    } else {
        s.parse::<u8>()
            .ok()
            .and_then(paint::color_from_ansi_number)
            .or_else(|| paint::color_from_ansi_name(s))
            .unwrap_or_else(die)
    }
}

fn color_from_rgb_or_ansi_code_with_default(
    arg: Option<&str>,
    default: Option<Color>,
) -> Option<Color> {
    match arg {
        Some(string) if string.to_lowercase() == "none" => None,
        Some(string) => Some(color_from_rgb_or_ansi_code(&string)),
        None => default,
    }
}
