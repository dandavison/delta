/// This module applies rewrite rules to the command line options, in order to
/// 1. Express deprecated usages in the new non-deprecated form
/// 2. Implement options such as --color-only which are defined to be equivalent to some set of
///    other options.
use std::process;

use crate::cli::{self, extract_special_attribute, unreachable};

pub fn apply_rewrite_rules(opt: &mut cli::Opt) {
    _rewrite_style_strings_to_honor_deprecated_minus_plus_options(opt);
    _rewrite_options_to_implement_deprecated_commit_file_hunk_header_options(opt);
    _rewrite_options_to_implement_color_only(opt);
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use structopt::StructOpt;

    use crate::cli;
    use crate::rewrite::apply_rewrite_rules;

    #[test]
    fn test_default_is_stable_under_rewrites() {
        let mut opt = cli::Opt::from_iter(Vec::<OsString>::new());
        let before = opt.clone();

        apply_rewrite_rules(&mut opt);

        assert_eq!(opt, before);
    }

    #[test]
    fn test_box_is_rewritten_as_decoration_attribute_1() {
        let mut opt = cli::Opt::from_iter(Vec::<OsString>::new());
        opt.commit_style = "box".to_string();
        assert_eq!(opt.commit_style, "box");
        assert_eq!(opt.commit_decoration_style, "");

        apply_rewrite_rules(&mut opt);

        assert_eq!(opt.commit_style, "");
        assert_eq!(opt.commit_decoration_style, "box");
    }

    #[test]
    fn test_box_is_rewritten_as_decoration_attribute_2() {
        let mut opt = cli::Opt::from_iter(Vec::<OsString>::new());
        opt.commit_style = "green box red".to_string();
        assert_eq!(opt.commit_style, "green box red");
        assert_eq!(opt.commit_decoration_style, "");

        apply_rewrite_rules(&mut opt);

        assert_eq!(opt.commit_style, "green red");
        assert_eq!(opt.commit_decoration_style, "box");
    }

    #[test]
    fn test_deprecated_commit_color_option_is_rewritten_as_style() {
        let mut opt = cli::Opt::from_iter(Vec::<OsString>::new());
        let default = "yellow";
        opt.deprecated_commit_color = Some("red".to_string());
        assert_eq!(opt.commit_style, default);

        apply_rewrite_rules(&mut opt);

        assert_eq!(opt.commit_style, "red");
        assert_eq!(opt.deprecated_commit_color, None);
    }

    #[test]
    fn test_deprecated_file_color_option_is_rewritten_as_style() {
        let mut opt = cli::Opt::from_iter(Vec::<OsString>::new());
        let default = "blue";
        opt.deprecated_file_color = Some("red".to_string());
        assert_eq!(opt.file_style, default);

        apply_rewrite_rules(&mut opt);

        assert_eq!(opt.file_style, "red");
        assert_eq!(opt.deprecated_file_color, None);
    }

    /// Since --hunk-header-decoration-style is at its default value of "box",
    /// the deprecated option is allowed to overwrite it.
    #[test]
    fn test_deprecated_hunk_style_is_rewritten() {
        let mut opt = cli::Opt::from_iter(Vec::<OsString>::new());
        opt.deprecated_hunk_style = Some("underline".to_string());
        let default = "blue box";
        assert_eq!(opt.hunk_header_decoration_style, default);
        apply_rewrite_rules(&mut opt);
        assert_eq!(opt.deprecated_hunk_style, None);
        assert_eq!(opt.hunk_header_decoration_style, "underline");
    }

    #[test]
    fn test_deprecated_hunk_style_is_not_rewritten() {
        let mut opt = cli::Opt::from_iter(Vec::<OsString>::new());
        opt.deprecated_hunk_style = Some("".to_string());
        let default = "blue box";
        assert_eq!(opt.hunk_header_decoration_style, default);
        apply_rewrite_rules(&mut opt);
        assert_eq!(opt.deprecated_hunk_style, None);
        assert_eq!(opt.hunk_header_decoration_style, default);
    }
}

/// Implement --color-only
fn _rewrite_options_to_implement_color_only(opt: &mut cli::Opt) {
    if opt.color_only {
        opt.keep_plus_minus_markers = true;
        opt.tab_width = 0;
        opt.commit_decoration_style = "".to_string();
        opt.file_decoration_style = "".to_string();
        opt.hunk_header_decoration_style = "".to_string();
    }
}

fn _rewrite_options_to_implement_deprecated_commit_file_hunk_header_options(opt: &mut cli::Opt) {
    _rewrite_options_to_implement_deprecated_decoration_style_attributes_in_style_string(opt);
    _rewrite_options_to_implement_deprecated_commit_file_hunk_header_color_options(opt);
    _rewrite_options_to_implement_deprecated_hunk_style_option(opt);
}

/// Honor deprecated arguments by rewriting the canonical --*-style arguments if appropriate.
// TODO: How to avoid repeating the default values for style options here and in
// the structopt definition?
fn _rewrite_style_strings_to_honor_deprecated_minus_plus_options(opt: &mut cli::Opt) {
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

fn _rewrite_options_to_implement_deprecated_decoration_style_attributes_in_style_string(
    opt: &mut cli::Opt,
) {
    // --commit-decoration
    if let Some((rewritten, decoration_style)) =
        _get_rewritten_decoration_style_string_with_special_attribute(
            &opt.commit_style,
            &opt.commit_decoration_style,
            "",
            "commit",
        )
    {
        opt.commit_decoration_style = decoration_style;
        opt.commit_style = rewritten;
    }
    // --file-decoration
    if let Some((rewritten, decoration_style)) =
        _get_rewritten_decoration_style_string_with_special_attribute(
            &opt.file_style,
            &opt.file_decoration_style,
            "underline",
            "file",
        )
    {
        opt.file_decoration_style = decoration_style;
        opt.file_style = rewritten;
    }
    // --hunk-header-decoration
    if let Some((rewritten, decoration_style)) =
        _get_rewritten_decoration_style_string_with_special_attribute(
            &opt.hunk_header_style,
            &opt.hunk_header_decoration_style,
            "box",
            "hunk-header",
        )
    {
        opt.hunk_header_decoration_style = decoration_style;
        opt.hunk_header_style = rewritten;
    }
}

// TODO: How to avoid repeating the default values for style options here and in
// the structopt definition?
fn _rewrite_options_to_implement_deprecated_commit_file_hunk_header_color_options(
    opt: &mut cli::Opt,
) {
    if let Some(rewritten) = _get_rewritten_commit_file_hunk_header_style_string(
        &opt.commit_style,
        ("yellow", None),
        (opt.deprecated_commit_color.as_deref(), None),
        "commit",
    ) {
        opt.commit_style = rewritten.to_string();
        opt.deprecated_commit_color = None;
    }

    if let Some(rewritten) = _get_rewritten_commit_file_hunk_header_style_string(
        &opt.file_style,
        ("blue", None),
        (opt.deprecated_file_color.as_deref(), None),
        "file",
    ) {
        opt.file_style = rewritten.to_string();
        opt.deprecated_file_color = None;
    }

    if let Some(rewritten) = _get_rewritten_commit_file_hunk_header_style_string(
        &opt.hunk_header_style,
        ("blue", None),
        (opt.deprecated_hunk_color.as_deref(), None),
        "hunk",
    ) {
        opt.hunk_header_style = rewritten.to_string();
        opt.deprecated_hunk_color = None;
    }
}

fn _rewrite_options_to_implement_deprecated_hunk_style_option(opt: &mut cli::Opt) {
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

/// A special decoration style attribute (one of 'box', 'underline', 'plain', and 'omit') is
/// currently allowed to be supplied in the style string. This is deprecated, but supported for
/// backwards-compatibility: the non-deprecated usage is for one of those special attributes to be
/// supplied in the decoration style string. If the deprecated form has been used, then this
/// function returns a (new_style_string, new_decoration_style_string) tuple, which should be used
/// to rewrite the comamnd line options in non-deprecated form.
fn _get_rewritten_decoration_style_string_with_special_attribute(
    style: &str,
    decoration_style: &str,
    decoration_style_default: &str,
    element_name: &str,
) -> Option<(String, String)> {
    match extract_special_attribute(style) {
        (style, Some(special_attribute)) => {
            if decoration_style == decoration_style_default {
                Some((style, special_attribute))
            } else {
                eprintln!(
                    "Special attribute {attr_name} may not be used in --{element_name}-style \
                     if you are also using --commit-decoration-style.",
                    attr_name = special_attribute,
                    element_name = element_name
                );
                process::exit(1);
            }
        }
        _ => None,
    }
}

fn _get_rewritten_commit_file_hunk_header_style_string(
    style: &str,
    style_default_pair: (&str, Option<&str>),
    deprecated_args_style_pair: (Option<&str>, Option<&str>),
    element_name: &str,
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
    match (style, deprecated_args_style_pair) {
        (_, (None, None)) => None, // no rewrite
        (style, deprecated_args_style_pair) if style == format_style(style_default_pair) => {
            // TODO: We allow the deprecated argument values to have effect if
            // the style argument value is equal to its default value. This is
            // non-ideal, because the user may have explicitly supplied the
            // style argument (i.e. it might just happen to equal the default).
            Some(format_style((
                deprecated_args_style_pair.0.unwrap_or(style_default_pair.0),
                match deprecated_args_style_pair.1 {
                    Some(s) => Some(s),
                    None => style_default_pair.1,
                },
            )))
        }
        (_, (Some(_), None)) => {
            eprintln!(
                "--{name}-color cannot be used with --{name}-style. \
                 Use --{name}-style=\"fg bg attr1 attr2 ...\" to set \
                 foreground color, background color, and style attributes. \
                 --{name}-color can only be used to set the foreground color. \
                 (It is still available for backwards-compatibility.)",
                name = element_name,
            );
            process::exit(1);
        }
        _ => unreachable(&format!(
            "Unexpected value deprecated_args_style_pair={:?} in \
             _get_rewritten_commit_file_hunk_header_style_string.",
            deprecated_args_style_pair,
        )),
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
