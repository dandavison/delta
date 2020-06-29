use std::process;

use crate::bat::assets::HighlightingAssets;
use crate::cli;
use crate::env;
use crate::syntax_theme;

pub fn preprocess_options(opt: &mut cli::Opt, assets: HighlightingAssets) {
    _check_validity(&opt, &assets);
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

fn _check_validity(opt: &cli::Opt, assets: &HighlightingAssets) {
    if opt.light && opt.dark {
        eprintln!("--light and --dark cannot be used together.");
        process::exit(1);
    }
    if let Some(ref syntax_theme) = opt.syntax_theme {
        if !syntax_theme::is_no_syntax_highlighting_theme_name(&syntax_theme) {
            if !assets.theme_set.themes.contains_key(syntax_theme.as_str()) {
                return;
            }
            let is_light_syntax_theme = syntax_theme::is_light_theme(&syntax_theme);
            if is_light_syntax_theme && opt.dark {
                eprintln!(
                    "{} is a light syntax theme, but you supplied --dark. \
                     If you use --syntax-theme, you do not need to supply --light or --dark.",
                    syntax_theme
                );
                process::exit(1);
            } else if !is_light_syntax_theme && opt.light {
                eprintln!(
                    "{} is a dark syntax theme, but you supplied --light. \
                     If you use --syntax-theme, you do not need to supply --light or --dark.",
                    syntax_theme
                );
                process::exit(1);
            }
        }
    }
}
