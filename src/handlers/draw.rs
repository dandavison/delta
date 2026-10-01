use std::cmp::max;
use std::io::Write;

use crate::ansi;
use crate::cli::Width;
use crate::style::{DecorationStyle, Style};

fn paint_text(text_style: Style, text: &str, addendum: &str) -> String {
    if addendum.is_empty() {
        text_style.paint(text).to_string()
    } else {
        text_style
            .paint(text.to_string() + " (" + addendum + ")")
            .to_string()
    }
}

/// Return `raw_text` if `text_style` is raw, and otherwise `text` and `addendum` painted in
/// `text_style`.
fn text_to_write(text: &str, raw_text: &str, addendum: &str, text_style: Style) -> String {
    if text_style.is_raw {
        raw_text.to_string()
    } else {
        paint_text(text_style, text, addendum)
    }
}

pub type DrawFunction = dyn FnMut(
    &mut dyn Write,
    &str,
    &str,
    &str,
    &Width,
    Style,
    ansi_term::Style,
) -> std::io::Result<()>;

pub fn get_draw_function(
    decoration_style: DecorationStyle,
) -> (Box<DrawFunction>, bool, ansi_term::Style) {
    match decoration_style {
        DecorationStyle::Box(style) => (Box::new(write_boxed), true, style),
        DecorationStyle::BoxWithUnderline(style) => {
            (Box::new(write_boxed_with_underline), true, style)
        }
        DecorationStyle::BoxWithOverline(style) => {
            // TODO: not implemented
            (Box::new(write_boxed), true, style)
        }
        DecorationStyle::BoxWithUnderOverline(style) => {
            // TODO: not implemented
            (Box::new(write_boxed), true, style)
        }
        DecorationStyle::Underline(style) => (Box::new(write_underlined), false, style),
        DecorationStyle::Overline(style) => (Box::new(write_overlined), false, style),
        DecorationStyle::UnderOverline(style) => (Box::new(write_underoverlined), false, style),
        DecorationStyle::NoDecoration => (
            Box::new(write_no_decoration),
            false,
            ansi_term::Style::new(),
        ),
    }
}

fn write_no_decoration(
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    addendum: &str,
    _line_width: &Width, // ignored
    text_style: Style,
    _decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    writeln!(
        writer,
        "{}",
        text_to_write(text, raw_text, addendum, text_style)
    )?;
    Ok(())
}

/// Write text to stream, surrounded by a box, leaving the cursor just
/// beyond the bottom right corner.
pub fn write_boxed(
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    addendum: &str,
    line_width: &Width,
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    let up_left = if decoration_style.is_bold {
        box_drawing::heavy::UP_LEFT
    } else {
        box_drawing::light::UP_LEFT
    };
    write_boxed_partial(
        writer,
        text,
        raw_text,
        addendum,
        line_width,
        text_style,
        decoration_style,
    )?;
    writeln!(writer, "{}", decoration_style.paint(up_left))?;
    Ok(())
}

/// Write text to stream, surrounded by a box, and extend a line from
/// the bottom right corner.
fn write_boxed_with_underline(
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    addendum: &str,
    line_width: &Width,
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    let box_width = write_boxed_with_horizontal_whisker(
        writer,
        text,
        raw_text,
        addendum,
        line_width,
        text_style,
        decoration_style,
    )?;
    let line_width = match *line_width {
        Width::Fixed(n) => n,
        Width::Variable => box_width,
    };
    write_horizontal_line(
        writer,
        if line_width > box_width {
            line_width - box_width - 1
        } else {
            0
        },
        text_style,
        decoration_style,
    )?;
    writeln!(writer)?;
    Ok(())
}

enum UnderOverline {
    Under,
    Over,
    Underover,
}

fn write_underlined(
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    addendum: &str,
    line_width: &Width,
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    _write_under_or_over_lined(
        UnderOverline::Under,
        writer,
        text,
        raw_text,
        addendum,
        line_width,
        text_style,
        decoration_style,
    )
}

fn write_overlined(
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    addendum: &str,
    line_width: &Width,
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    _write_under_or_over_lined(
        UnderOverline::Over,
        writer,
        text,
        raw_text,
        addendum,
        line_width,
        text_style,
        decoration_style,
    )
}

fn write_underoverlined(
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    addendum: &str,
    line_width: &Width,
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    _write_under_or_over_lined(
        UnderOverline::Underover,
        writer,
        text,
        raw_text,
        addendum,
        line_width,
        text_style,
        decoration_style,
    )
}

#[allow(clippy::too_many_arguments)]
fn _write_under_or_over_lined(
    underoverline: UnderOverline,
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    addendum: &str,
    line_width: &Width,
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    let text = text_to_write(text, raw_text, addendum, text_style);
    let line_width = match *line_width {
        Width::Fixed(n) => n,
        Width::Variable => ansi::measure_text_width(&text),
    };
    let write_line = |writer: &mut dyn Write| -> std::io::Result<()> {
        write_horizontal_line(writer, line_width, text_style, decoration_style)?;
        writeln!(writer)?;
        Ok(())
    };
    match underoverline {
        UnderOverline::Under => {}
        _ => write_line(writer)?,
    }
    writeln!(writer, "{text}")?;
    match underoverline {
        UnderOverline::Over => {}
        _ => write_line(writer)?,
    }
    Ok(())
}

fn write_horizontal_line(
    writer: &mut dyn Write,
    width: usize,
    _text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    let horizontal = if decoration_style.is_bold {
        box_drawing::heavy::HORIZONTAL
    } else {
        box_drawing::light::HORIZONTAL
    };
    write!(
        writer,
        "{}",
        decoration_style.paint(horizontal.repeat(width))
    )
}

/// Return the width of the box without its right edge.
fn write_boxed_with_horizontal_whisker(
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    addendum: &str,
    line_width: &Width,
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<usize> {
    let up_horizontal = if decoration_style.is_bold {
        box_drawing::heavy::UP_HORIZONTAL
    } else {
        box_drawing::light::UP_HORIZONTAL
    };
    let box_width = write_boxed_partial(
        writer,
        text,
        raw_text,
        addendum,
        line_width,
        text_style,
        decoration_style,
    )?;
    write!(writer, "{}", decoration_style.paint(up_horizontal))?;
    Ok(box_width)
}

/// Write the text surrounded by a box, leaving out the bottom right corner. If the box would be
/// wider than a fixed `line_width`, wrap the text inside the box. Return the width of the box
/// without its right edge.
fn write_boxed_partial(
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    addendum: &str,
    line_width: &Width,
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<usize> {
    let (horizontal, down_left, vertical) = if decoration_style.is_bold {
        (
            box_drawing::heavy::HORIZONTAL,
            box_drawing::heavy::DOWN_LEFT,
            box_drawing::heavy::VERTICAL,
        )
    } else {
        (
            box_drawing::light::HORIZONTAL,
            box_drawing::light::DOWN_LEFT,
            box_drawing::light::VERTICAL,
        )
    };
    let text = text_to_write(text, raw_text, addendum, text_style);
    let text_width = ansi::measure_text_width(&text);
    let (lines, box_width) = match *line_width {
        // The right edge of the box takes up one column.
        Width::Fixed(n) if text_width >= n => {
            // Keep a space between the text and the right edge on every line. This matches the
            // space that callers add to the end of the text.
            let wrap_width = max(n.saturating_sub(2), 1);
            let mut lines = ansi::wrap_str(&text, wrap_width);
            // The space that callers add to the end of the text can end up on a line of its own.
            let is_blank = |line: &String| ansi::strip_ansi_codes(line).trim().is_empty();
            while lines.len() > 1 && lines.last().is_some_and(is_blank) {
                lines.pop();
            }
            (lines, wrap_width + 1)
        }
        _ => (vec![text], text_width),
    };
    let horizontal_edge = horizontal.repeat(box_width);
    writeln!(
        writer,
        "{}{}",
        decoration_style.paint(&horizontal_edge),
        decoration_style.paint(down_left),
    )?;
    for line in &lines {
        let padding = box_width.saturating_sub(ansi::measure_text_width(line));
        writeln!(
            writer,
            "{}{}{}",
            line,
            " ".repeat(padding),
            decoration_style.paint(vertical),
        )?;
    }
    write!(writer, "{}", decoration_style.paint(&horizontal_edge))?;
    Ok(box_width)
}
