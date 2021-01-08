/// This module applies rewrite rules to the command line options, in order to
/// 1. Express deprecated usages in the new non-deprecated form
/// 2. Implement options such as --raw which are defined to be equivalent to some set of
///    other options.
use std::process;

use structopt::clap;

use crate::cli;
use crate::config::user_supplied_option;

pub fn apply_rewrite_rules(opt: &mut cli::Opt, arg_matches: &clap::ArgMatches) {
    rewrite_style_strings_to_honor_deprecated_minus_plus_options(opt);
    rewrite_options_to_implement_deprecated_commit_and_file_style_box_option(opt);
    rewrite_options_to_implement_deprecated_hunk_style_option(opt);
    rewrite_options_to_implement_deprecated_theme_option(opt, arg_matches);
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
        opt.minus_style = rewritten;
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
        opt.minus_emph_style = rewritten;
    }
    if let Some(rewritten) = _get_rewritten_minus_plus_style_string(
        &opt.plus_style,
        ("syntax", "auto"),
        (None, opt.deprecated_plus_background_color.as_deref()),
        "plus",
    ) {
        opt.plus_style = rewritten;
    }
    if let Some(rewritten) = _get_rewritten_minus_plus_style_string(
        &opt.plus_emph_style,
        ("syntax", "auto"),
        (None, opt.deprecated_plus_emph_background_color.as_deref()),
        "plus-emph",
    ) {
        opt.plus_emph_style = rewritten;
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
            Some(attr) if attr.is_empty() => {}
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
    use crate::options::rewrite::apply_rewrite_rules;

    /// Since --hunk-header-decoration-style is at its default value of "box",
    /// the deprecated option is allowed to overwrite it.
    #[test]
    fn test_deprecated_hunk_style_is_rewritten() {
        let mut opt = cli::Opt::from_iter(Vec::<OsString>::new());
        opt.deprecated_hunk_style = Some("underline".to_string());
        let default = "blue box";
        assert_eq!(opt.hunk_header_decoration_style, default);
        apply_rewrite_rules(&mut opt, &clap::ArgMatches::new());
        assert_eq!(opt.deprecated_hunk_style, None);
        assert_eq!(opt.hunk_header_decoration_style, "underline");
    }

    #[test]
    fn test_deprecated_hunk_style_is_not_rewritten() {
        let mut opt = cli::Opt::from_iter(Vec::<OsString>::new());
        opt.deprecated_hunk_style = Some("".to_string());
        let default = "blue box";
        assert_eq!(opt.hunk_header_decoration_style, default);
        apply_rewrite_rules(&mut opt, &clap::ArgMatches::new());
        assert_eq!(opt.deprecated_hunk_style, None);
        assert_eq!(opt.hunk_header_decoration_style, default);
    }
}
