use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Write;

use ansi_term::ANSIString;
use itertools::Itertools;
use syntect::easy::HighlightLines;
use syntect::highlighting::Style as SyntectStyle;
use syntect::parsing::{SyntaxReference, SyntaxSet};
use unicode_segmentation::UnicodeSegmentation;

use crate::config::{self, delta_unreachable, Config};
use crate::delta::{DiffType, InMergeConflict, MergeParents, State};
use crate::features::hyperlinks;
use crate::features::line_numbers::{self, LineNumbersData};
use crate::features::side_by_side::ansifill;
use crate::features::side_by_side::{self, PanelSide};
use crate::handlers::merge_conflict;
use crate::minusplus::*;
use crate::paint::superimpose_style_sections::superimpose_style_sections;
use crate::style::Style;
use crate::{ansi, style};
use crate::{edits, utils};

pub type LineSections<'a, S> = Vec<(S, &'a str)>;

pub struct Painter<'p> {
    pub minus_lines: Vec<(String, State)>,
    pub plus_lines: Vec<(String, State)>,
    pub writer: &'p mut dyn Write,
    pub syntax: &'p SyntaxReference,
    pub highlighter: Option<HighlightLines<'p>>,
    pub config: &'p config::Config,
    pub output_buffer: String,
    // If config.line_numbers is true, then the following is always Some().
    // In side-by-side mode it is always Some (but possibly an empty one), even
    // if config.line_numbers is false. See `UseFullPanelWidth` as well.
    pub line_numbers_data: Option<line_numbers::LineNumbersData<'p>>,
    pub merge_conflict_lines: merge_conflict::MergeConflictLines,
    pub merge_conflict_commit_names: merge_conflict::MergeConflictCommitNames,
}

// How the background of a line is filled up to the end
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BgFillMethod {
    // Fill the background with ANSI spaces if possible,
    // but might fallback to Spaces (e.g. in the left side-by-side panel),
    // also see `UseFullPanelWidth`
    TryAnsiSequence,
    Spaces,
}

// If the background of a line extends to the end, and if configured to do so, how.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BgShouldFill {
    With(BgFillMethod),
    No,
}

impl Default for BgShouldFill {
    fn default() -> Self {
        BgShouldFill::With(BgFillMethod::TryAnsiSequence)
    }
}

#[derive(PartialEq, Debug)]
pub enum StyleSectionSpecifier<'l> {
    Style(Style),
    StyleSections(LineSections<'l, Style>),
}

impl<'p> Painter<'p> {
    pub fn new(writer: &'p mut dyn Write, config: &'p config::Config) -> Self {
        let default_syntax = Self::get_syntax(&config.syntax_set, None);

        let panel_width_fix = ansifill::UseFullPanelWidth::new(config);

        let line_numbers_data = if config.line_numbers {
            Some(line_numbers::LineNumbersData::from_format_strings(
                &config.line_numbers_format,
                panel_width_fix,
            ))
        } else if config.side_by_side {
            // If line numbers are disabled in side-by-side then the data is still used
            // for width calculaction and to pad odd width to even, see `UseFullPanelWidth`
            // for details.
            Some(line_numbers::LineNumbersData::empty_for_sbs(
                panel_width_fix,
            ))
        } else {
            None
        };
        Self {
            minus_lines: Vec::new(),
            plus_lines: Vec::new(),
            output_buffer: String::new(),
            syntax: default_syntax,
            highlighter: None,
            writer,
            config,
            line_numbers_data,
            merge_conflict_lines: merge_conflict::MergeConflictLines::new(),
            merge_conflict_commit_names: merge_conflict::MergeConflictCommitNames::new(),
        }
    }

    pub fn set_syntax(&mut self, extension: Option<&str>) {
        self.syntax = Painter::get_syntax(&self.config.syntax_set, extension);
    }

    fn get_syntax<'a>(syntax_set: &'a SyntaxSet, extension: Option<&str>) -> &'a SyntaxReference {
        if let Some(extension) = extension {
            if let Some(syntax) = syntax_set.find_syntax_by_extension(extension) {
                return syntax;
            }
        }
        syntax_set
            .find_syntax_by_extension("txt")
            .unwrap_or_else(|| delta_unreachable("Failed to find any language syntax definitions."))
    }

    pub fn set_highlighter(&mut self) {
        if let Some(ref syntax_theme) = self.config.syntax_theme {
            self.highlighter = Some(HighlightLines::new(self.syntax, syntax_theme))
        };
    }

    pub fn paint_buffered_minus_and_plus_lines(&mut self) {
        if self.minus_lines.is_empty() && self.plus_lines.is_empty() {
            return;
        }
        paint_minus_and_plus_lines(
            MinusPlus::new(&self.minus_lines, &self.plus_lines),
            &mut self.line_numbers_data,
            &mut self.highlighter,
            &mut self.output_buffer,
            self.config,
        );
        self.minus_lines.clear();
        self.plus_lines.clear();
    }

    pub fn paint_zero_line(&mut self, line: &str, state: State) {
        let lines = &[(line.to_string(), state.clone())];
        let syntax_style_sections =
            get_syntax_style_sections_for_lines(lines, self.highlighter.as_mut(), self.config);
        let mut diff_style_sections = vec![vec![(self.config.zero_style, lines[0].0.as_str())]]; // TODO: compute style from state
        Painter::update_diff_style_sections(
            lines,
            &mut diff_style_sections,
            None,
            None,
            &[false],
            self.config,
        );
        if self.config.side_by_side {
            // `lines[0].0` so the line has the '\n' already added (as in the +- case)
            side_by_side::paint_zero_lines_side_by_side(
                &lines[0].0,
                syntax_style_sections,
                diff_style_sections,
                &mut self.output_buffer,
                self.config,
                &mut self.line_numbers_data.as_mut(),
                painted_prefix(state, self.config),
                BgShouldFill::With(BgFillMethod::Spaces),
            );
        } else {
            Painter::paint_lines(
                lines,
                &syntax_style_sections,
                diff_style_sections.as_slice(),
                &[false],
                &mut self.output_buffer,
                self.config,
                &mut self.line_numbers_data.as_mut(),
                None,
                BgShouldFill::With(BgFillMethod::Spaces),
            );
        }
    }

    /// Superimpose background styles and foreground syntax
    /// highlighting styles, and write colored lines to output buffer.
    #[allow(clippy::too_many_arguments)]
    pub fn paint_lines<'a>(
        lines: &'a [(String, State)],
        syntax_style_sections: &[LineSections<'a, SyntectStyle>],
        diff_style_sections: &[LineSections<'a, Style>],
        lines_have_homolog: &[bool],
        output_buffer: &mut String,
        config: &config::Config,
        line_numbers_data: &mut Option<&mut line_numbers::LineNumbersData>,
        empty_line_style: Option<Style>, // a style with background color to highlight an empty line
        background_color_extends_to_terminal_width: BgShouldFill,
    ) {
        // There's some unfortunate hackery going on here for two reasons:
        //
        // 1. The prefix needs to be injected into the output stream.
        //
        // 2. We must ensure that we fill rightwards with the appropriate
        //    non-emph background color. In that case we don't use the last
        //    style of the line, because this might be emph.
        for ((((_, state), syntax_sections), diff_sections), &line_has_homolog) in lines
            .iter()
            .zip_eq(syntax_style_sections)
            .zip_eq(diff_style_sections)
            .zip_eq(lines_have_homolog)
        {
            let (mut line, line_is_empty) = Painter::paint_line(
                syntax_sections,
                diff_sections,
                state,
                line_numbers_data,
                None,
                painted_prefix(state.clone(), config),
                config,
            );
            let (bg_fill_mode, fill_style) =
                Painter::get_should_right_fill_background_color_and_fill_style(
                    diff_sections,
                    Some(line_has_homolog),
                    state,
                    background_color_extends_to_terminal_width,
                    config,
                );

            if let Some(BgFillMethod::TryAnsiSequence) = bg_fill_mode {
                Painter::right_fill_background_color(&mut line, fill_style);
            } else if let Some(BgFillMethod::Spaces) = bg_fill_mode {
                let text_width = ansi::measure_text_width(&line);
                line.push_str(
                    #[allow(clippy::unnecessary_to_owned)]
                    &fill_style
                        .paint(" ".repeat(config.available_terminal_width - text_width))
                        .to_string(),
                );
            } else if line_is_empty {
                if let Some(empty_line_style) = empty_line_style {
                    Painter::mark_empty_line(
                        &empty_line_style,
                        &mut line,
                        if config.line_numbers { Some(" ") } else { None },
                    );
                }
            };

            output_buffer.push_str(&line);
            output_buffer.push('\n');
        }
    }

    /// Write painted line to the output buffer, with syntax-highlighting and `style` superimposed.
    // Note that, if passing `style_sections` as
    // `StyleSectionSpecifier::StyleSections`, then tabs must already have been
    // expanded in the text.
    pub fn syntax_highlight_and_paint_line(
        &mut self,
        line: &str,
        style_sections: StyleSectionSpecifier,
        state: State,
        background_color_extends_to_terminal_width: BgShouldFill,
    ) {
        let lines = vec![(
            expand_tabs(line.graphemes(true), self.config.tab_width),
            state,
        )];
        let syntax_style_sections =
            get_syntax_style_sections_for_lines(&lines, self.highlighter.as_mut(), self.config);
        let diff_style_sections = match style_sections {
            StyleSectionSpecifier::Style(style) => vec![vec![(style, lines[0].0.as_str())]],
            StyleSectionSpecifier::StyleSections(style_sections) => vec![style_sections],
        };
        Painter::paint_lines(
            &lines,
            &syntax_style_sections,
            &diff_style_sections,
            &[false],
            &mut self.output_buffer,
            self.config,
            &mut None,
            None,
            background_color_extends_to_terminal_width,
        );
    }

    /// Determine whether the terminal should fill the line rightwards with a background color, and
    /// the style for doing so.
    pub fn get_should_right_fill_background_color_and_fill_style(
        diff_sections: &[(Style, &str)],
        line_has_homolog: Option<bool>,
        state: &State,
        background_color_extends_to_terminal_width: BgShouldFill,
        config: &config::Config,
    ) -> (Option<BgFillMethod>, Style) {
        let fill_style = match state {
            State::HunkMinus(_, None) | State::HunkMinusWrapped => {
                if let Some(true) = line_has_homolog {
                    config.minus_non_emph_style
                } else {
                    config.minus_style
                }
            }
            State::HunkZero(_, None) | State::HunkZeroWrapped => config.zero_style,
            State::HunkPlus(_, None) | State::HunkPlusWrapped => {
                if let Some(true) = line_has_homolog {
                    config.plus_non_emph_style
                } else {
                    config.plus_style
                }
            }
            State::HunkMinus(_, Some(_))
            | State::HunkZero(_, Some(_))
            | State::HunkPlus(_, Some(_)) => {
                // Consider the following raw line, from git colorMoved:
                // ␛[1;36m+␛[m␛[1;36mclass·X:·pass␛[m␊ The last style section returned by
                // parse_style_sections will be a default style associated with the terminal newline
                // character; we want the last "real" style.
                diff_sections
                    .iter()
                    .rev()
                    .filter(|(_, s)| s != &"\n")
                    .map(|(style, _)| *style)
                    .next()
                    .unwrap_or(config.null_style)
            }
            State::Blame(_) => diff_sections[0].0,
            _ => config.null_style,
        };

        match (
            fill_style.get_background_color().is_some(),
            background_color_extends_to_terminal_width,
        ) {
            (false, _) | (_, BgShouldFill::No) => (None, fill_style),
            (_, BgShouldFill::With(bgmode)) => {
                if config.background_color_extends_to_terminal_width {
                    (Some(bgmode), fill_style)
                } else {
                    (None, fill_style)
                }
            }
        }
    }

    /// Emit line with ANSI sequences that extend the background color to the terminal width.
    pub fn right_fill_background_color(line: &mut String, fill_style: Style) {
        // HACK: How to properly incorporate the ANSI_CSI_CLEAR_TO_EOL into ansi_strings?
        line.push_str(&ansi_term::ANSIStrings(&[fill_style.paint("")]).to_string());
        if line
            .to_lowercase()
            .ends_with(&ansi::ANSI_SGR_RESET.to_lowercase())
        {
            line.truncate(line.len() - ansi::ANSI_SGR_RESET.len());
        }
        line.push_str(ansi::ANSI_CSI_CLEAR_TO_EOL);
        line.push_str(ansi::ANSI_SGR_RESET);
    }

    /// Use ANSI sequences to visually mark the current line as empty. If `marker` is None then the
    /// line is marked using terminal emulator colors only, i.e. without appending any marker text
    /// to the line. This is typically appropriate only when the `line` buffer is empty, since
    /// otherwise the ANSI_CSI_CLEAR_TO_BOL instruction would overwrite the text to the left of the
    /// current buffer position.
    pub fn mark_empty_line(empty_line_style: &Style, line: &mut String, marker: Option<&str>) {
        line.push_str(
            #[allow(clippy::unnecessary_to_owned)]
            &empty_line_style
                .paint(marker.unwrap_or(ansi::ANSI_CSI_CLEAR_TO_BOL))
                .to_string(),
        );
    }

    /// Return painted line (maybe prefixed with line numbers field) and an is_empty? boolean.
    pub fn paint_line(
        syntax_sections: &[(SyntectStyle, &str)],
        diff_sections: &[(Style, &str)],
        state: &State,
        line_numbers_data: &mut Option<&mut line_numbers::LineNumbersData>,
        side_by_side_panel: Option<PanelSide>,
        mut painted_prefix: Option<ansi_term::ANSIString>,
        config: &config::Config,
    ) -> (String, bool) {
        let mut ansi_strings = Vec::new();

        let output_line_numbers = line_numbers_data.is_some();
        if output_line_numbers {
            // Unified diff lines are printed in one go, but side-by-side lines
            // are printed in two parts, so do not increment line numbers when the
            // first (left) part is printed.
            let increment = !matches!(side_by_side_panel, Some(side_by_side::Left));
            if let Some((line_numbers, styles)) = line_numbers::linenumbers_and_styles(
                line_numbers_data.as_mut().unwrap(),
                state,
                config,
                increment,
            ) {
                ansi_strings.extend(line_numbers::format_and_paint_line_numbers(
                    line_numbers_data.as_ref().unwrap(),
                    side_by_side_panel,
                    styles,
                    line_numbers,
                    config,
                ))
            }
        }
        let superimposed = superimpose_style_sections(
            syntax_sections,
            diff_sections,
            config.true_color,
            config.null_syntect_style,
        );

        let mut handled_prefix = false;
        for (section_style, text) in &superimposed {
            // If requested re-insert the +/- prefix with proper styling.
            if !handled_prefix {
                if let Some(painted_prefix) = painted_prefix.take() {
                    ansi_strings.push(painted_prefix)
                }
            }

            if !text.is_empty() {
                ansi_strings.push(section_style.paint(text.as_str()));
            }
            handled_prefix = true;
        }

        // Only if syntax is empty (implies diff empty) can a line actually be empty.
        let is_empty = syntax_sections.is_empty();
        (ansi_term::ANSIStrings(&ansi_strings).to_string(), is_empty)
    }

    /// Write output buffer to output stream, and clear the buffer.
    pub fn emit(&mut self) -> std::io::Result<()> {
        write!(self.writer, "{}", self.output_buffer)?;
        self.output_buffer.clear();
        Ok(())
    }

    pub fn should_compute_syntax_highlighting(state: &State, config: &config::Config) -> bool {
        if config.syntax_theme.is_none() {
            return false;
        }
        match state {
            State::HunkMinus(_, None) => {
                config.minus_style.is_syntax_highlighted
                    || config.minus_emph_style.is_syntax_highlighted
                    || config.minus_non_emph_style.is_syntax_highlighted
            }
            State::HunkZero(_, None) => config.zero_style.is_syntax_highlighted,
            State::HunkPlus(_, None) => {
                config.plus_style.is_syntax_highlighted
                    || config.plus_emph_style.is_syntax_highlighted
                    || config.plus_non_emph_style.is_syntax_highlighted
            }
            State::HunkHeader(_, _, _, _) => true,
            State::HunkMinus(_, Some(_raw_line))
            | State::HunkZero(_, Some(_raw_line))
            | State::HunkPlus(_, Some(_raw_line)) => {
                // It is possible that the captured raw line contains an ANSI
                // style that has been mapped (via map-styles) to a delta Style
                // with syntax-highlighting.
                true
            }
            State::Blame(_) => true,
            State::GitShowFile => true,
            State::Grep => true,
            State::Unknown
            | State::CommitMeta
            | State::DiffHeader(_)
            | State::HunkMinusWrapped
            | State::HunkZeroWrapped
            | State::HunkPlusWrapped
            | State::MergeConflict(_, _)
            | State::SubmoduleLog
            | State::SubmoduleShort(_) => {
                panic!(
                    "should_compute_syntax_highlighting is undefined for state {:?}",
                    state
                )
            }
        }
    }

    /// There are some rules according to which we update line section styles that were computed
    /// during the initial edit inference pass. This function applies those rules. The rules are
    /// 1. If there are multiple diff styles in the line, then the line must have some
    ///    inferred edit operations and so, if there is a special non-emph style that is
    ///    distinct from the default style, then it should be used for the non-emph style
    ///    sections.
    /// 2. If the line constitutes a whitespace error, then the whitespace error style
    ///    should be applied to the added material.
    /// 3. If delta recognized the raw line as one containing ANSI colors that
    ///    are going to be preserved in the output, then replace delta's
    ///    computed diff styles with these styles from the raw line. (This is
    ///    how support for git's --color-moved is implemented.)
    fn update_diff_style_sections<'a>(
        lines: &'a [(String, State)],
        diff_style_sections: &mut Vec<LineSections<'a, Style>>,
        whitespace_error_style: Option<Style>,
        non_emph_style: Option<Style>,
        lines_have_homolog: &[bool],
        config: &config::Config,
    ) {
        for (((_, state), style_sections), line_has_homolog) in lines
            .iter()
            .zip_eq(diff_style_sections)
            .zip_eq(lines_have_homolog)
        {
            if let State::HunkMinus(_, Some(raw_line))
            | State::HunkZero(_, Some(raw_line))
            | State::HunkPlus(_, Some(raw_line)) = state
            {
                // raw_line is captured in handle_hunk_line under certain conditions. If we have
                // done so, then overwrite the style sections with styles parsed directly from the
                // raw line. Currently the only reason this is done is to handle a diff.colorMoved
                // line.
                *style_sections = parse_style_sections(raw_line, config);
                continue;
            }
            let line_has_emph_and_non_emph_sections =
                style_sections_contain_more_than_one_style(style_sections);
            let should_update_non_emph_styles = non_emph_style.is_some() && *line_has_homolog;
            let is_whitespace_error =
                whitespace_error_style.is_some() && is_whitespace_error(style_sections);
            for (style, _) in style_sections.iter_mut() {
                // If the line as a whole constitutes a whitespace error then highlight this
                // section if either (a) it is an emph section, or (b) the line lacks any
                // emph/non-emph distinction.

                // TODO: is this logic correct now, after introducing
                // line_has_homolog for non_emph style?
                if is_whitespace_error && (style.is_emph || !line_has_emph_and_non_emph_sections) {
                    *style = whitespace_error_style.unwrap();
                }
                // Otherwise, update the style if this is a non-emph section that needs updating.
                else if should_update_non_emph_styles && !style.is_emph {
                    *style = non_emph_style.unwrap();
                }
            }
        }
    }
}

/// Remove initial -/+ character, expand tabs as spaces, and terminate with newline.
// Terminating with newline character is necessary for many of the sublime syntax definitions to
// highlight correctly.
// See https://docs.rs/syntect/3.2.0/syntect/parsing/struct.SyntaxSetBuilder.html#method.add_from_folder
pub fn prepare(line: &str, prefix_length: usize, config: &config::Config) -> String {
    if !line.is_empty() {
        // The prefix contains -/+/space characters, added by git. We removes them now so they
        // are not present during syntax highlighting or wrapping. If --keep-plus-minus-markers
        // is in effect the prefix is re-inserted in Painter::paint_line.
        let line = line.graphemes(true).skip(prefix_length);
        format!("{}\n", expand_tabs(line, config.tab_width))
    } else {
        "\n".to_string()
    }
}

// Remove initial -/+ characters, expand tabs as spaces, retaining ANSI sequences. Terminate with
// newline character.
pub fn prepare_raw_line(raw_line: &str, prefix_length: usize, config: &config::Config) -> String {
    format!(
        "{}\n",
        ansi::ansi_preserving_slice(
            &expand_tabs(raw_line.graphemes(true), config.tab_width),
            prefix_length
        ),
    )
}

/// Expand tabs as spaces.
/// tab_width = 0 is documented to mean do not replace tabs.
pub fn expand_tabs<'a, I>(line: I, tab_width: usize) -> String
where
    I: Iterator<Item = &'a str>,
{
    if tab_width > 0 {
        let tab_replacement = " ".repeat(tab_width);
        line.map(|s| if s == "\t" { &tab_replacement } else { s })
            .collect::<String>()
    } else {
        line.collect::<String>()
    }
}

pub fn paint_minus_and_plus_lines(
    lines: MinusPlus<&Vec<(String, State)>>,
    line_numbers_data: &mut Option<LineNumbersData>,
    highlighter: &mut Option<HighlightLines>,
    output_buffer: &mut String,
    config: &config::Config,
) {
    let syntax_style_sections = MinusPlus::new(
        get_syntax_style_sections_for_lines(lines[Minus], highlighter.as_mut(), config),
        get_syntax_style_sections_for_lines(lines[Plus], highlighter.as_mut(), config),
    );
    let (mut diff_style_sections, line_alignment) = get_diff_style_sections(&lines, config);
    let lines_have_homolog = edits::make_lines_have_homolog(&line_alignment);
    Painter::update_diff_style_sections(
        lines[Minus],
        &mut diff_style_sections[Minus],
        None,
        if config.minus_non_emph_style != config.minus_emph_style {
            Some(config.minus_non_emph_style)
        } else {
            None
        },
        &lines_have_homolog[Minus],
        config,
    );
    Painter::update_diff_style_sections(
        lines[Plus],
        &mut diff_style_sections[Plus],
        Some(config.whitespace_error_style),
        if config.plus_non_emph_style != config.plus_emph_style {
            Some(config.plus_non_emph_style)
        } else {
            None
        },
        &lines_have_homolog[Plus],
        config,
    );
    if config.side_by_side {
        side_by_side::paint_minus_and_plus_lines_side_by_side(
            lines,
            syntax_style_sections,
            diff_style_sections,
            lines_have_homolog,
            line_alignment,
            line_numbers_data,
            output_buffer,
            config,
        )
    } else {
        // Unified diff mode:
        if !lines[Minus].is_empty() {
            Painter::paint_lines(
                lines[Minus],
                &syntax_style_sections[Minus],
                &diff_style_sections[Minus],
                &lines_have_homolog[Minus],
                output_buffer,
                config,
                &mut line_numbers_data.as_mut(),
                Some(config.minus_empty_line_marker_style),
                BgShouldFill::default(),
            );
        }
        if !lines[Plus].is_empty() {
            Painter::paint_lines(
                lines[Plus],
                &syntax_style_sections[Plus],
                &diff_style_sections[Plus],
                &lines_have_homolog[Plus],
                output_buffer,
                config,
                &mut line_numbers_data.as_mut(),
                Some(config.plus_empty_line_marker_style),
                BgShouldFill::default(),
            );
        }
    }
}

pub fn get_syntax_style_sections_for_lines<'a>(
    lines: &'a [(String, State)],
    highlighter: Option<&mut HighlightLines>,
    config: &config::Config,
) -> Vec<LineSections<'a, SyntectStyle>> {
    let mut line_sections = Vec::new();
    match (
        highlighter,
        lines
            .iter()
            .any(|(_, state)| Painter::should_compute_syntax_highlighting(state, config)),
    ) {
        (Some(highlighter), true) => {
            for (line, _) in lines.iter() {
                line_sections.push(
                    highlighter
                        .highlight_line(line, &config.syntax_set)
                        .unwrap(),
                );
            }
        }
        _ => {
            for (line, _) in lines.iter() {
                line_sections.push(vec![(config.null_syntect_style, line.as_str())])
            }
        }
    }
    line_sections
}

/// Get background styles to represent diff for minus and plus lines in buffer.
#[allow(clippy::type_complexity)]
fn get_diff_style_sections<'a>(
    lines: &MinusPlus<&'a Vec<(String, State)>>,
    config: &config::Config,
) -> (
    MinusPlus<Vec<LineSections<'a, Style>>>,
    Vec<(Option<usize>, Option<usize>)>,
) {
    let (minus_lines, minus_styles): (Vec<&str>, Vec<Style>) = lines[Minus]
        .iter()
        .map(|(s, state)| (s.as_str(), *config.get_style(state)))
        .unzip();
    let (plus_lines, plus_styles): (Vec<&str>, Vec<Style>) = lines[Plus]
        .iter()
        .map(|(s, state)| (s.as_str(), *config.get_style(state)))
        .unzip();
    let (minus_line_diff_style_sections, plus_line_diff_style_sections, line_alignment) =
        edits::infer_edits(
            minus_lines,
            plus_lines,
            minus_styles,
            config.minus_emph_style, // FIXME
            plus_styles,
            config.plus_emph_style, // FIXME
            &config.tokenization_regex,
            config.max_line_distance,
            config.max_line_distance_for_naively_paired_lines,
        );
    let diff_sections = MinusPlus::new(
        minus_line_diff_style_sections,
        plus_line_diff_style_sections,
    );
    (diff_sections, line_alignment)
}

fn painted_prefix(state: State, config: &config::Config) -> Option<ANSIString> {
    use DiffType::*;
    use State::*;
    match (state, config.keep_plus_minus_markers) {
        // For a combined diff, unless we are in a merge conflict, we do not honor
        // keep_plus_minus_markers -- i.e. we always emit the prefix -- because there is currently
        // no way to distinguish, say, a '+ ' line from a ' +' line, by styles alone. In a merge
        // conflict we do honor the setting because the way merge conflicts are displayed indicates
        // from which commit the lines derive.
        (HunkMinus(Combined(MergeParents::Prefix(prefix), InMergeConflict::No), _), _) => {
            Some(config.minus_style.paint(prefix))
        }
        (HunkZero(Combined(MergeParents::Prefix(prefix), InMergeConflict::No), _), _) => {
            Some(config.zero_style.paint(prefix))
        }
        (HunkPlus(Combined(MergeParents::Prefix(prefix), InMergeConflict::No), _), _) => {
            Some(config.plus_style.paint(prefix))
        }
        // But otherwise we honor keep_plus_minus_markers
        (HunkMinus(_, _), true) => Some(config.minus_style.paint("-".to_string())),
        (HunkZero(_, _), true) => Some(config.zero_style.paint(" ".to_string())),
        (HunkPlus(_, _), true) => Some(config.plus_style.paint("+".to_string())),
        _ => None,
    }
}

// Parse ANSI styles encountered in `raw_line` and apply `styles_map`.
pub fn parse_style_sections<'a>(
    raw_line: &'a str,
    config: &config::Config,
) -> LineSections<'a, Style> {
    let empty_map = HashMap::new();
    let styles_map = config.styles_map.as_ref().unwrap_or(&empty_map);
    ansi::parse_style_sections(raw_line)
        .iter()
        .map(|(original_style, s)| {
            match styles_map.get(&style::ansi_term_style_equality_key(*original_style)) {
                Some(mapped_style) => (*mapped_style, *s),
                None => (
                    Style {
                        ansi_term_style: *original_style,
                        ..Style::default()
                    },
                    *s,
                ),
            }
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub fn paint_file_path_with_line_number(
    line_number: Option<usize>,
    file_path: &str,
    pad_line_number: bool,
    separator: &str,
    terminate_with_separator: bool,
    file_style: Option<Style>,        // None means do not include file path
    line_number_style: Option<Style>, // None means do not include line number
    config: &Config,
) -> String {
    let mut file_with_line_number = Vec::new();
    if let Some(file_style) = file_style {
        let file_path = if let Some(regex_replacement) = &config.file_regex_replacement {
            regex_replacement.execute(file_path)
        } else {
            Cow::from(file_path)
        };
        file_with_line_number.push(file_style.paint(file_path))
    };
    if let Some(line_number) = line_number {
        if let Some(line_number_style) = line_number_style {
            if !file_with_line_number.is_empty() {
                file_with_line_number.push(ansi_term::ANSIString::from(separator));
            }
            file_with_line_number.push(line_number_style.paint(format!("{}", line_number)))
        }
    }
    if terminate_with_separator {
        file_with_line_number.push(ansi_term::ANSIGenericString::from(separator));
    }
    if pad_line_number {
        // If requested we pad line numbers to a width of at least
        // 3, so that we do not see any misalignment up to line
        // number 999. However, see
        // https://github.com/BurntSushi/ripgrep/issues/795 for
        // discussion about aligning grep output.
        match line_number {
            Some(n) if n < 10 => {
                file_with_line_number.push(ansi_term::ANSIGenericString::from("  "))
            }
            Some(n) if n < 100 => {
                file_with_line_number.push(ansi_term::ANSIGenericString::from(" "))
            }
            _ => {}
        }
    }
    let file_with_line_number = ansi_term::ANSIStrings(&file_with_line_number).to_string();
    match if config.hyperlinks && !file_with_line_number.is_empty() {
        utils::path::absolute_path(file_path, config)
    } else {
        None
    } {
        Some(absolute_path) => hyperlinks::format_osc8_file_hyperlink(
            absolute_path,
            line_number,
            &file_with_line_number,
            config,
        )
        .into(),
        _ => file_with_line_number,
    }
}

// edits::annotate doesn't return "coalesced" annotations (see comment there), so we can't assume
// that `sections.len() > 1 <=> (multiple styles)`.
fn style_sections_contain_more_than_one_style(sections: &[(Style, &str)]) -> bool {
    if sections.len() > 1 {
        let (first_style, _) = sections[0];
        sections.iter().any(|(style, _)| *style != first_style)
    } else {
        false
    }
}

/// True iff the line represented by `sections` constitutes a whitespace error.
// A line is a whitespace error iff it is non-empty and contains only whitespace
// characters.
// TODO: Git recognizes blank lines at end of file (blank-at-eof)
// as a whitespace error but delta does not yet.
// https://git-scm.com/docs/git-config#Documentation/git-config.txt-corewhitespace
fn is_whitespace_error(sections: &[(Style, &str)]) -> bool {
    let mut any_chars = false;
    for c in sections.iter().flat_map(|(_, s)| s.chars()) {
        if c == '\n' {
            return any_chars;
        } else if c != ' ' && c != '\t' {
            return false;
        } else {
            any_chars = true;
        }
    }
    false
}

mod superimpose_style_sections {
    use syntect::highlighting::Style as SyntectStyle;

    use crate::style::Style;
    use crate::utils::bat::terminal::to_ansi_color;

    // We have two different annotations of the same line:
    // `syntax_style_sections` contains foreground styles computed by syntect,
    // and `diff_style_sections` contains styles computed by delta reflecting
    // within-line edits. The delta styles may assign a foreground color, or
    // they may indicate that the foreground color comes from syntax
    // highlighting (the is_syntax_highlighting attribute on style::Style). This
    // function takes in the two input streams and outputs one stream with a
    // single style assigned to each character.
    pub fn superimpose_style_sections(
        syntax_style_sections: &[(SyntectStyle, &str)],
        diff_style_sections: &[(Style, &str)],
        true_color: bool,
        null_syntect_style: SyntectStyle,
    ) -> Vec<(Style, String)> {
        coalesce(
            superimpose(
                explode(syntax_style_sections)
                    .iter()
                    .zip(explode(diff_style_sections))
                    .collect::<Vec<(&(SyntectStyle, char), (Style, char))>>(),
            ),
            true_color,
            null_syntect_style,
        )
    }

    fn explode<T>(style_sections: &[(T, &str)]) -> Vec<(T, char)>
    where
        T: Copy,
    {
        let mut exploded: Vec<(T, char)> = Vec::new();
        for (style, s) in style_sections {
            for c in s.chars() {
                exploded.push((*style, c));
            }
        }
        exploded
    }

    #[allow(clippy::type_complexity)]
    fn superimpose(
        style_section_pairs: Vec<(&(SyntectStyle, char), (Style, char))>,
    ) -> Vec<((SyntectStyle, Style), char)> {
        let mut superimposed: Vec<((SyntectStyle, Style), char)> = Vec::new();
        for ((syntax_style, char_1), (style, char_2)) in style_section_pairs {
            if *char_1 != char_2 {
                panic!(
                    "String mismatch encountered while superimposing style sections: '{}' vs '{}'",
                    *char_1, char_2
                )
            }
            superimposed.push(((*syntax_style, style), *char_1));
        }
        superimposed
    }

    fn coalesce(
        style_sections: Vec<((SyntectStyle, Style), char)>,
        true_color: bool,
        null_syntect_style: SyntectStyle,
    ) -> Vec<(Style, String)> {
        let make_superimposed_style = |(syntect_style, style): (SyntectStyle, Style)| {
            if style.is_syntax_highlighted && syntect_style != null_syntect_style {
                Style {
                    ansi_term_style: ansi_term::Style {
                        foreground: to_ansi_color(syntect_style.foreground, true_color),
                        ..style.ansi_term_style
                    },
                    ..style
                }
            } else {
                style
            }
        };
        let mut coalesced: Vec<(Style, String)> = Vec::new();
        let mut style_sections = style_sections.iter();
        if let Some((style_pair, c)) = style_sections.next() {
            let mut current_string = c.to_string();
            let mut current_style_pair = style_pair;
            for (style_pair, c) in style_sections {
                if style_pair != current_style_pair {
                    let style = make_superimposed_style(*current_style_pair);
                    coalesced.push((style, current_string));
                    current_string = String::new();
                    current_style_pair = style_pair;
                }
                current_string.push(*c);
            }

            // TODO: This is not the ideal location for the following code.
            if current_string.ends_with('\n') {
                // Remove the terminating newline whose presence was necessary for the syntax
                // highlighter to work correctly.
                current_string.truncate(current_string.len() - 1);
            }
            let style = make_superimposed_style(*current_style_pair);
            coalesced.push((style, current_string));
        }
        coalesced
    }

    #[cfg(test)]
    mod tests {
        use lazy_static::lazy_static;

        use super::*;
        use ansi_term::{self, Color};
        use syntect::highlighting::Color as SyntectColor;
        use syntect::highlighting::FontStyle as SyntectFontStyle;
        use syntect::highlighting::Style as SyntectStyle;

        use crate::style::{DecorationStyle, Style};

        lazy_static! {
            static ref SYNTAX_STYLE: SyntectStyle = SyntectStyle {
                foreground: SyntectColor::BLACK,
                background: SyntectColor::BLACK,
                font_style: SyntectFontStyle::BOLD,
            };
        }
        lazy_static! {
            static ref SYNTAX_HIGHLIGHTED_STYLE: Style = Style {
                ansi_term_style: ansi_term::Style {
                    foreground: Some(Color::White),
                    background: Some(Color::White),
                    is_underline: true,
                    ..ansi_term::Style::new()
                },
                is_emph: false,
                is_omitted: false,
                is_raw: false,
                is_syntax_highlighted: true,
                decoration_style: DecorationStyle::NoDecoration,
            };
        }
        lazy_static! {
            static ref NON_SYNTAX_HIGHLIGHTED_STYLE: Style = Style {
                ansi_term_style: ansi_term::Style {
                    foreground: Some(Color::White),
                    background: Some(Color::White),
                    is_underline: true,
                    ..ansi_term::Style::new()
                },
                is_emph: false,
                is_omitted: false,
                is_raw: false,
                is_syntax_highlighted: false,
                decoration_style: DecorationStyle::NoDecoration,
            };
        }
        lazy_static! {
            static ref SUPERIMPOSED_STYLE: Style = Style {
                ansi_term_style: ansi_term::Style {
                    foreground: to_ansi_color(SyntectColor::BLACK, true),
                    background: Some(Color::White),
                    is_underline: true,
                    ..ansi_term::Style::new()
                },
                is_emph: false,
                is_omitted: false,
                is_raw: false,
                is_syntax_highlighted: true,
                decoration_style: DecorationStyle::NoDecoration,
            };
        }

        #[test]
        fn test_superimpose_style_sections_1() {
            let sections_1 = vec![(*SYNTAX_STYLE, "ab")];
            let sections_2 = vec![(*SYNTAX_HIGHLIGHTED_STYLE, "ab")];
            let superimposed = vec![(*SUPERIMPOSED_STYLE, "ab".to_string())];
            assert_eq!(
                superimpose_style_sections(&sections_1, &sections_2, true, SyntectStyle::default()),
                superimposed
            );
        }

        #[test]
        fn test_superimpose_style_sections_2() {
            let sections_1 = vec![(*SYNTAX_STYLE, "ab")];
            let sections_2 = vec![
                (*SYNTAX_HIGHLIGHTED_STYLE, "a"),
                (*SYNTAX_HIGHLIGHTED_STYLE, "b"),
            ];
            let superimposed = vec![(*SUPERIMPOSED_STYLE, String::from("ab"))];
            assert_eq!(
                superimpose_style_sections(&sections_1, &sections_2, true, SyntectStyle::default()),
                superimposed
            );
        }

        #[test]
        fn test_superimpose_style_sections_3() {
            let sections_1 = vec![(*SYNTAX_STYLE, "ab")];
            let sections_2 = vec![(*NON_SYNTAX_HIGHLIGHTED_STYLE, "ab")];
            let superimposed = vec![(*NON_SYNTAX_HIGHLIGHTED_STYLE, "ab".to_string())];
            assert_eq!(
                superimpose_style_sections(&sections_1, &sections_2, true, SyntectStyle::default()),
                superimposed
            );
        }

        #[test]
        fn test_explode() {
            let arbitrary = 0;
            assert_eq!(
                explode(&[(arbitrary, "ab")]),
                vec![(arbitrary, 'a'), (arbitrary, 'b')]
            )
        }

        #[test]
        fn test_superimpose() {
            let x = (*SYNTAX_STYLE, 'a');
            let pairs = vec![(&x, (*SYNTAX_HIGHLIGHTED_STYLE, 'a'))];
            assert_eq!(
                superimpose(pairs),
                vec![((*SYNTAX_STYLE, *SYNTAX_HIGHLIGHTED_STYLE), 'a')]
            );
        }
    }
}
