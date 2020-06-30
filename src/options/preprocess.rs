use std::process;

use crate::bat::assets::HighlightingAssets;
use crate::cli;
use crate::env;
use crate::syntax_theme;

#[allow(non_snake_case)]
pub fn set__is_light_mode__syntax_theme__syntax_set(
    opt: &mut cli::Opt,
    assets: HighlightingAssets,
) {
    let syntax_theme_name_from_bat_theme = env::get_env_var("BAT_THEME");
    let (is_light_mode, syntax_theme_name) = syntax_theme::get_is_light_mode_and_theme_name(
        opt.syntax_theme.as_ref(),
        syntax_theme_name_from_bat_theme.as_ref(),
        opt.light,
        &assets.theme_set,
    );
    opt.computed.is_light_mode = is_light_mode;

    opt.computed.syntax_theme =
        if syntax_theme::is_no_syntax_highlighting_theme_name(&syntax_theme_name) {
            None
        } else {
            Some(assets.theme_set.themes[&syntax_theme_name].clone())
        };
    opt.computed.syntax_set = assets.syntax_set;
}
