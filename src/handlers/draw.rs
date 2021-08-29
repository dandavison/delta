use std::cmp::max;
use std::io::Write;

use crate::ansi;
use crate::cli::Width;
use crate::style::{DecorationStyle, Style};

pub type DrawFunction =
    dyn FnMut(&mut dyn Write, &str, &str, &Width, Style, ansi_term::Style) -> std::io::Result<()>;

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
    _line_width: &Width, // ignored
    text_style: Style,
    _decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    if text_style.is_raw {
        writeln!(writer, "{}", raw_text)?;
    } else {
        writeln!(writer, "{}", text_style.paint(text))?;
    }
    Ok(())
}

/// Write text to stream, surrounded by a box, leaving the cursor just
/// beyond the bottom right corner.
fn write_boxed(
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
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
        box_width,
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
    line_width: &Width,
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    let box_width = ansi::measure_text_width(text);
    write_boxed_with_horizontal_whisker(
        writer,
        text,
        raw_text,
        box_width,
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
    line_width: &Width,
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    _write_under_or_over_lined(
        UnderOverline::Under,
        writer,
        text,
        raw_text,
        line_width,
        text_style,
        decoration_style,
    )
}

fn write_overlined(
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    line_width: &Width,
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    _write_under_or_over_lined(
        UnderOverline::Over,
        writer,
        text,
        raw_text,
        line_width,
        text_style,
        decoration_style,
    )
}

fn write_underoverlined(
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    line_width: &Width,
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    _write_under_or_over_lined(
        UnderOverline::Underover,
        writer,
        text,
        raw_text,
        line_width,
        text_style,
        decoration_style,
    )
}

fn _write_under_or_over_lined(
    underoverline: UnderOverline,
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    line_width: &Width,
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
    let text_width = ansi::measure_text_width(text);
    let line_width = match *line_width {
        Width::Fixed(n) => max(n, text_width),
        Width::Variable => text_width,
    };
    let mut write_line: Box<dyn FnMut(&mut dyn Write) -> std::io::Result<()>> =
        Box::new(|writer| {
            write_horizontal_line(writer, line_width, text_style, decoration_style)?;
            writeln!(writer)?;
            Ok(())
        });
    match underoverline {
        UnderOverline::Under => {}
        _ => write_line(writer)?,
    }
    if text_style.is_raw {
        writeln!(writer, "{}", raw_text)?;
    } else {
        writeln!(writer, "{}", text_style.paint(text))?;
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

fn write_boxed_with_horizontal_whisker(
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    box_width: usize,
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
        box_width,
        text_style,
        decoration_style,
    )?;
    write!(writer, "{}", decoration_style.paint(up_horizontal))?;
    Ok(())
}

fn write_boxed_partial(
    writer: &mut dyn Write,
    text: &str,
    raw_text: &str,
    box_width: usize,
    text_style: Style,
    decoration_style: ansi_term::Style,
) -> std::io::Result<()> {
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
    let horizontal_edge = horizontal.repeat(box_width);
    writeln!(
        writer,
        "{}{}",
        decoration_style.paint(&horizontal_edge),
        decoration_style.paint(down_left),
    )?;
    if text_style.is_raw {
        write!(writer, "{}", raw_text)?;
    } else {
        write!(writer, "{}", text_style.paint(text))?;
    }
    write!(
        writer,
        "{}\n{}",
        decoration_style.paint(vertical),
        decoration_style.paint(&horizontal_edge),
    )
}
