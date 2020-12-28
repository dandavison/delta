use std::borrow::Cow;
use std::fmt::Write as FmtWrite;

use crate::config::Config;
use crate::delta;
use crate::draw;
use crate::features;
use crate::paint::Painter;
use crate::style::DecorationStyle;

pub fn write_hunk_header_raw(
    painter: &mut Painter,
    line: &str,
    raw_line: &str,
    config: &Config,
) -> std::io::Result<()> {
    let (mut draw_fn, pad, decoration_ansi_term_style) =
        draw::get_draw_function(config.hunk_header_style.decoration_style);
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

pub fn write_hunk_header(
    raw_code_fragment: &str,
    line_numbers: &[(usize, usize)],
    painter: &mut Painter,
    line: &str,
    plus_file: &str,
    config: &Config,
) -> std::io::Result<()> {
    let (mut draw_fn, _, decoration_ansi_term_style) =
        draw::get_draw_function(config.hunk_header_style.decoration_style);
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
    if !config.line_numbers
        && config.hunk_header_style_include_line_number
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

fn _write_line_number(
    line_numbers: &[(usize, usize)],
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
