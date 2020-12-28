use std::borrow::Cow;
use std::io::BufRead;
use std::io::Write;

use bytelines::ByteLines;
use unicode_segmentation::UnicodeSegmentation;

use crate::ansi;
use crate::cli;
use crate::config::Config;
use crate::draw;
use crate::features;
use crate::format;
use crate::hunk_header;
use crate::paint::Painter;
use crate::parse;
use crate::style::{self, DecorationStyle};

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    CommitMeta,                // In commit metadata section
    FileMeta, // In diff metadata section, between (possible) commit metadata and first hunk
    HunkHeader, // In hunk metadata line
    HunkZero, // In hunk; unchanged line
    HunkMinus(Option<String>), // In hunk; removed line (raw_line)
    HunkPlus(Option<String>), // In hunk; added line (raw_line)
    Unknown,
}

#[derive(Debug, PartialEq)]
pub enum Source {
    GitDiff,     // Coming from a `git diff` command
    DiffUnified, // Coming from a `diff -u` command
    Unknown,
}

impl State {
    fn is_in_hunk(&self) -> bool {
        matches!(*self, State::HunkHeader | State::HunkZero | State::HunkMinus(_) | State::HunkPlus(_))
    }
}

// Possible transitions, with actions on entry:
//
//
// | from \ to   | CommitMeta  | FileMeta    | HunkHeader  | HunkZero    | HunkMinus   | HunkPlus |
// |-------------+-------------+-------------+-------------+-------------+-------------+----------|
// | CommitMeta  | emit        | emit        |             |             |             |          |
// | FileMeta    |             | emit        | emit        |             |             |          |
// | HunkHeader  |             |             |             | emit        | push        | push     |
// | HunkZero    | emit        | emit        | emit        | emit        | push        | push     |
// | HunkMinus   | flush, emit | flush, emit | flush, emit | flush, emit | push        | push     |
// | HunkPlus    | flush, emit | flush, emit | flush, emit | flush, emit | flush, push | push     |

struct StateMachine<'a> {
    painter: Painter<'a>,
}

impl<'a> StateMachine<'a> {
    pub fn new(writer: &'a mut dyn Write, config: &'a Config) -> Self {
        Self {
            painter: Painter::new(writer, config),
        }
    }

    fn handle_commit_meta_header_line(
        &mut self,
        line: &str,
        raw_line: &str,
        config: &Config,
    ) -> std::io::Result<()> {
        if config.commit_style.is_omitted {
            return Ok(());
        }
        let (mut draw_fn, pad, decoration_ansi_term_style) =
            draw::get_draw_function(config.commit_style.decoration_style);
        let (formatted_line, formatted_raw_line) = if config.hyperlinks {
            (
                features::hyperlinks::format_commit_line_with_osc8_commit_hyperlink(line, config),
                features::hyperlinks::format_commit_line_with_osc8_commit_hyperlink(
                    raw_line, config,
                ),
            )
        } else {
            (Cow::from(line), Cow::from(raw_line))
        };

        draw_fn(
            self.painter.writer,
            &format!("{}{}", formatted_line, if pad { " " } else { "" }),
            &format!("{}{}", formatted_raw_line, if pad { " " } else { "" }),
            &config.decorations_width,
            config.commit_style,
            decoration_ansi_term_style,
        )?;
        Ok(())
    }
}

pub fn delta<I>(
    mut lines: ByteLines<I>,
    writer: &mut dyn Write,
    config: &Config,
) -> std::io::Result<()>
where
    I: BufRead,
{
    let mut machine = StateMachine::new(writer, config);
    let mut minus_file = "".to_string();
    let mut plus_file = "".to_string();
    let mut file_event = parse::FileEvent::NoEvent;
    let mut state = State::Unknown;
    let mut should_continue;
    let mut source = Source::Unknown;

    // When a file is modified, we use lines starting with '---' or '+++' to obtain the file name.
    // When a file is renamed without changes, we use lines starting with 'rename' to obtain the
    // file name (there is no diff hunk and hence no lines starting with '---' or '+++'). But when
    // a file is renamed with changes, both are present, and we rely on the following variables to
    // avoid emitting the file meta header line twice (#245).
    let mut current_file_pair;
    let mut handled_file_meta_header_line_file_pair = None;

    while let Some(Ok(raw_line_bytes)) = lines.next() {
        let raw_line = String::from_utf8_lossy(&raw_line_bytes);
        let raw_line = if config.max_line_length > 0 && raw_line.len() > config.max_line_length {
            ansi::truncate_str(&raw_line, config.max_line_length, &config.truncation_symbol)
        } else {
            raw_line
        };
        let line = ansi::strip_ansi_codes(&raw_line).to_string();
        if source == Source::Unknown {
            source = detect_source(&line);
        }
        if line.starts_with("commit ") {
            machine.painter.paint_buffered_minus_and_plus_lines();
            state = State::CommitMeta;
            if should_handle(&state, config) {
                machine.painter.emit()?;
                machine.handle_commit_meta_header_line(&line, &raw_line, config)?;
                continue;
            }
        } else if line.starts_with("diff ") {
            machine.painter.paint_buffered_minus_and_plus_lines();
            state = State::FileMeta;
            handled_file_meta_header_line_file_pair = None;
        } else if (state == State::FileMeta || source == Source::DiffUnified)
            && (line.starts_with("--- ")
                || line.starts_with("rename from ")
                || line.starts_with("copy from "))
        {
            let parsed_file_meta_line =
                parse::parse_file_meta_line(&line, source == Source::GitDiff);
            minus_file = parsed_file_meta_line.0;
            file_event = parsed_file_meta_line.1;

            should_continue = handle_file_meta_minus_line(
                &mut state,
                &source,
                &minus_file,
                &mut machine.painter,
                &line,
                &raw_line,
                config,
            )?;
            if should_continue {
                continue;
            }
        } else if (state == State::FileMeta || source == Source::DiffUnified)
            && (line.starts_with("+++ ")
                || line.starts_with("rename to ")
                || line.starts_with("copy to "))
        {
            let parsed_file_meta_line =
                parse::parse_file_meta_line(&line, source == Source::GitDiff);
            plus_file = parsed_file_meta_line.0;
            machine
                .painter
                .set_syntax(parse::get_file_extension_from_file_meta_line_file_path(
                    &plus_file,
                ));
            current_file_pair = Some((minus_file.clone(), plus_file.clone()));

            // In color_only mode, raw_line's structure shouldn't be changed.
            // So it needs to avoid fn handle_file_meta_header_line
            // (it connects the plus_file and minus_file),
            // and to call fn handle_generic_file_meta_header_line directly.
            if config.color_only {
                handle_generic_file_meta_header_line(
                    &mut machine.painter,
                    &line,
                    &raw_line,
                    config,
                )?;
                continue;
            }
            if should_handle(&State::FileMeta, config)
                && handled_file_meta_header_line_file_pair != current_file_pair
            {
                machine.painter.emit()?;
                handle_file_meta_header_line(
                    &mut machine.painter,
                    &minus_file,
                    &plus_file,
                    config,
                    &file_event,
                    source == Source::DiffUnified,
                )?;
                handled_file_meta_header_line_file_pair = current_file_pair
            }
        } else if line.starts_with("@@") {
            machine.painter.paint_buffered_minus_and_plus_lines();
            state = State::HunkHeader;
            machine.painter.set_highlighter();
            machine.painter.emit()?;
            handle_hunk_header_line(&mut machine.painter, &line, &raw_line, &plus_file, config)?;
            machine.painter.set_highlighter();
            continue;
        } else if source == Source::DiffUnified && line.starts_with("Only in ")
            || line.starts_with("Submodule ")
            || line.starts_with("Binary files ")
        {
            // Additional FileMeta cases:
            //
            // 1. When comparing directories with diff -u, if filenames match between the
            //    directories, the files themselves will be compared. However, if an equivalent
            //    filename is not present, diff outputs a single line (Only in...) starting
            //    indicating that the file is present in only one of the directories.
            //
            // 2. Git diff emits lines describing submodule state such as "Submodule x/y/z contains
            //    untracked content"
            //
            // See https://github.com/dandavison/delta/issues/60#issuecomment-557485242 for a
            // proposal for more robust parsing logic.

            machine.painter.paint_buffered_minus_and_plus_lines();
            state = State::FileMeta;
            if should_handle(&State::FileMeta, config) {
                machine.painter.emit()?;
                handle_generic_file_meta_header_line(
                    &mut machine.painter,
                    &line,
                    &raw_line,
                    config,
                )?;
                continue;
            }
        } else if state.is_in_hunk() {
            // A true hunk line should start with one of: '+', '-', ' '. However, handle_hunk_line
            // handles all lines until the state machine transitions away from the hunk states.
            state = handle_hunk_line(&mut machine.painter, &line, &raw_line, state, config);
            machine.painter.emit()?;
            continue;
        }

        if state == State::FileMeta && should_handle(&State::FileMeta, config) && !config.color_only
        {
            // The file metadata section is 4 lines. Skip them under non-plain file-styles.
            // However in the case of color_only mode,
            // we won't skip because we can't change raw_line structure.
            continue;
        } else {
            machine.painter.emit()?;
            writeln!(
                machine.painter.writer,
                "{}",
                format::format_raw_line(&raw_line, config)
            )?;
        }
    }

    machine.painter.paint_buffered_minus_and_plus_lines();
    machine.painter.emit()?;
    Ok(())
}

/// Should a handle_* function be called on this element?
fn should_handle(state: &State, config: &Config) -> bool {
    let style = config.get_style(state);
    !(style.is_raw && style.decoration_style == DecorationStyle::NoDecoration)
}

/// Try to detect what is producing the input for delta.
///
/// Currently can detect:
/// * git diff
/// * diff -u
fn detect_source(line: &str) -> Source {
    if line.starts_with("commit ") || line.starts_with("diff --git ") {
        Source::GitDiff
    } else if line.starts_with("diff -u")
        || line.starts_with("diff -ru")
        || line.starts_with("diff -r -u")
        || line.starts_with("diff -U")
        || line.starts_with("--- ")
        || line.starts_with("Only in ")
    {
        Source::DiffUnified
    } else {
        Source::Unknown
    }
}

fn handle_file_meta_minus_line(
    state: &mut State,
    source: &Source,
    minus_file: &str,
    painter: &mut Painter,
    line: &str,
    raw_line: &str,
    config: &Config,
) -> std::io::Result<bool> {
    let mut should_continue = false;
    if source == &Source::DiffUnified {
        *state = State::FileMeta;
        painter.set_syntax(parse::get_file_extension_from_marker_line(&line));
    } else {
        painter.set_syntax(parse::get_file_extension_from_file_meta_line_file_path(
            &minus_file,
        ));
    }

    // In color_only mode, raw_line's structure shouldn't be changed.
    // So it needs to avoid fn handle_file_meta_header_line
    // (it connects the plus_file and minus_file),
    // and to call fn handle_generic_file_meta_header_line directly.
    if config.color_only {
        handle_generic_file_meta_header_line(painter, &line, &raw_line, config)?;
        should_continue = true;
    }
    Ok(should_continue)
}

/// Construct file change line from minus and plus file and write with FileMeta styling.
fn handle_file_meta_header_line(
    painter: &mut Painter,
    minus_file: &str,
    plus_file: &str,
    config: &Config,
    file_event: &parse::FileEvent,
    comparing: bool,
) -> std::io::Result<()> {
    let line = parse::get_file_change_description_from_file_paths(
        minus_file, plus_file, comparing, file_event, config,
    );
    // FIXME: no support for 'raw'
    handle_generic_file_meta_header_line(painter, &line, &line, config)
}

/// Write `line` with FileMeta styling.
fn handle_generic_file_meta_header_line(
    painter: &mut Painter,
    line: &str,
    raw_line: &str,
    config: &Config,
) -> std::io::Result<()> {
    // If file_style is "omit", we'll skip the process and print nothing.
    // However in the case of color_only mode,
    // we won't skip because we can't change raw_line structure.
    if config.file_style.is_omitted && !config.color_only {
        return Ok(());
    }
    let (mut draw_fn, pad, decoration_ansi_term_style) =
        draw::get_draw_function(config.file_style.decoration_style);
    // Prints the new line below file-meta-line.
    // However in the case of color_only mode,
    // we won't print it because we can't change raw_line structure.
    if !config.color_only {
        writeln!(painter.writer)?;
    }
    draw_fn(
        painter.writer,
        &format!("{}{}", line, if pad { " " } else { "" }),
        &format!("{}{}", raw_line, if pad { " " } else { "" }),
        &config.decorations_width,
        config.file_style,
        decoration_ansi_term_style,
    )?;
    Ok(())
}

/// Emit the hunk header, with any requested decoration.
fn handle_hunk_header_line(
    painter: &mut Painter,
    line: &str,
    raw_line: &str,
    plus_file: &str,
    config: &Config,
) -> std::io::Result<()> {
    let (raw_code_fragment, line_numbers) = parse::parse_hunk_header(&line);
    if config.line_numbers {
        painter
            .line_numbers_data
            .initialize_hunk(&line_numbers, plus_file.to_string());
    }

    if config.hunk_header_style.is_raw {
        hunk_header::write_hunk_header_raw(painter, line, raw_line, config)?;
    } else if config.hunk_header_style.is_omitted {
        writeln!(painter.writer)?;
    } else {
        hunk_header::write_hunk_header(
            &raw_code_fragment,
            &line_numbers,
            painter,
            line,
            plus_file,
            config,
        )?;
    };
    Ok(())
}

/// Handle a hunk line, i.e. a minus line, a plus line, or an unchanged line.
// In the case of a minus or plus line, we store the line in a
// buffer. When we exit the changed region we process the collected
// minus and plus lines jointly, in order to paint detailed
// highlighting according to inferred edit operations. In the case of
// an unchanged line, we paint it immediately.
fn handle_hunk_line(
    painter: &mut Painter,
    line: &str,
    raw_line: &str,
    state: State,
    config: &Config,
) -> State {
    // Don't let the line buffers become arbitrarily large -- if we
    // were to allow that, then for a large deleted/added file we
    // would process the entire file before painting anything.
    if painter.minus_lines.len() > config.line_buffer_size
        || painter.plus_lines.len() > config.line_buffer_size
    {
        painter.paint_buffered_minus_and_plus_lines();
    }
    match line.chars().next() {
        Some('-') => {
            if let State::HunkPlus(_) = state {
                painter.paint_buffered_minus_and_plus_lines();
            }
            let state = match config.inspect_raw_lines {
                cli::InspectRawLines::True
                    if style::line_has_style_other_than(
                        raw_line,
                        [*style::GIT_DEFAULT_MINUS_STYLE, config.git_minus_style].iter(),
                    ) =>
                {
                    State::HunkMinus(Some(painter.prepare_raw_line(raw_line)))
                }
                _ => State::HunkMinus(None),
            };
            painter
                .minus_lines
                .push((painter.prepare(&line), state.clone()));
            state
        }
        Some('+') => {
            let state = match config.inspect_raw_lines {
                cli::InspectRawLines::True
                    if style::line_has_style_other_than(
                        raw_line,
                        [*style::GIT_DEFAULT_PLUS_STYLE, config.git_plus_style].iter(),
                    ) =>
                {
                    State::HunkPlus(Some(painter.prepare_raw_line(raw_line)))
                }
                _ => State::HunkPlus(None),
            };
            painter
                .plus_lines
                .push((painter.prepare(&line), state.clone()));
            state
        }
        Some(' ') => {
            painter.paint_buffered_minus_and_plus_lines();
            painter.paint_zero_line(&line);
            State::HunkZero
        }
        _ => {
            // The first character here could be e.g. '\' from '\ No newline at end of file'. This
            // is not a hunk line, but the parser does not have a more accurate state corresponding
            // to this.
            painter.paint_buffered_minus_and_plus_lines();
            painter
                .output_buffer
                .push_str(&painter.expand_tabs(raw_line.graphemes(true)));
            painter.output_buffer.push('\n');
            State::HunkZero
        }
    }
}
