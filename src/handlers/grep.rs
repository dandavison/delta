use std::borrow::Cow;

use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use unicode_segmentation::UnicodeSegmentation;

use crate::ansi;
use crate::delta::{State, StateMachine};
use crate::handlers::{self, ripgrep_json};
use crate::paint::{self, expand_tabs, BgShouldFill, StyleSectionSpecifier};
use crate::style::Style;
use crate::utils::process;

#[derive(Debug, PartialEq, Eq)]
pub struct GrepLine<'b> {
    pub path: Cow<'b, str>,
    pub line_number: Option<usize>,
    pub line_type: LineType,
    pub code: Cow<'b, str>,
    pub submatches: Option<Vec<(usize, usize)>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineType {
    ContextHeader,
    Context,
    Match,
    Ignore,
}

struct GrepOutputConfig {
    add_navigate_marker_to_matches: bool,
    render_context_header_as_hunk_header: bool,
    pad_line_number: bool,
}

lazy_static! {
    static ref OUTPUT_CONFIG: GrepOutputConfig = make_output_config();
}

impl<'a> StateMachine<'a> {
    // If this is a line of git grep output then render it accordingly.
    pub fn handle_grep_line(&mut self) -> std::io::Result<bool> {
        self.painter.emit()?;
        let mut handled_line = false;

        let try_parse = matches!(&self.state, State::Grep | State::Unknown);

        if try_parse {
            if let Some(mut grep_line) = parse_grep_line(&self.line) {
                if matches!(grep_line.line_type, LineType::Ignore) {
                    handled_line = true;
                    return Ok(handled_line);
                }

                // Emit syntax-highlighted code
                // TODO: Determine the language less frequently, e.g. only when the file changes.
                if let Some(lang) = handlers::diff_header::get_extension(&grep_line.path)
                    .or(self.config.default_language.as_deref())
                {
                    self.painter.set_syntax(Some(lang));
                    self.painter.set_highlighter();
                }
                self.state = State::Grep;

                match (
                    &grep_line.line_type,
                    OUTPUT_CONFIG.render_context_header_as_hunk_header,
                ) {
                    // Emit context header line
                    (LineType::ContextHeader, true) => handlers::hunk_header::write_hunk_header(
                        &grep_line.code,
                        &[(grep_line.line_number.unwrap_or(0), 0)],
                        &mut self.painter,
                        &self.line,
                        &grep_line.path,
                        self.config,
                    )?,
                    _ => {
                        if self.config.navigate {
                            write!(
                                self.painter.writer,
                                "{}",
                                match (
                                    &grep_line.line_type,
                                    OUTPUT_CONFIG.add_navigate_marker_to_matches
                                ) {
                                    (LineType::Match, true) => "• ",
                                    (_, true) => "  ",
                                    _ => "",
                                }
                            )?
                        }
                        // Emit file & line-number
                        let separator = if self.config.grep_separator_symbol == "keep" {
                            // grep, rg, and git grep use ":" for matching lines
                            // and "-" for non-matching lines (and `git grep -W`
                            // uses "=" for a context header line).
                            match grep_line.line_type {
                                LineType::Match => ":",
                                LineType::Context => "-",
                                LineType::ContextHeader => "=",
                                LineType::Ignore => "",
                            }
                        } else {
                            // But ":" results in a "file/path:number:"
                            // construct that terminal emulators are more likely
                            // to recognize and render as a clickable link. If
                            // navigate is enabled then there is already a good
                            // visual indicator of match lines (in addition to
                            // the grep-match-style highlighting) and so we use
                            // ":" for matches and non-matches alike.
                            &self.config.grep_separator_symbol
                        };
                        write!(
                            self.painter.writer,
                            "{}",
                            paint::paint_file_path_with_line_number(
                                grep_line.line_number,
                                &grep_line.path,
                                OUTPUT_CONFIG.pad_line_number,
                                separator,
                                true,
                                Some(self.config.grep_file_style),
                                Some(self.config.grep_line_number_style),
                                self.config
                            )
                        )?;

                        // Emit code line
                        let code_style_sections =
                            match (&grep_line.line_type, &grep_line.submatches) {
                                (LineType::Match, Some(submatches)) => {
                                    // We expand tabs at this late stage because
                                    // the tabs are escaped in the JSON, so
                                    // expansion must come after JSON parsing.
                                    // (At the time of writing, we are in this
                                    // arm iff we are handling `ripgrep --json`
                                    // output.)
                                    grep_line.code = paint::expand_tabs(
                                        grep_line.code.graphemes(true),
                                        self.config.tab_width,
                                    )
                                    .into();
                                    make_style_sections(
                                        &grep_line.code,
                                        submatches,
                                        self.config.grep_match_word_style,
                                        self.config.grep_match_line_style,
                                    )
                                }
                                (LineType::Match, None) => {
                                    // HACK: We need tabs expanded, and we need
                                    // the &str passed to
                                    // `get_code_style_sections` to live long
                                    // enough. But at the point it is guaranteed
                                    // that this handler is going to handle this
                                    // line, so mutating it is acceptable.
                                    self.raw_line = expand_tabs(
                                        self.raw_line.graphemes(true),
                                        self.config.tab_width,
                                    );
                                    get_code_style_sections(
                                        &self.raw_line,
                                        self.config.grep_match_word_style,
                                        self.config.grep_match_line_style,
                                        &grep_line,
                                    )
                                    .unwrap_or(
                                        StyleSectionSpecifier::Style(
                                            self.config.grep_match_line_style,
                                        ),
                                    )
                                }
                                _ => StyleSectionSpecifier::Style(
                                    self.config.grep_context_line_style,
                                ),
                            };
                        self.painter.syntax_highlight_and_paint_line(
                            &format!("{}\n", grep_line.code),
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

fn make_style_sections<'a>(
    line: &'a str,
    submatches: &[(usize, usize)],
    match_style: Style,
    non_match_style: Style,
) -> StyleSectionSpecifier<'a> {
    let mut sections = Vec::new();
    let mut curr = 0;
    for (start_, end_) in submatches {
        let (start, end) = (*start_, *end_);
        if start > curr {
            sections.push((non_match_style, &line[curr..start]))
        };
        sections.push((match_style, &line[start..end]));
        curr = end;
    }
    if curr < line.len() {
        sections.push((non_match_style, &line[curr..]))
    }
    StyleSectionSpecifier::StyleSections(sections)
}

// Return style sections describing colors received from git.
fn get_code_style_sections<'b>(
    raw_line: &'b str,
    match_style: Style,
    non_match_style: Style,
    grep: &GrepLine,
) -> Option<StyleSectionSpecifier<'b>> {
    if let Some(prefix_end) = ansi::ansi_preserving_index(
        raw_line,
        match grep.line_number {
            Some(n) => format!("{}:{}:", grep.path, n).len() - 1,
            None => grep.path.len(),
        },
    ) {
        let match_style_sections = ansi::parse_style_sections(&raw_line[(prefix_end + 1)..])
            .iter()
            .map(|(ansi_term_style, s)| {
                if ansi_term_style.is_bold
                    && ansi_term_style.foreground == Some(ansi_term::Colour::Red)
                {
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
    match &*process::calling_process() {
        process::CallingProcess::GitGrep(command_line)
            if command_line.short_options.contains("-W")
                || command_line.long_options.contains("--function-context") =>
        {
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
        process::CallingProcess::GitGrep(command_line)
            if command_line.short_options.contains("-p")
                || command_line.long_options.contains("--show-function") =>
        {
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

enum GrepLineRegex {
    WithFileExtensionAndLineNumber,
    WithFileExtension,
    WithFileExtensionNoSpaces,
    WithoutSeparatorCharacters,
}

lazy_static! {
    static ref GREP_LINE_REGEX_ASSUMING_FILE_EXTENSION_AND_LINE_NUMBER: Regex =
        make_grep_line_regex(GrepLineRegex::WithFileExtensionAndLineNumber);
}

lazy_static! {
    static ref GREP_LINE_REGEX_ASSUMING_FILE_EXTENSION_NO_SPACES: Regex =
        make_grep_line_regex(GrepLineRegex::WithFileExtensionNoSpaces);
}

lazy_static! {
    static ref GREP_LINE_REGEX_ASSUMING_FILE_EXTENSION: Regex =
        make_grep_line_regex(GrepLineRegex::WithFileExtension);
}

lazy_static! {
    static ref GREP_LINE_REGEX_ASSUMING_NO_INTERNAL_SEPARATOR_CHARS: Regex =
        make_grep_line_regex(GrepLineRegex::WithoutSeparatorCharacters);
}

// See tests for example grep lines
fn make_grep_line_regex(regex_variant: GrepLineRegex) -> Regex {
    // Grep tools such as `git grep` and `rg` emit lines like the following,
    // where "xxx" represents arbitrary code. Note that there are 3 possible
    // "separator characters": ':', '-', '='.

    // The format is ambiguous, but we attempt to parse it.

    // src/co-7-fig.rs:xxx
    // src/co-7-fig.rs:7:xxx
    // src/co-7-fig.rs-xxx
    // src/co-7-fig.rs-7-xxx
    // src/co-7-fig.rs=xxx
    // src/co-7-fig.rs=7=xxx

    // Makefile:xxx
    // Makefile:7:xxx
    // Makefile-xxx
    // Makefile-7-xxx

    // Make-7-file:xxx
    // Make-7-file:7:xxx
    // Make-7-file-xxx
    // Make-7-file-7-xxx

    let file_path = match regex_variant {
        GrepLineRegex::WithFileExtensionAndLineNumber | GrepLineRegex::WithFileExtension => {
            r"
        (                        # 1. file name (colons not allowed)
            [^:|\ ]                 # try to be strict about what a file path can start with
            [^:]*                   # anything
            [^\ ]\.[^.\ :=-]{1,10}   # extension
        )    
        "
        }
        GrepLineRegex::WithFileExtensionNoSpaces => {
            r"
        (                        # 1. file name (colons not allowed)
            [^:|\ ]+                # try to be strict about what a file path can start with
            [^\ ]\.[^.\ :=-]{1,6}   # extension
        )    
        "
        }
        GrepLineRegex::WithoutSeparatorCharacters => {
            r"
        (                        # 1. file name (colons not allowed)
            [^:|\ =-]               # try to be strict about what a file path can start with
            [^:=-]*                 # anything except separators
            [^:\ ]                  # a file name cannot end with whitespace
        )    
        "
        }
    };

    let separator = match regex_variant {
        GrepLineRegex::WithFileExtensionAndLineNumber => {
            r#"
    (?:
        (
            :                # 2. match marker
            ([0-9]+):        # 3. line number followed by second match marker
        )
        |
        (
            -                # 4. nomatch marker
            ([0-9]+)-        # 5. line number followed by second nomatch marker
        )
        |
        (
            =                # 6. match marker
            ([0-9]+)=        # 7. line number followed by second header marker
        )
    )
            "#
        }
        _ => {
            r#"
    (?:
        (                    
            :                # 2. match marker
            (?:([0-9]+):)?   # 3. optional: line number followed by second match marker
        )
        |
        (
            -                # 4. nomatch marker
            (?:([0-9]+)-)?   # 5. optional: line number followed by second nomatch marker
        )
        |
        (
            =                # 6. match marker
            (?:([0-9]+)=)?   # 7. optional: line number followed by second header marker
        )
    )
        "#
        }
    };

    Regex::new(&format!(
        "(?x)
^
{file_path}
{separator}
(.*)                     # 8. code (i.e. line contents)
$
",
        file_path = file_path,
        separator = separator,
    ))
    .unwrap()
}

pub fn parse_grep_line(line: &str) -> Option<GrepLine> {
    if line.starts_with('{') {
        ripgrep_json::parse_line(line)
    } else {
        match &*process::calling_process() {
            process::CallingProcess::GitGrep(_) | process::CallingProcess::OtherGrep => [
                &*GREP_LINE_REGEX_ASSUMING_FILE_EXTENSION_AND_LINE_NUMBER,
                &*GREP_LINE_REGEX_ASSUMING_FILE_EXTENSION_NO_SPACES,
                &*GREP_LINE_REGEX_ASSUMING_FILE_EXTENSION,
                &*GREP_LINE_REGEX_ASSUMING_NO_INTERNAL_SEPARATOR_CHARS,
            ]
            .iter()
            .find_map(|regex| _parse_grep_line(*regex, line)),
            _ => None,
        }
    }
}

pub fn _parse_grep_line<'b>(regex: &Regex, line: &'b str) -> Option<GrepLine<'b>> {
    let caps = regex.captures(line)?;
    let file = caps.get(1).unwrap().as_str().into();
    let (line_type, line_number) = &[
        (2, LineType::Match),
        (4, LineType::Context),
        (6, LineType::ContextHeader),
    ]
    .iter()
    .find_map(|(i, line_type)| {
        if caps.get(*i).is_some() {
            let line_number: Option<usize> = caps.get(i + 1).and_then(|m| m.as_str().parse().ok());
            Some((*line_type, line_number))
        } else {
            None
        }
    })
    .unwrap(); // The regex matches so one of the three alternatives must have matched
    let code = caps.get(8).unwrap().as_str().into();

    Some(GrepLine {
        path: file,
        line_number: *line_number,
        line_type: *line_type,
        code,
        submatches: None,
    })
}

#[cfg(test)]
mod tests {
    use crate::handlers::grep::{parse_grep_line, GrepLine, LineType};
    use crate::utils::process::tests::FakeParentArgs;

    #[test]
    fn test_parse_grep_match() {
        let fake_parent_grep_command = "git --doesnt-matter grep --nor-this nor_this -- nor_this";
        let _args = FakeParentArgs::for_scope(fake_parent_grep_command);

        assert_eq!(
            parse_grep_line("src/co-7-fig.rs:xxx"),
            Some(GrepLine {
                path: "src/co-7-fig.rs".into(),
                line_number: None,
                line_type: LineType::Match,
                code: "xxx".into(),
                submatches: None,
            })
        );

        assert_eq!(
            parse_grep_line("src/config.rs:use crate::minusplus::MinusPlus;"),
            Some(GrepLine {
                path: "src/config.rs".into(),
                line_number: None,
                line_type: LineType::Match,
                code: "use crate::minusplus::MinusPlus;".into(),
                submatches: None,
            })
        );

        assert_eq!(
            parse_grep_line(
                "src/config.rs:    pub line_numbers_style_minusplus: MinusPlus<Style>,"
            ),
            Some(GrepLine {
                path: "src/config.rs".into(),
                line_number: None,
                line_type: LineType::Match,
                code: "    pub line_numbers_style_minusplus: MinusPlus<Style>,".into(),
                submatches: None,
            })
        );

        assert_eq!(
            parse_grep_line("src/con-fig.rs:use crate::minusplus::MinusPlus;"),
            Some(GrepLine {
                path: "src/con-fig.rs".into(),
                line_number: None,
                line_type: LineType::Match,
                code: "use crate::minusplus::MinusPlus;".into(),
                submatches: None,
            })
        );

        assert_eq!(
            parse_grep_line(
                "src/con-fig.rs:    pub line_numbers_style_minusplus: MinusPlus<Style>,"
            ),
            Some(GrepLine {
                path: "src/con-fig.rs".into(),
                line_number: None,
                line_type: LineType::Match,
                code: "    pub line_numbers_style_minusplus: MinusPlus<Style>,".into(),
                submatches: None,
            })
        );

        assert_eq!(
            parse_grep_line(
                "src/de lta.rs:pub fn delta<I>(lines: ByteLines<I>, writer: &mut dyn Write, config: &Config) -> std::io::Result<()>"
            ),
            Some(GrepLine {
                path: "src/de lta.rs".into(),
                line_number: None,
                line_type: LineType::Match,
                code: "pub fn delta<I>(lines: ByteLines<I>, writer: &mut dyn Write, config: &Config) -> std::io::Result<()>".into(),
                submatches: None,
            })
        );

        assert_eq!(
            parse_grep_line(
                "src/de lta.rs:    pub fn new(writer: &'a mut dyn Write, config: &'a Config) -> Self {"
            ),
            Some(GrepLine {
                path: "src/de lta.rs".into(),
                line_number: None,
                line_type: LineType::Match,
                code: "    pub fn new(writer: &'a mut dyn Write, config: &'a Config) -> Self {".into(),
                submatches: None,
            })
        );
    }

    #[test]
    fn test_parse_grep_n_match() {
        let fake_parent_grep_command =
            "/usr/local/bin/git --doesnt-matter grep --nor-this nor_this -- nor_this";
        let _args = FakeParentArgs::for_scope(fake_parent_grep_command);

        assert_eq!(
            parse_grep_line("src/co-7-fig.rs:7:xxx"),
            Some(GrepLine {
                path: "src/co-7-fig.rs".into(),
                line_number: Some(7),
                line_type: LineType::Match,
                code: "xxx".into(),
                submatches: None,
            })
        );

        assert_eq!(
            parse_grep_line("src/config.rs:21:use crate::minusplus::MinusPlus;"),
            Some(GrepLine {
                path: "src/config.rs".into(),
                line_number: Some(21),
                line_type: LineType::Match,
                code: "use crate::minusplus::MinusPlus;".into(),
                submatches: None,
            })
        );

        assert_eq!(
            parse_grep_line(
                "src/config.rs:95:    pub line_numbers_style_minusplus: MinusPlus<Style>,"
            ),
            Some(GrepLine {
                path: "src/config.rs".into(),
                line_number: Some(95),
                line_type: LineType::Match,
                code: "    pub line_numbers_style_minusplus: MinusPlus<Style>,".into(),
                submatches: None,
            })
        );

        assert_eq!(
            parse_grep_line("Makefile:10:test: unit-test end-to-end-test"),
            Some(GrepLine {
                path: "Makefile".into(),
                line_number: Some(10),
                line_type: LineType::Match,
                code: "test: unit-test end-to-end-test".into(),
                submatches: None,
            })
        );

        assert_eq!(
            parse_grep_line(
                "Makefile:16:    ./tests/test_raw_output_matches_git_on_full_repo_history"
            ),
            Some(GrepLine {
                path: "Makefile".into(),
                line_number: Some(16),
                line_type: LineType::Match,
                code: "    ./tests/test_raw_output_matches_git_on_full_repo_history".into(),
                submatches: None,
            })
        );
    }

    #[test]
    #[ignore]
    fn test_parse_grep_n_match_file_name_with_dashes_and_no_extension() {
        // git grep -n
        // This fails: we can't parse it currently.
        let fake_parent_grep_command =
            "/usr/local/bin/git --doesnt-matter grep --nor-this nor_this -- nor_this";
        let _args = FakeParentArgs::once(fake_parent_grep_command);

        assert_eq!(
            parse_grep_line("etc/examples/119-within-line-edits:4:repo=$(mktemp -d)"),
            Some(GrepLine {
                path: "etc/examples/119-within-line-edits".into(),
                line_number: Some(4),
                line_type: LineType::Match,
                code: "repo=$(mktemp -d)".into(),
                submatches: None,
            })
        );
    }

    #[test]
    fn test_parse_grep_n_match_directory_name_with_dashes() {
        let fake_parent_grep_command =
            "/usr/local/bin/git --doesnt-matter grep --nor-this nor_this -- nor_this";
        let _args = FakeParentArgs::once(fake_parent_grep_command);

        assert_eq!(
            parse_grep_line("etc/META-INF/foo.properties:4:value=hi-there"),
            Some(GrepLine {
                path: "etc/META-INF/foo.properties".into(),
                line_number: Some(4),
                line_type: LineType::Match,
                code: "value=hi-there".into(),
                submatches: None,
            })
        );
    }

    #[test]
    fn test_parse_grep_no_match() {
        let fake_parent_grep_command =
            "/usr/local/bin/git --doesnt-matter grep --nor-this nor_this -- nor_this";
        let _args = FakeParentArgs::for_scope(fake_parent_grep_command);

        assert_eq!(
            parse_grep_line("src/co-7-fig.rs-xxx"),
            Some(GrepLine {
                path: "src/co-7-fig.rs".into(),
                line_number: None,
                line_type: LineType::Context,
                code: "xxx".into(),
                submatches: None,
            })
        );

        assert_eq!(
            parse_grep_line("src/config.rs-    pub available_terminal_width: usize,"),
            Some(GrepLine {
                path: "src/config.rs".into(),
                line_number: None,
                line_type: LineType::Context,
                code: "    pub available_terminal_width: usize,".into(),
                submatches: None,
            })
        );

        assert_eq!(
            parse_grep_line("src/con-fig.rs-use crate::minusplus::MinusPlus;"),
            Some(GrepLine {
                path: "src/con-fig.rs".into(),
                line_number: None,
                line_type: LineType::Context,
                code: "use crate::minusplus::MinusPlus;".into(),
                submatches: None,
            })
        );

        assert_eq!(
            parse_grep_line("de-lta.rs-            if self.source == Source::Unknown {"),
            Some(GrepLine {
                path: "de-lta.rs".into(),
                line_number: None,
                line_type: LineType::Context,
                code: "            if self.source == Source::Unknown {".into(),
                submatches: None,
            })
        );

        assert_eq!(
            parse_grep_line(r#"aaa/bbb.scala-              s"xxx.yyy.zzz: $ccc ddd""#),
            Some(GrepLine {
                path: "aaa/bbb.scala".into(),
                line_number: None,
                line_type: LineType::Context,
                code: r#"              s"xxx.yyy.zzz: $ccc ddd""#.into(),
                submatches: None,
            })
        );

        assert_eq!(
            parse_grep_line(r#"aaa/bbb.scala-  val atRegex = Regex.compile("(@.*)|(-shdw@.*)""#),
            Some(GrepLine {
                path: "aaa/bbb.scala".into(),
                line_number: None,
                line_type: LineType::Context,
                code: r#"  val atRegex = Regex.compile("(@.*)|(-shdw@.*)""#.into(),
                submatches: None,
            })
        );
    }

    #[test]
    fn test_parse_grep_n_no_match() {
        // git grep -n
        let fake_parent_grep_command =
            "/usr/local/bin/git --doesnt-matter grep --nor-this nor_this -- nor_this";

        let _args = FakeParentArgs::for_scope(fake_parent_grep_command);
        assert_eq!(
            parse_grep_line("src/co-7-fig.rs-7-xxx"),
            Some(GrepLine {
                path: "src/co-7-fig.rs".into(),
                line_number: Some(7),
                line_type: LineType::Context,
                code: "xxx".into(),
                submatches: None,
            })
        );

        assert_eq!(
            parse_grep_line("src/config.rs-58-    pub available_terminal_width: usize,"),
            Some(GrepLine {
                path: "src/config.rs".into(),
                line_number: Some(58),
                line_type: LineType::Context,
                code: "    pub available_terminal_width: usize,".into(),
                submatches: None,
            })
        );

        assert_eq!(
            parse_grep_line(r#"foo.rs-12-  .x-"#),
            Some(GrepLine {
                path: "foo.rs".into(),
                line_number: Some(12),
                line_type: LineType::Context,
                code: r#"  .x-"#.into(),
                submatches: None,
            })
        );

        assert_eq!(
            parse_grep_line(r#"foo.rs-12-.x-"#),
            Some(GrepLine {
                path: "foo.rs".into(),
                line_number: Some(12),
                line_type: LineType::Context,
                code: r#".x-"#.into(),
                submatches: None,
            })
        );
    }

    #[test]
    fn test_parse_grep_match_no_extension() {
        let fake_parent_grep_command =
            "/usr/local/bin/git --doesnt-matter grep --nor-this nor_this -- nor_this";
        let _args = FakeParentArgs::once(fake_parent_grep_command);

        assert_eq!(
            parse_grep_line("Makefile:xxx"),
            Some(GrepLine {
                path: "Makefile".into(),
                line_number: None,
                line_type: LineType::Match,
                code: "xxx".into(),
                submatches: None,
            })
        );
    }

    #[test]
    fn test_parse_grep_n_match_no_extension() {
        // git grep -n
        let fake_parent_grep_command =
            "/usr/local/bin/git --doesnt-matter grep --nor-this nor_this -- nor_this";
        let _args = FakeParentArgs::once(fake_parent_grep_command);
        assert_eq!(
            parse_grep_line("Makefile:7:xxx"),
            Some(GrepLine {
                path: "Makefile".into(),
                line_number: Some(7),
                line_type: LineType::Match,
                code: "xxx".into(),
                submatches: None,
            })
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_parse_grep_W_context_header() {
        // git grep -W
        let fake_parent_grep_command =
            "/usr/local/bin/git --doesnt-matter grep --nor-this nor_this -- nor_this";

        let _args = FakeParentArgs::once(fake_parent_grep_command);
        assert_eq!(
            parse_grep_line("src/config.rs=pub struct Config {"), // match
            Some(GrepLine {
                path: "src/config.rs".into(),
                line_number: None,
                line_type: LineType::ContextHeader,
                code: "pub struct Config {".into(),
                submatches: None,
            })
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_parse_grep_W_n_context_header() {
        // git grep -n -W
        let fake_parent_grep_command =
            "/usr/local/bin/git --doesnt-matter grep --nor-this nor_this -- nor_this";
        let _args = FakeParentArgs::once(fake_parent_grep_command);

        assert_eq!(
            parse_grep_line("src/config.rs=57=pub struct Config {"),
            Some(GrepLine {
                path: "src/config.rs".into(),
                line_number: Some(57),
                line_type: LineType::ContextHeader,
                code: "pub struct Config {".into(),
                submatches: None,
            })
        );
    }

    #[test]
    fn test_parse_grep_not_grep_output() {
        let fake_parent_grep_command =
            "/usr/local/bin/git --doesnt-matter grep --nor-this nor_this -- nor_this";
        let _args = FakeParentArgs::once(fake_parent_grep_command);

        let not_grep_output = "|       expose it in delta's color output styled with grep:";
        assert_eq!(parse_grep_line(not_grep_output), None);
    }

    #[test]
    fn test_parse_grep_parent_command_is_not_grep_1() {
        let fake_parent_grep_command =
            "/usr/local/bin/notgrep --doesnt-matter --nor-this nor_this -- nor_this";
        let _args = FakeParentArgs::once(fake_parent_grep_command);

        let apparently_grep_output = "src/co-7-fig.rs:xxx";
        assert_eq!(parse_grep_line(apparently_grep_output), None);
    }

    #[test]
    fn test_parse_grep_parent_command_is_not_grep_2() {
        // No fake parent grep command
        let apparently_grep_output = "src/co-7-fig.rs:xxx";
        assert_eq!(parse_grep_line(apparently_grep_output), None);
    }

    #[test]
    fn test_get_code_style_sections() {
        use crate::ansi::strip_ansi_codes;
        use crate::handlers::grep::get_code_style_sections;
        use crate::paint::StyleSectionSpecifier;
        use crate::style::Style;

        let fake_parent_grep_command = "git --doesnt-matter grep --nor-this nor_this -- nor_this";
        let _args = FakeParentArgs::for_scope(fake_parent_grep_command);

        let miss = Style::new();
        let hit = Style {
            is_emph: true,
            ..miss
        };

        let escape = "\x1B";
        let working_example = format!("foo/bar/baz.yaml{escape}[36m:{escape}[m1090{escape}[36m:{escape}[m  - {escape}[1;31mkind: Service{escape}[mAccount");
        let stripped = strip_ansi_codes(&working_example);
        let grep = parse_grep_line(&stripped).unwrap();

        assert_eq!(
            get_code_style_sections(&working_example, hit, miss, &grep),
            Some(StyleSectionSpecifier::StyleSections(vec![
                (miss, "  - "),
                (hit, "kind: Service"),
                (miss, "Account")
            ]))
        );

        let broken_example = format!("foo/bar/baz.yaml{escape}[36m:{escape}[m2{escape}[36m:{escape}[m{escape}[1;31mkind: Service{escape}[m");
        let broken_stripped = strip_ansi_codes(&broken_example);
        let broken_grep = parse_grep_line(&broken_stripped).unwrap();

        assert_eq!(
            get_code_style_sections(&broken_example, hit, miss, &broken_grep),
            Some(StyleSectionSpecifier::StyleSections(vec![(
                hit,
                "kind: Service"
            )]))
        );

        let plus_example = format!("etc/examples/189-merge-conflict.2.diff{escape}[36m:{escape}[m10{escape}[36m:{escape}[m{escape}[32m +        let (style, non_emph_style) = {escape}[1;31mmatch{escape}[m state {{{escape}[m");
        let plus_stripped = strip_ansi_codes(&plus_example);
        let plus_grep = parse_grep_line(&plus_stripped).unwrap();

        assert_eq!(
            get_code_style_sections(&plus_example, hit, miss, &plus_grep),
            Some(StyleSectionSpecifier::StyleSections(vec![
                (miss, " +        let (style, non_emph_style) = "),
                (hit, "match"),
                (miss, " state {")
            ]))
        );
    }
}
