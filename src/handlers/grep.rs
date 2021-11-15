// TODO
// Bad parsing: "etc/examples/119-within-line-edits:4:repo=$(mktemp -d)"
// Parsing "Makefile"
// Inspect process tree once
use std::convert::{TryFrom, TryInto};

use lazy_static::lazy_static;
use regex::Regex;
use unicode_segmentation::UnicodeSegmentation;

use crate::ansi;
use crate::delta::{State, StateMachine};
use crate::handlers;
use crate::paint::{self, BgShouldFill, StyleSectionSpecifier};
use crate::style::Style;
use crate::utils;

struct GrepOutputConfig {
    add_navigate_marker_to_matches: bool,
    render_context_header_as_hunk_header: bool,
    pad_line_number: bool,
}

impl<'a> StateMachine<'a> {
    /// If this is a line of git grep output then render it accordingly. If this
    /// is the first grep line, then set the syntax-highlighter language.
    pub fn handle_grep_line(&mut self) -> std::io::Result<bool> {
        self.painter.emit()?;
        let mut handled_line = false;

        // TODO: It should be possible to eliminate some of the .clone()s and
        // .to_owned()s.
        let (_previous_file, repeat_grep_line, try_parse) = match &self.state {
            State::Grep(file, repeat_grep_line) => {
                (Some(file.as_str()), repeat_grep_line.clone(), true)
            }
            State::Unknown => (None, None, true),
            _ => (None, None, false),
        };
        if try_parse {
            if let Some(grep) = parse_grep_line(&self.line) {
                let output_config = make_output_config();

                // Emit syntax-highlighted code
                // TODO: Determine the language less frequently, e.g. only when the file changes.
                if let Some(lang) = handlers::file_meta::get_extension(grep.file)
                    .or_else(|| self.config.default_language.as_deref())
                {
                    self.painter.set_syntax(Some(lang));
                    self.painter.set_highlighter();
                }
                self.state = State::Grep(grep.file.to_owned(), repeat_grep_line);

                match (
                    &grep.line_type,
                    output_config.render_context_header_as_hunk_header,
                ) {
                    // Emit context header line
                    (LineType::ContextHeader, true) => handlers::hunk_header::write_hunk_header(
                        grep.code,
                        &[(grep.line_number.unwrap_or(0), 0)],
                        &mut self.painter,
                        &self.line,
                        grep.file,
                        self.config,
                    )?,
                    _ => {
                        if self.config.navigate {
                            write!(
                                self.painter.writer,
                                "{}",
                                match (
                                    &grep.line_type,
                                    output_config.add_navigate_marker_to_matches
                                ) {
                                    (LineType::Match, true) => "• ",
                                    (_, true) => "  ",
                                    _ => "",
                                }
                            )?
                        }
                        // Emit file & line-number
                        write!(
                            self.painter.writer,
                            "{}",
                            paint::paint_file_path_with_line_number(
                                grep.line_number,
                                grep.file,
                                output_config.pad_line_number,
                                true,
                                Some(self.config.grep_match_file_style),
                                Some(self.config.grep_match_line_number_style),
                                self.config
                            )
                        )?;

                        // Emit code line
                        let code_style_sections = if matches!(&grep.line_type, LineType::Match) {
                            // HACK: We need tabs expanded, and we need the &str
                            // passed to `get_code_style_sections` to live long
                            // enough.
                            self.raw_line = self.painter.expand_tabs(self.raw_line.graphemes(true));
                            get_code_style_sections(
                                &self.raw_line,
                                self.config.grep_match_style,
                                &grep,
                            )
                            .unwrap_or(StyleSectionSpecifier::Style(self.config.zero_style))
                        } else {
                            StyleSectionSpecifier::Style(self.config.zero_style)
                        };
                        self.painter.syntax_highlight_and_paint_line(
                            &format!("{}\n", grep.code),
                            code_style_sections,
                            self.state.clone(),
                            BgShouldFill::default(),
                        )
                    }
                }
                handled_line = true
            }
        }
        Ok(handled_line)
    }
}

// Return style sections describing colors received from git.
fn get_code_style_sections<'b>(
    raw_line: &'b str,
    match_style: Style,
    grep: &GrepLine,
) -> Option<StyleSectionSpecifier<'b>> {
    if let Some(raw_code_start) = ansi::ansi_preserving_index(
        raw_line,
        match grep.line_number {
            Some(n) => format!("{}:{}:", grep.file, n).len(),
            None => grep.file.len() + 1,
        },
    ) {
        let non_match_style = Style {
            is_syntax_highlighted: true,
            ..Style::new()
        };

        let match_style_sections = ansi::parse_style_sections(&raw_line[raw_code_start..])
            .iter()
            .map(|(ansi_term_style, s)| {
                if ansi_term_style.foreground.is_some() {
                    (match_style, *s)
                } else {
                    (non_match_style, *s)
                }
            })
            .collect();
        Some(StyleSectionSpecifier::StyleSections(match_style_sections))
    } else {
        None
    }
}

fn make_output_config() -> GrepOutputConfig {
    match utils::parent_command_options() {
        Some((longs, shorts)) if shorts.contains("-W") || longs.contains("--function-context") => {
            // --function-context is in effect: i.e. the entire function is
            // being displayed. In that case we don't render the first line as a
            // header, since the second line is the true next line, and it will
            // be more readable to have these displayed normally. We do add the
            // navigate marker, since match lines will be surrounded by (many)
            // non-match lines. And, since we are printing (many) successive lines
            // of code, we pad line numbers <100 in order to maintain code
            // alignment up to line 9999.
            GrepOutputConfig {
                render_context_header_as_hunk_header: false,
                add_navigate_marker_to_matches: true,
                pad_line_number: true,
            }
        }
        Some((longs, shorts)) if shorts.contains("-p") || longs.contains("--show-function") => {
            // --show-function is in effect, i.e. the function header is being
            // displayed, along with matches within the function. Therefore we
            // render the first line as a header, but we do not add the navigate
            // marker, since all non-header lines are matches.
            GrepOutputConfig {
                render_context_header_as_hunk_header: true,
                add_navigate_marker_to_matches: false,
                pad_line_number: false,
            }
        }
        _ => GrepOutputConfig {
            render_context_header_as_hunk_header: true,
            add_navigate_marker_to_matches: false,
            pad_line_number: false,
        },
    }
}

#[derive(Debug, PartialEq)]
pub struct GrepLine<'a> {
    pub file: &'a str,
    pub line_number: Option<usize>,
    pub line_type: LineType,
    pub code: &'a str,
}

#[derive(Debug, PartialEq)]
pub enum LineType {
    ContextHeader,
    Match,
    NoMatch,
}

// See tests for example grep lines
lazy_static! {
    static ref GREP_LINE_REGEX: Regex = Regex::new(
        r"(?x)
^
([^\ ].*?\.[^-.=: ]+)    # 1. file name (TODO: it must have an extension)
(?:
    [-=:]([0-9]+)        # 2. optional line number
)?
([-=:])                  # 3. line-type marker
(.*)                     # 4. code (i.e. line contents)
$
"
    )
    .unwrap();
}

pub fn parse_grep_line(line: &str) -> Option<GrepLine> {
    let caps = GREP_LINE_REGEX.captures(line)?;
    let file = caps.get(1).unwrap().as_str();
    let line_number = caps.get(2).map(|m| m.as_str().parse().ok()).flatten();
    let line_type = caps.get(3).map(|m| m.as_str()).try_into().ok()?;
    let code = caps.get(4).unwrap().as_str();

    Some(GrepLine {
        file,
        line_number,
        line_type,
        code,
    })
}

impl TryFrom<Option<&str>> for LineType {
    type Error = ();
    fn try_from(from: Option<&str>) -> Result<Self, Self::Error> {
        match from {
            Some(marker) if marker == ":" => Ok(LineType::Match),
            Some(marker) if marker == "-" => Ok(LineType::NoMatch),
            Some(marker) if marker == "=" => Ok(LineType::ContextHeader),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::handlers::grep::{parse_grep_line, GrepLine, LineType};

    #[test]
    fn test_parse_grep_line() {
        // git grep MinusPlus
        assert_eq!(
            parse_grep_line("src/config.rs:use crate::minusplus::MinusPlus;"),
            Some(GrepLine {
                file: "src/config.rs",
                line_number: None,
                line_type: LineType::Match,
                code: "use crate::minusplus::MinusPlus;",
            })
        );

        // git grep -n MinusPlus    [with line numbers]
        assert_eq!(
            parse_grep_line("src/config.rs:21:use crate::minusplus::MinusPlus;"),
            Some(GrepLine {
                file: "src/config.rs",
                line_number: Some(21),
                line_type: LineType::Match,
                code: "use crate::minusplus::MinusPlus;",
            })
        );

        // git grep -W MinusPlus    [with function context]
        assert_eq!(
            parse_grep_line("src/config.rs=pub struct Config {"), // match
            Some(GrepLine {
                file: "src/config.rs",
                line_number: None,
                line_type: LineType::ContextHeader,
                code: "pub struct Config {",
            })
        );
        assert_eq!(
            parse_grep_line("src/config.rs-    pub available_terminal_width: usize,"),
            Some(GrepLine {
                file: "src/config.rs",
                line_number: None,
                line_type: LineType::NoMatch,
                code: "    pub available_terminal_width: usize,",
            })
        );
        assert_eq!(
            parse_grep_line(
                "src/config.rs:    pub line_numbers_style_minusplus: MinusPlus<Style>,"
            ),
            Some(GrepLine {
                file: "src/config.rs",
                line_number: None,
                line_type: LineType::Match,
                code: "    pub line_numbers_style_minusplus: MinusPlus<Style>,",
            })
        );

        // git grep -n -W MinusPlus [with line numbers and function context]
        assert_eq!(
            parse_grep_line("src/config.rs=57=pub struct Config {"),
            Some(GrepLine {
                file: "src/config.rs",
                line_number: Some(57),
                line_type: LineType::ContextHeader,
                code: "pub struct Config {",
            })
        );
        assert_eq!(
            parse_grep_line("src/config.rs-58-    pub available_terminal_width: usize,"),
            Some(GrepLine {
                file: "src/config.rs",
                line_number: Some(58),
                line_type: LineType::NoMatch,
                code: "    pub available_terminal_width: usize,",
            })
        );
        assert_eq!(
            parse_grep_line(
                "src/config.rs:95:    pub line_numbers_style_minusplus: MinusPlus<Style>,"
            ),
            Some(GrepLine {
                file: "src/config.rs",
                line_number: Some(95),
                line_type: LineType::Match,
                code: "    pub line_numbers_style_minusplus: MinusPlus<Style>,",
            })
        );

        // git grep -h MinusPlus    [no file names: TODO: handle this?]
        //use crate::minusplus::MinusPlus;
    }

    #[test]
    fn test_parse_grep_line_filenames() {
        assert_eq!(
            parse_grep_line("src/con-fig.rs:use crate::minusplus::MinusPlus;"),
            Some(GrepLine {
                file: "src/con-fig.rs",
                line_number: None,
                line_type: LineType::Match,
                code: "use crate::minusplus::MinusPlus;",
            })
        );
        assert_eq!(
            parse_grep_line("src/con-fig.rs-use crate::minusplus::MinusPlus;"),
            Some(GrepLine {
                file: "src/con-fig.rs",
                line_number: None,
                line_type: LineType::NoMatch,
                code: "use crate::minusplus::MinusPlus;",
            })
        );
        assert_eq!(
            parse_grep_line("de-lta.rs-            if self.source == Source::Unknown {"),
            Some(GrepLine {
                file: "de-lta.rs",
                line_number: None,
                line_type: LineType::NoMatch,
                code: "            if self.source == Source::Unknown {",
            })
        );
        assert_eq!(
            parse_grep_line("etc/examples/119-within-line-edits:4:repo=$(mktemp -d)"),
            Some(GrepLine {
                file: "etc/examples/119-within-line-edits",
                line_number: Some(4),
                line_type: LineType::Match,
                code: "repo=$(mktemp -d)",
            })
        );
    }
}
