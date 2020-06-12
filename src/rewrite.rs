/// This module applies rewrite rules to the command line options, in order to
/// 1. Express deprecated usages in the new non-deprecated form
/// 2. Implement options such as --color-only which are defined to be equivalent to some set of
///    other options.
use std::process;

use structopt::clap;

use crate::cli;
use crate::config::user_supplied_option;
use crate::gitconfig::GetOptionValue;

pub fn apply_rewrite_rules(
    opt: &mut cli::Opt,
    arg_matches: clap::ArgMatches,
    git_config: &mut Option<git2::Config>,
) {
    rewrite_options_to_honor_git_config(opt, &arg_matches, git_config);
    rewrite_style_strings_to_honor_deprecated_minus_plus_options(opt);
    rewrite_options_to_implement_deprecated_commit_and_file_style_box_option(opt);
    rewrite_options_to_implement_deprecated_hunk_style_option(opt);
    rewrite_options_to_implement_deprecated_theme_option(opt, &arg_matches);
    rewrite_options_to_implement_color_only(opt);
    rewrite_options_to_implement_navigate(opt, &arg_matches);
}

/// Implement --color-only
fn rewrite_options_to_implement_color_only(opt: &mut cli::Opt) {
    if opt.color_only {
        opt.keep_plus_minus_markers = true;
        opt.tab_width = 0;
        opt.commit_style = "raw".to_string();
        opt.commit_decoration_style = "none".to_string();
        opt.file_style = "raw".to_string();
        opt.file_decoration_style = "none".to_string();
        opt.hunk_header_style = "raw".to_string();
        opt.hunk_header_decoration_style = "none".to_string();
    }
}

fn rewrite_options_to_honor_git_config(
    opt: &mut cli::Opt,
    arg_matches: &clap::ArgMatches,
    git_config: &mut Option<git2::Config>,
) {
    if opt.no_gitconfig {
        return;
    }
    // --presets must be set first
    set_options__option_string!([("presets", presets)], opt, arg_matches, git_config);
    set_options__bool!(
        [
            ("light", light),
            ("dark", dark),
            ("navigate", navigate),
            ("color-only", color_only),
            ("keep-plus-minus-markers", keep_plus_minus_markers),
            ("number", show_line_numbers)
        ],
        opt,
        arg_matches,
        git_config
    );
    set_options__f64!(
        [("max-line-distance", max_line_distance)],
        opt,
        arg_matches,
        git_config
    );
    set_options__string!(
        [
            ("commit-decoration-style", commit_decoration_style),
            ("commit-style", commit_style),
            ("file-added-label", file_added_label),
            ("file-decoration-style", file_decoration_style),
            ("file-modified-label", file_modified_label),
            ("file-removed-label", file_removed_label),
            ("file-renamed-label", file_renamed_label),
            ("file-style", file_style),
            ("hunk-header-decoration-style", hunk_header_decoration_style),
            ("hunk-header-style", hunk_header_style),
            // Hack: minus-style must come before minus-*emph-style because the latter default
            // dynamically to the value of the former.
            ("minus-style", minus_style),
            ("minus-emph-style", minus_emph_style),
            ("minus-non-emph-style", minus_non_emph_style),
            ("number-minus-format", number_minus_format),
            ("number-minus-format-style", number_minus_format_style),
            ("number-minus-style", number_minus_style),
            ("number-plus-format", number_plus_format),
            ("number-plus-format-style", number_plus_format_style),
            ("number-plus-style", number_plus_style),
            ("paging-mode", paging_mode),
            // Hack: plus-style must come before plus-*emph-style because the latter default
            // dynamically to the value of the former.
            ("plus-style", plus_style),
            ("plus-emph-style", plus_emph_style),
            ("plus-non-emph-style", plus_non_emph_style),
            ("true-color", true_color),
            ("word-diff-regex", tokenization_regex),
            ("zero-style", zero_style)
        ],
        opt,
        arg_matches,
        git_config
    );
    set_options__option_string!(
        [("syntax_theme", syntax_theme), ("width", width)],
        opt,
        arg_matches,
        git_config
    );
    set_options__usize!([("tabs", tab_width)], opt, arg_matches, git_config);
}

/// Implement --navigate
fn rewrite_options_to_implement_navigate(opt: &mut cli::Opt, arg_matches: &clap::ArgMatches) {
    if opt.navigate {
        if !user_supplied_option("file-modified-label", arg_matches) {
            opt.file_modified_label = "Δ".to_string();
        }
    }
}

/// Honor deprecated --theme
fn rewrite_options_to_implement_deprecated_theme_option(
    opt: &mut cli::Opt,
    arg_matches: &clap::ArgMatches,
) {
    if user_supplied_option("deprecated-theme", arg_matches) {
        if let Some(syntax_theme) = opt.deprecated_theme.as_ref() {
            opt.syntax_theme = Some(syntax_theme.to_string());
        }
    }
}

/// Honor deprecated arguments by rewriting the canonical --*-style arguments if appropriate.
// TODO: How to avoid repeating the default values for style options here and in
// the structopt definition?
fn rewrite_style_strings_to_honor_deprecated_minus_plus_options(opt: &mut cli::Opt) {
    // If --highlight-removed was passed then we should set minus and minus emph foreground to
    // "syntax", if they are still at their default values.
    let deprecated_minus_foreground_arg = if opt.deprecated_highlight_minus_lines {
        Some("syntax")
    } else {
        None
    };

    if let Some(rewritten) = _get_rewritten_minus_plus_style_string(
        &opt.minus_style,
        ("normal", "auto"),
        (
            deprecated_minus_foreground_arg,
            opt.deprecated_minus_background_color.as_deref(),
        ),
        "minus",
    ) {
        opt.minus_style = rewritten.to_string();
    }
    if let Some(rewritten) = _get_rewritten_minus_plus_style_string(
        &opt.minus_emph_style,
        ("normal", "auto"),
        (
            deprecated_minus_foreground_arg,
            opt.deprecated_minus_emph_background_color.as_deref(),
        ),
        "minus-emph",
    ) {
        opt.minus_emph_style = rewritten.to_string();
    }
    if let Some(rewritten) = _get_rewritten_minus_plus_style_string(
        &opt.plus_style,
        ("syntax", "auto"),
        (None, opt.deprecated_plus_background_color.as_deref()),
        "plus",
    ) {
        opt.plus_style = rewritten.to_string();
    }
    if let Some(rewritten) = _get_rewritten_minus_plus_style_string(
        &opt.plus_emph_style,
        ("syntax", "auto"),
        (None, opt.deprecated_plus_emph_background_color.as_deref()),
        "plus-emph",
    ) {
        opt.plus_emph_style = rewritten.to_string();
    }
}

/// For backwards-compatibility, --{commit,file}-style box means --element-decoration-style 'box ul'.
fn rewrite_options_to_implement_deprecated_commit_and_file_style_box_option(opt: &mut cli::Opt) {
    if &opt.commit_style == "box" {
        opt.commit_decoration_style = format!("box ul {}", opt.commit_decoration_style);
        opt.commit_style.clear();
    }
    if &opt.file_style == "box" {
        opt.file_decoration_style = format!("box ul {}", opt.file_decoration_style);
        opt.file_style.clear();
    }
}

fn rewrite_options_to_implement_deprecated_hunk_style_option(opt: &mut cli::Opt) {
    // Examples of how --hunk-style was originally used are
    // --hunk-style box       => --hunk-header-decoration-style box
    // --hunk-style underline => --hunk-header-decoration-style underline
    // --hunk-style plain     => --hunk-header-decoration-style ''
    if opt.deprecated_hunk_style.is_some() {
        // As in the other cases, we only honor the deprecated option if the replacement option has
        // apparently been left at its default value.
        let hunk_header_decoration_default = "blue box";
        if opt.hunk_header_decoration_style != hunk_header_decoration_default {
            eprintln!(
                "Deprecated option --hunk-style cannot be used with --hunk-header-decoration-style. \
                 Use --hunk-header-decoration-style.");
            process::exit(1);
        }
        match opt.deprecated_hunk_style.as_deref().map(str::to_lowercase) {
            Some(attr) if attr == "plain" => opt.hunk_header_decoration_style = "".to_string(),
            Some(attr) if attr == "" => {}
            Some(attr) => opt.hunk_header_decoration_style = attr,
            None => {}
        }
        opt.deprecated_hunk_style = None;
    }
}

fn _get_rewritten_commit_file_hunk_header_style_string(
    style_default_pair: (&str, Option<&str>),
    deprecated_args_style_pair: (Option<&str>, Option<&str>),
) -> Option<String> {
    let format_style = |pair: (&str, Option<&str>)| {
        format!(
            "{}{}",
            pair.0,
            match pair.1 {
                Some(s) => format!(" {}", s),
                None => "".to_string(),
            }
        )
    };
    match deprecated_args_style_pair {
        (None, None) => None,
        deprecated_args_style_pair => Some(format_style((
            deprecated_args_style_pair.0.unwrap_or(style_default_pair.0),
            match deprecated_args_style_pair.1 {
                Some(s) => Some(s),
                None => style_default_pair.1,
            },
        ))),
    }
}

fn _get_rewritten_minus_plus_style_string(
    style: &str,
    style_default_pair: (&str, &str),
    deprecated_args_style_pair: (Option<&str>, Option<&str>),
    element_name: &str,
) -> Option<String> {
    let format_style = |pair: (&str, &str)| format!("{} {}", pair.0, pair.1);
    match (style, deprecated_args_style_pair) {
        (_, (None, None)) => None, // no rewrite
        (style, deprecated_args_style_pair) if style == format_style(style_default_pair) => {
            // TODO: We allow the deprecated argument values to have effect if
            // the style argument value is equal to its default value. This is
            // non-ideal, because the user may have explicitly supplied the
            // style argument (i.e. it might just happen to equal the default).
            Some(format_style((
                deprecated_args_style_pair.0.unwrap_or(style_default_pair.0),
                deprecated_args_style_pair.1.unwrap_or(style_default_pair.1),
            )))
        }
        (_, (_, Some(_))) => {
            eprintln!(
                "--{name}-color cannot be used with --{name}-style. \
                 Use --{name}-style=\"fg bg attr1 attr2 ...\" to set \
                 foreground color, background color, and style attributes. \
                 --{name}-color can only be used to set the background color. \
                 (It is still available for backwards-compatibility.)",
                name = element_name,
            );
            process::exit(1);
        }
        (_, (Some(_), None)) => {
            eprintln!(
                "Deprecated option --highlight-removed cannot be used with \
                 --{name}-style. Use --{name}-style=\"fg bg attr1 attr2 ...\" \
                 to set foreground color, background color, and style \
                 attributes.",
                name = element_name,
            );
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use structopt::{clap, StructOpt};

    use crate::cli;
    use crate::rewrite::apply_rewrite_rules;

    #[test]
    fn test_default_is_stable_under_rewrites() {
        let mut opt = cli::Opt::from_iter(Vec::<OsString>::new());
        let before = opt.clone();

        apply_rewrite_rules(&mut opt, clap::ArgMatches::new(), &mut None);

        assert_eq!(opt, before);
    }

    /// Since --hunk-header-decoration-style is at its default value of "box",
    /// the deprecated option is allowed to overwrite it.
    #[test]
    fn test_deprecated_hunk_style_is_rewritten() {
        let mut opt = cli::Opt::from_iter(Vec::<OsString>::new());
        opt.deprecated_hunk_style = Some("underline".to_string());
        let default = "blue box";
        assert_eq!(opt.hunk_header_decoration_style, default);
        apply_rewrite_rules(&mut opt, clap::ArgMatches::new(), &mut None);
        assert_eq!(opt.deprecated_hunk_style, None);
        assert_eq!(opt.hunk_header_decoration_style, "underline");
    }

    #[test]
    fn test_deprecated_hunk_style_is_not_rewritten() {
        let mut opt = cli::Opt::from_iter(Vec::<OsString>::new());
        opt.deprecated_hunk_style = Some("".to_string());
        let default = "blue box";
        assert_eq!(opt.hunk_header_decoration_style, default);
        apply_rewrite_rules(&mut opt, clap::ArgMatches::new(), &mut None);
        assert_eq!(opt.deprecated_hunk_style, None);
        assert_eq!(opt.hunk_header_decoration_style, default);
    }
}
