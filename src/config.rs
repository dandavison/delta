use std::str::FromStr;

use syntect::highlighting::{Color, Style, StyleModifier, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;

use crate::cli;
use crate::style;

pub struct Config<'a> {
    pub theme: &'a Theme,
    pub theme_name: &'a str,
    pub minus_style_modifier: StyleModifier,
    pub minus_emph_style_modifier: StyleModifier,
    pub plus_style_modifier: StyleModifier,
    pub plus_emph_style_modifier: StyleModifier,
    pub syntax_set: &'a SyntaxSet,
    pub terminal_width: usize,
    pub width: Option<usize>,
    pub pager: &'a str,
    pub opt: &'a cli::Opt,
    pub no_style: Style,
}

pub fn get_config<'a>(
    opt: &'a cli::Opt,
    syntax_set: &'a SyntaxSet,
    theme_set: &'a ThemeSet,
    terminal_width: usize,
    width: Option<usize>,
) -> Config<'a> {
    let theme_name = match opt.theme {
        Some(ref theme) => theme,
        None => match opt.light {
            true => style::DEFAULT_LIGHT_THEME,
            false => style::DEFAULT_DARK_THEME,
        },
    };
    let is_light_theme = style::LIGHT_THEMES.contains(&theme_name);

    let minus_style_modifier = StyleModifier {
        background: Some(color_from_arg(
            &opt.minus_color,
            is_light_theme,
            style::LIGHT_THEME_MINUS_COLOR,
            style::DARK_THEME_MINUS_COLOR,
        )),
        foreground: if opt.highlight_removed {
            None
        } else {
            Some(style::NO_COLOR)
        },
        font_style: None,
    };

    let minus_emph_style_modifier = StyleModifier {
        background: Some(color_from_arg(
            &opt.minus_emph_color,
            is_light_theme,
            style::LIGHT_THEME_MINUS_EMPH_COLOR,
            style::DARK_THEME_MINUS_EMPH_COLOR,
        )),
        foreground: if opt.highlight_removed {
            None
        } else {
            Some(style::NO_COLOR)
        },
        font_style: None,
    };

    let plus_style_modifier = StyleModifier {
        background: Some(color_from_arg(
            &opt.plus_color,
            is_light_theme,
            style::LIGHT_THEME_PLUS_COLOR,
            style::DARK_THEME_PLUS_COLOR,
        )),
        foreground: None,
        font_style: None,
    };

    let plus_emph_style_modifier = StyleModifier {
        background: Some(color_from_arg(
            &opt.plus_emph_color,
            is_light_theme,
            style::LIGHT_THEME_PLUS_EMPH_COLOR,
            style::DARK_THEME_PLUS_EMPH_COLOR,
        )),
        foreground: None,
        font_style: None,
    };

    Config {
        theme: &theme_set.themes[theme_name],
        theme_name: theme_name,
        minus_style_modifier,
        minus_emph_style_modifier,
        plus_style_modifier,
        plus_emph_style_modifier,
        terminal_width,
        width,
        syntax_set,
        pager: "less",
        opt,
        no_style: style::get_no_style(),
    }
}

fn color_from_arg(
    arg: &Option<String>,
    is_light_theme: bool,
    light_theme_default: Color,
    dark_theme_default: Color,
) -> Color {
    arg.as_ref()
        .and_then(|s| Color::from_str(s).ok())
        .unwrap_or_else(|| {
            if is_light_theme {
                light_theme_default
            } else {
                dark_theme_default
            }
        })
}
