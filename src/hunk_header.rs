use std::borrow::Cow;
use std::fmt::Write as FmtWrite;
use std::io::Write;

use crate::cli;
use crate::config::Config;
use crate::delta;
use crate::draw;
use crate::features;
use crate::paint::Painter;
use crate::parse;
use crate::style::{self, DecorationStyle};

/// Emit the hunk header, with any requested decoration.
pub fn handle_hunk_header_line(
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
        _write_hunk_header_raw(painter, line, raw_line, config)?;
    } else if config.hunk_header_style.is_omitted {
        writeln!(painter.writer)?;
    } else {
        _write_hunk_header(
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

fn _write_hunk_header_raw(
    painter: &mut Painter,
    line: &str,
    raw_line: &str,
    config: &Config,
) -> std::io::Result<()> {
    let (mut draw_fn, pad, decoration_ansi_term_style) = _get_draw_fn(config);
    if config.hunk_header_style.decoration_style != DecorationStyle::NoDecoration {
        writeln!(painter.writer)?;
    }
    draw_fn(
        painter.writer,
        &format!("{}{}", line, if pad { " " } else { "" }),
        &format!("{}{}", raw_line, if pad { " " } else { "" }),
        &config.decorations_width,
        config.hunk_header_style,
        decoration_ansi_term_style,
    )?;
    Ok(())
}

fn _write_hunk_header(
    raw_code_fragment: &str,
    line_numbers: &Vec<(usize, usize)>,
    painter: &mut Painter,
    line: &str,
    plus_file: &str,
    config: &Config,
) -> std::io::Result<()> {
    let (mut draw_fn, _, decoration_ansi_term_style) = _get_draw_fn(config);
    let line = if config.color_only {
        format!(" {}", &line)
    } else if !raw_code_fragment.is_empty() {
        format!("{} ", raw_code_fragment)
    } else {
        "".to_string()
    };

    // Add a blank line below the hunk-header-line for readability, unless
    // color_only mode is active.
    if !config.color_only {
        writeln!(painter.writer)?;
    }

    let mut have_hunk_header = false;
    if config.hunk_header_style_include_file_path {
        let _ = write!(
            &mut painter.output_buffer,
            "{}",
            config.file_style.paint(plus_file),
        );
        have_hunk_header = true;
    };
    if false
        && !config.line_numbers
        && config.line_numbers_show_first_line_number
        && !config.hunk_header_style.is_raw
        && !config.color_only
    {
        if have_hunk_header {
            let _ = write!(&mut painter.output_buffer, ":");
        }
        _write_line_number(&line_numbers, painter, plus_file, config)?;
        have_hunk_header = true;
    }
    if !line.is_empty() {
        if have_hunk_header {
            let _ = write!(&mut painter.output_buffer, ": ");
        }
        let lines = vec![(line, delta::State::HunkHeader)];
        let syntax_style_sections = Painter::get_syntax_style_sections_for_lines(
            &lines,
            &delta::State::HunkHeader,
            &mut painter.highlighter,
            &painter.config,
        );
        Painter::paint_lines(
            syntax_style_sections,
            vec![vec![(config.hunk_header_style, &lines[0].0)]], // TODO: compute style from state
            [delta::State::HunkHeader].iter(),
            &mut painter.output_buffer,
            config,
            &mut None,
            None,
            None,
            Some(false),
        );
        painter.output_buffer.pop(); // trim newline
        have_hunk_header = true;
    } else if have_hunk_header {
        let _ = write!(&mut painter.output_buffer, " ");
    }
    if have_hunk_header {
        draw_fn(
            painter.writer,
            &painter.output_buffer,
            &painter.output_buffer,
            &config.decorations_width,
            config.null_style,
            decoration_ansi_term_style,
        )?;
        painter.output_buffer.clear();
    }

    Ok(())
}

fn _get_draw_fn(
    config: &Config,
) -> (
    Box<
        dyn FnMut(
            &mut dyn Write,
            &str,
            &str,
            &cli::Width,
            style::Style,
            ansi_term::Style,
        ) -> std::io::Result<()>,
    >,
    bool,
    ansi_term::Style,
) {
    match config.hunk_header_style.decoration_style {
        DecorationStyle::Box(style) => (Box::new(draw::write_boxed), true, style),
        DecorationStyle::BoxWithUnderline(style) => {
            (Box::new(draw::write_boxed_with_underline), true, style)
        }
        DecorationStyle::BoxWithOverline(style) => {
            // TODO: not implemented
            (Box::new(draw::write_boxed), true, style)
        }
        DecorationStyle::BoxWithUnderOverline(style) => {
            // TODO: not implemented
            (Box::new(draw::write_boxed), true, style)
        }
        DecorationStyle::Underline(style) => (Box::new(draw::write_underlined), false, style),
        DecorationStyle::Overline(style) => (Box::new(draw::write_overlined), false, style),
        DecorationStyle::UnderOverline(style) => {
            (Box::new(draw::write_underoverlined), false, style)
        }
        DecorationStyle::NoDecoration => (
            Box::new(draw::write_no_decoration),
            false,
            ansi_term::Style::new(),
        ),
    }
}

fn _write_line_number(
    line_numbers: &Vec<(usize, usize)>,
    painter: &mut Painter,
    plus_file: &str,
    config: &Config,
) -> std::io::Result<()> {
    let plus_line_number = line_numbers[line_numbers.len() - 1].0;
    let formatted_plus_line_number = if config.hyperlinks {
        features::hyperlinks::format_osc8_file_hyperlink(
            plus_file,
            Some(plus_line_number),
            &format!("{}", plus_line_number),
            config,
        )
    } else {
        Cow::from(format!("{}", plus_line_number))
    };
    match config.hunk_header_style.decoration_ansi_term_style() {
        Some(style) => {
            let _ = write!(
                &mut painter.output_buffer,
                "{}",
                style.paint(formatted_plus_line_number)
            );
        }
        None => {
            let _ = write!(&mut painter.output_buffer, "{}", formatted_plus_line_number);
        }
    }
    Ok(())
}
