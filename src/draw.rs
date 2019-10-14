use std::io::Write;

use ansi_term::Style;
use box_drawing;
use console::strip_ansi_codes;
use unicode_segmentation::UnicodeSegmentation;

/// Write text to stream, surrounded by a box, leaving the cursor just
/// beyond the bottom right corner.
pub fn write_boxed(
    writer: &mut dyn Write,
    text: &str,
    _line_width: usize, // ignored
    line_style: Style,
    heavy: bool,
) -> std::io::Result<()> {
    let up_left = if heavy {
        box_drawing::heavy::UP_LEFT
    } else {
        box_drawing::light::UP_LEFT
    };
    let box_width = strip_ansi_codes(text).graphemes(true).count() + 1;
    write_boxed_partial(writer, text, box_width, line_style, heavy)?;
    write!(writer, "{}", line_style.paint(up_left))?;
    Ok(())
}

/// Write text to stream, surrounded by a box, and extend a line from
/// the bottom right corner.
pub fn write_boxed_with_line(
    writer: &mut dyn Write,
    text: &str,
    line_width: usize,
    line_style: Style,
    heavy: bool,
) -> std::io::Result<()> {
    let box_width = strip_ansi_codes(text).graphemes(true).count() + 1;
    write_boxed_with_horizontal_whisker(writer, text, box_width, line_style, heavy)?;
    write_horizontal_line(
        writer,
        if line_width > box_width {
            line_width - box_width - 1
        } else {
            0
        },
        line_style,
        heavy,
    )?;
    Ok(())
}

pub fn write_underlined(
    writer: &mut dyn Write,
    text: &str,
    line_width: usize,
    line_style: Style,
    heavy: bool,
) -> std::io::Result<()> {
    writeln!(writer, "{}", line_style.paint(text))?;
    write_horizontal_line(writer, line_width - 1, line_style, heavy)?;
    write!(writer, "\n")?;
    Ok(())
}

fn write_horizontal_line(
    writer: &mut dyn Write,
    line_width: usize,
    line_style: Style,
    heavy: bool,
) -> std::io::Result<()> {
    let horizontal = if heavy {
        box_drawing::heavy::HORIZONTAL
    } else {
        box_drawing::light::HORIZONTAL
    };
    write!(
        writer,
        "{}",
        line_style.paint(horizontal.repeat(line_width),)
    )
}

pub fn write_boxed_with_horizontal_whisker(
    writer: &mut dyn Write,
    text: &str,
    box_width: usize,
    line_style: Style,
    heavy: bool,
) -> std::io::Result<()> {
    let up_horizontal = if heavy {
        box_drawing::heavy::UP_HORIZONTAL
    } else {
        box_drawing::light::UP_HORIZONTAL
    };
    write_boxed_partial(writer, text, box_width, line_style, heavy)?;
    write!(writer, "{}", line_style.paint(up_horizontal))?;
    Ok(())
}

fn write_boxed_partial(
    writer: &mut dyn Write,
    text: &str,
    box_width: usize,
    line_style: Style,
    heavy: bool,
) -> std::io::Result<()> {
    let horizontal = if heavy {
        box_drawing::heavy::HORIZONTAL
    } else {
        box_drawing::light::HORIZONTAL
    };
    let down_left = if heavy {
        box_drawing::heavy::DOWN_LEFT
    } else {
        box_drawing::light::DOWN_LEFT
    };
    let vertical = if heavy {
        box_drawing::heavy::VERTICAL
    } else {
        box_drawing::light::VERTICAL
    };

    let horizontal_edge = horizontal.repeat(box_width);
    write!(
        writer,
        "{}{}\n{} {}\n{}",
        line_style.paint(&horizontal_edge),
        line_style.paint(down_left),
        line_style.paint(text),
        line_style.paint(vertical),
        line_style.paint(&horizontal_edge),
    )
}
