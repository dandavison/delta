use std::collections::{HashMap, HashSet};

use crate::cli;
use crate::color;
use crate::fatal;
use crate::git_config::GitConfigEntry;
use crate::style::{self, Style};

#[derive(Debug, Clone)]
enum StyleReference {
    Style(Style),
    Reference(String),
}

fn is_style_reference(style_string: &str) -> bool {
    style_string.ends_with("-style") && !style_string.chars().any(|c| c == ' ')
}

pub fn parse_styles(opt: &cli::Opt) -> HashMap<String, Style> {
    let mut styles: HashMap<&str, StyleReference> = HashMap::new();

    make_hunk_styles(opt, &mut styles);
    make_commit_file_hunk_header_styles(opt, &mut styles);
    make_line_number_styles(opt, &mut styles);
    make_grep_styles(opt, &mut styles);

    styles.insert(
        "inline-hint-style",
        style_from_str(&opt.inline_hint_style, None, None, opt.computed.true_color),
    );
    styles.insert(
        "git-minus-style",
        StyleReference::Style(match opt.git_config_entries.get("color.diff.old") {
            Some(GitConfigEntry::Style(s)) => Style::from_git_str(s),
            _ => *style::GIT_DEFAULT_MINUS_STYLE,
        }),
    );
    styles.insert(
        "git-plus-style",
        StyleReference::Style(match opt.git_config_entries.get("color.diff.new") {
            Some(GitConfigEntry::Style(s)) => Style::from_git_str(s),
            _ => *style::GIT_DEFAULT_PLUS_STYLE,
        }),
    );
    let mut resolved_styles = resolve_style_references(styles, opt);
    resolved_styles.get_mut("minus-emph-style").unwrap().is_emph = true;
    resolved_styles.get_mut("plus-emph-style").unwrap().is_emph = true;
    resolved_styles
}

fn resolve_style_references(
    edges: HashMap<&str, StyleReference>,
    opt: &cli::Opt,
) -> HashMap<String, Style> {
    let mut resolved_styles = HashMap::new();

    for starting_node in edges.keys() {
        if resolved_styles.contains_key(*starting_node) {
            continue;
        }
        let mut visited = HashSet::new();
        let mut node = *starting_node;
        loop {
            if !visited.insert(node) {
                #[cfg(not(test))]
                fatal(format!("Your delta styles form a cycle! {:?}", visited));
                #[cfg(test)]
                return [("__cycle__", Style::default())]
                    .iter()
                    .map(|(a, b)| (a.to_string(), *b))
                    .collect();
            }
            match &edges.get(&node) {
                Some(StyleReference::Reference(child_node)) => node = child_node,
                Some(StyleReference::Style(style)) => {
                    resolved_styles.extend(visited.iter().map(|node| (node.to_string(), *style)));
                    break;
                }
                None => {
                    if let Some(git_config) = &opt.git_config {
                        match git_config.get::<String>(&format!("delta.{}", node)) {
                            Some(s) => {
                                let style = Style::from_git_str(&s);
                                resolved_styles
                                    .extend(visited.iter().map(|node| (node.to_string(), style)));
                                break;
                            }
                            _ => fatal(format!("Style not found: {}", node)),
                        }
                    } else {
                        fatal(format!("Style not found: {}", node));
                    }
                }
            }
        }
    }
    resolved_styles
}

fn make_hunk_styles<'a>(opt: &'a cli::Opt, styles: &'a mut HashMap<&str, StyleReference>) {
    let is_light_mode = opt.computed.is_light_mode;
    let true_color = opt.computed.true_color;
    let minus_style = style_from_str(
        &opt.minus_style,
        Some(Style::from_colors(
            None,
            Some(color::get_minus_background_color_default(
                is_light_mode,
                true_color,
            )),
        )),
        None,
        true_color,
    );

    let minus_emph_style = style_from_str(
        &opt.minus_emph_style,
        Some(Style::from_colors(
            None,
            Some(color::get_minus_emph_background_color_default(
                is_light_mode,
                true_color,
            )),
        )),
        None,
        true_color,
    );

    let minus_non_emph_style = style_from_str(&opt.minus_non_emph_style, None, None, true_color);

    // The style used to highlight a removed empty line when otherwise it would be invisible due to
    // lack of background color in minus-style.
    let minus_empty_line_marker_style = style_from_str(
        &opt.minus_empty_line_marker_style,
        Some(Style::from_colors(
            None,
            Some(color::get_minus_background_color_default(
                is_light_mode,
                true_color,
            )),
        )),
        None,
        true_color,
    );

    let zero_style = style_from_str(&opt.zero_style, None, None, true_color);

    let plus_style = style_from_str(
        &opt.plus_style,
        Some(Style::from_colors(
            None,
            Some(color::get_plus_background_color_default(
                is_light_mode,
                true_color,
            )),
        )),
        None,
        true_color,
    );

    let plus_emph_style = style_from_str(
        &opt.plus_emph_style,
        Some(Style::from_colors(
            None,
            Some(color::get_plus_emph_background_color_default(
                is_light_mode,
                true_color,
            )),
        )),
        None,
        true_color,
    );

    let plus_non_emph_style = style_from_str(&opt.plus_non_emph_style, None, None, true_color);

    // The style used to highlight an added empty line when otherwise it would be invisible due to
    // lack of background color in plus-style.
    let plus_empty_line_marker_style = style_from_str(
        &opt.plus_empty_line_marker_style,
        Some(Style::from_colors(
            None,
            Some(color::get_plus_background_color_default(
                is_light_mode,
                true_color,
            )),
        )),
        None,
        true_color,
    );

    let whitespace_error_style =
        style_from_str(&opt.whitespace_error_style, None, None, true_color);

    styles.extend([
        ("minus-style", minus_style),
        ("minus-emph-style", minus_emph_style),
        ("minus-non-emph-style", minus_non_emph_style),
        (
            "minus-empty-line-marker-style",
            minus_empty_line_marker_style,
        ),
        ("zero-style", zero_style),
        ("plus-style", plus_style),
        ("plus-emph-style", plus_emph_style),
        ("plus-non-emph-style", plus_non_emph_style),
        ("plus-empty-line-marker-style", plus_empty_line_marker_style),
        ("whitespace-error-style", whitespace_error_style),
    ])
}

fn make_line_number_styles(opt: &cli::Opt, styles: &mut HashMap<&str, StyleReference>) {
    let true_color = opt.computed.true_color;
    let line_numbers_left_style =
        style_from_str(&opt.line_numbers_left_style, None, None, true_color);

    let line_numbers_minus_style =
        style_from_str(&opt.line_numbers_minus_style, None, None, true_color);

    let line_numbers_zero_style =
        style_from_str(&opt.line_numbers_zero_style, None, None, true_color);

    let line_numbers_plus_style =
        style_from_str(&opt.line_numbers_plus_style, None, None, true_color);

    let line_numbers_right_style =
        style_from_str(&opt.line_numbers_right_style, None, None, true_color);

    styles.extend([
        ("line-numbers-minus-style", line_numbers_minus_style),
        ("line-numbers-zero-style", line_numbers_zero_style),
        ("line-numbers-plus-style", line_numbers_plus_style),
        ("line-numbers-left-style", line_numbers_left_style),
        ("line-numbers-right-style", line_numbers_right_style),
    ])
}

fn make_commit_file_hunk_header_styles(opt: &cli::Opt, styles: &mut HashMap<&str, StyleReference>) {
    let true_color = opt.computed.true_color;
    styles.extend([
        ("commit-style",
            style_from_str_with_handling_of_special_decoration_attributes_and_respecting_deprecated_foreground_color_arg(
                &opt.commit_style,
                None,
                Some(&opt.commit_decoration_style),
                opt.deprecated_commit_color.as_deref(),
                true_color,
            )
        ),
        ("file-style",
            style_from_str_with_handling_of_special_decoration_attributes_and_respecting_deprecated_foreground_color_arg(
                &opt.file_style,
                None,
                Some(&opt.file_decoration_style),
                opt.deprecated_file_color.as_deref(),
                true_color,
            )
        ),
        ("hunk-header-style",
            style_from_str_with_handling_of_special_decoration_attributes_and_respecting_deprecated_foreground_color_arg(
                &opt.hunk_header_style,
                None,
                Some(&opt.hunk_header_decoration_style),
                opt.deprecated_hunk_color.as_deref(),
                true_color,
            )
        ),
        ("hunk-header-file-style",
            style_from_str_with_handling_of_special_decoration_attributes(
                &opt.hunk_header_file_style,
                None,
                None,
                true_color,
            )
        ),
        ("hunk-header-line-number-style",
            style_from_str_with_handling_of_special_decoration_attributes(
                &opt.hunk_header_line_number_style,
                None,
                None,
                true_color,
            )
        ),
    ]);
}

fn make_grep_styles(opt: &cli::Opt, styles: &mut HashMap<&str, StyleReference>) {
    styles.extend([
        (
            "grep-match-line-style",
            if let Some(s) = &opt.grep_match_line_style {
                style_from_str(s, None, None, opt.computed.true_color)
            } else {
                StyleReference::Reference("zero-style".to_owned())
            },
        ),
        (
            "grep-match-word-style",
            if let Some(s) = &opt.grep_match_word_style {
                style_from_str(s, None, None, opt.computed.true_color)
            } else {
                StyleReference::Reference("plus-emph-style".to_owned())
            },
        ),
        (
            "grep-context-line-style",
            if let Some(s) = &opt.grep_context_line_style {
                style_from_str(s, None, None, opt.computed.true_color)
            } else {
                StyleReference::Reference("zero-style".to_owned())
            },
        ),
        (
            "grep-file-style",
            if let Some(s) = &opt.grep_file_style {
                style_from_str(s, None, None, opt.computed.true_color)
            } else {
                StyleReference::Reference("hunk-header-file-style".to_owned())
            },
        ),
        (
            "grep-line-number-style",
            if let Some(s) = &opt.grep_line_number_style {
                style_from_str(s, None, None, opt.computed.true_color)
            } else {
                StyleReference::Reference("hunk-header-line-number-style".to_owned())
            },
        ),
    ])
}

fn style_from_str(
    style_string: &str,
    default: Option<Style>,
    decoration_style_string: Option<&str>,
    true_color: bool,
) -> StyleReference {
    if is_style_reference(style_string) {
        StyleReference::Reference(style_string.to_owned())
    } else {
        StyleReference::Style(Style::from_str(
            style_string,
            default,
            decoration_style_string,
            true_color,
        ))
    }
}

fn style_from_str_with_handling_of_special_decoration_attributes(
    style_string: &str,
    default: Option<Style>,
    decoration_style_string: Option<&str>,
    true_color: bool,
) -> StyleReference {
    if is_style_reference(style_string) {
        StyleReference::Reference(style_string.to_owned())
    } else {
        StyleReference::Style(
            Style::from_str_with_handling_of_special_decoration_attributes(
                style_string,
                default,
                decoration_style_string,
                true_color,
            ),
        )
    }
}

fn style_from_str_with_handling_of_special_decoration_attributes_and_respecting_deprecated_foreground_color_arg(
    style_string: &str,
    default: Option<Style>,
    decoration_style_string: Option<&str>,
    deprecated_foreground_color_arg: Option<&str>,
    true_color: bool,
) -> StyleReference {
    if is_style_reference(style_string) {
        StyleReference::Reference(style_string.to_owned())
    } else {
        StyleReference::Style(Style::from_str_with_handling_of_special_decoration_attributes_and_respecting_deprecated_foreground_color_arg(
            style_string,
            default,
            decoration_style_string,
            deprecated_foreground_color_arg,
            true_color,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::integration_test_utils;

    fn resolve_style_references(edges: HashMap<&str, StyleReference>) -> HashMap<String, Style> {
        let opt = integration_test_utils::make_options_from_args(&[]);
        super::resolve_style_references(edges, &opt)
    }

    #[test]
    fn test_resolve_style_references_1() {
        let style_1 = Style::default();
        let mut style_2 = Style::default();
        style_2.is_syntax_highlighted = !style_1.is_syntax_highlighted;

        let edges: HashMap<&str, StyleReference> = [
            ("a", StyleReference::Style(style_1)),
            ("b", StyleReference::Reference("c".to_string())),
            ("c", StyleReference::Style(style_2)),
        ]
        .iter()
        .map(|(a, b)| (*a, b.clone()))
        .collect();

        let expected = [("a", style_1), ("b", style_2), ("c", style_2)]
            .iter()
            .map(|(a, b)| (a.to_string(), *b))
            .collect();

        assert_eq!(resolve_style_references(edges), expected);
    }

    #[test]
    fn test_resolve_style_references_2() {
        let style_1 = Style::default();
        let mut style_2 = Style::default();
        style_2.is_syntax_highlighted = !style_1.is_syntax_highlighted;

        let edges: HashMap<&str, StyleReference> = [
            ("a", StyleReference::Reference("b".to_string())),
            ("b", StyleReference::Reference("c".to_string())),
            ("c", StyleReference::Style(style_1)),
            ("d", StyleReference::Reference("b".to_string())),
            ("e", StyleReference::Reference("a".to_string())),
            ("f", StyleReference::Style(style_2)),
            ("g", StyleReference::Reference("f".to_string())),
            ("h", StyleReference::Reference("g".to_string())),
            ("i", StyleReference::Reference("g".to_string())),
        ]
        .iter()
        .map(|(a, b)| (*a, b.clone()))
        .collect();

        let expected = [
            ("a", style_1),
            ("b", style_1),
            ("c", style_1),
            ("d", style_1),
            ("e", style_1),
            ("f", style_2),
            ("g", style_2),
            ("h", style_2),
            ("i", style_2),
        ]
        .iter()
        .map(|(a, b)| (a.to_string(), *b))
        .collect();

        assert_eq!(resolve_style_references(edges), expected);
    }

    #[test]
    fn test_resolve_style_references_cycle() {
        let edges: HashMap<&str, StyleReference> = [
            ("a", StyleReference::Reference("b".to_string())),
            ("b", StyleReference::Reference("c".to_string())),
            ("c", StyleReference::Reference("a".to_string())),
        ]
        .iter()
        .map(|(a, b)| (*a, b.clone()))
        .collect();

        assert_eq!(
            resolve_style_references(edges).keys().next().unwrap(),
            "__cycle__"
        );
    }
}
