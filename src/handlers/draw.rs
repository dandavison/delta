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
            (Box::new(write_boxed_with_overline), true, style)
        }
        DecorationStyle::BoxWithUnderOverline(style) => {
            (Box::new(write_boxed_with_underoverline), true, style)
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
    if text_style.is_raw {
        writeln!(writer, "{raw_text}")?;
    } else {
        writeln!(writer, "{}", paint_text(text_style, text, addendum))?;
    }
    Ok(())
}

/// Write text to stream, surrounded by a box, leaving the cursor just
/// beyond the bottom right corner.
pub fn write_boxed(
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    addendum: &str,
    line_width: &Width, // ignored
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    _write_boxed(
        false,
        writer,
        text,
        raw_text,
        addendum,
        line_width,
        text_style,
        decoration_style,
    )
}

/// Write text to stream, surrounded by a box, and extend a whisker from
/// the top right corner.
fn write_boxed_with_overline(
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    addendum: &str,
    line_width: &Width, // ignored
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    _write_boxed(
        true,
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
fn _write_boxed(
    overline: bool,
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    addendum: &str,
    _line_width: &Width, // ignored
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    let up_left = if decoration_style.is_bold {
        box_drawing::heavy::UP_LEFT
    } else {
        box_drawing::light::UP_LEFT
    };
    let box_width = ansi::measure_text_width(text);
    write_boxed_partial(
        writer,
        text,
        raw_text,
        addendum,
        box_width,
        overline,
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
    _write_boxed_with_underline(
        false,
        writer,
        text,
        raw_text,
        addendum,
        line_width,
        text_style,
        decoration_style,
    )
}

/// Write text to stream, surrounded by a box, extend a whisker from the
/// top right corner, and extend a line from the bottom right corner.
fn write_boxed_with_underoverline(
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    addendum: &str,
    line_width: &Width,
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    _write_boxed_with_underline(
        true,
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
fn _write_boxed_with_underline(
    overline: bool,
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    addendum: &str,
    line_width: &Width,
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    let box_width = ansi::measure_text_width(text);
    write_boxed_with_horizontal_whisker(
        writer,
        text,
        raw_text,
        addendum,
        box_width,
        overline,
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
    let text_width = ansi::measure_text_width(text);
    let line_width = match *line_width {
        Width::Fixed(n) => max(n, text_width),
        Width::Variable => text_width,
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
    if text_style.is_raw {
        writeln!(writer, "{raw_text}")?;
    } else {
        writeln!(writer, "{}", paint_text(text_style, text, addendum))?;
    }
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

#[allow(clippy::too_many_arguments)]
fn write_boxed_with_horizontal_whisker(
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    addendum: &str,
    box_width: usize,
    overline: bool,
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    let up_horizontal = if decoration_style.is_bold {
        box_drawing::heavy::UP_HORIZONTAL
    } else {
        box_drawing::light::UP_HORIZONTAL
    };
    write_boxed_partial(
        writer,
        text,
        raw_text,
        addendum,
        box_width,
        overline,
        text_style,
        decoration_style,
    )?;
    write!(writer, "{}", decoration_style.paint(up_horizontal))?;
    Ok(())
}

/// Write the top edge of the box, the text row, and the bottom edge of the box,
/// leaving the cursor at the end of the bottom edge. If `overline` is true then
/// the top right corner is a T-junction with a short horizontal whisker,
/// otherwise it is a plain corner.
#[allow(clippy::too_many_arguments)]
fn write_boxed_partial(
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    addendum: &str,
    box_width: usize,
    overline: bool,
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    let (horizontal, down_left, down_horizontal, vertical) = if decoration_style.is_bold {
        (
            box_drawing::heavy::HORIZONTAL,
            box_drawing::heavy::DOWN_LEFT,
            box_drawing::heavy::DOWN_HORIZONTAL,
            box_drawing::heavy::VERTICAL,
        )
    } else {
        (
            box_drawing::light::HORIZONTAL,
            box_drawing::light::DOWN_LEFT,
            box_drawing::light::DOWN_HORIZONTAL,
            box_drawing::light::VERTICAL,
        )
    };
    let top_right_corner = if overline {
        format!("{down_horizontal}{horizontal}")
    } else {
        down_left.to_string()
    };
    let horizontal_edge = horizontal.repeat(box_width);
    writeln!(
        writer,
        "{}{}",
        decoration_style.paint(&horizontal_edge),
        decoration_style.paint(top_right_corner),
    )?;
    if text_style.is_raw {
        write!(writer, "{raw_text}")?;
    } else {
        write!(writer, "{}", paint_text(text_style, text, addendum))?;
    }
    write!(
        writer,
        "{}\n{}",
        decoration_style.paint(vertical),
        decoration_style.paint(&horizontal_edge),
    )
}
