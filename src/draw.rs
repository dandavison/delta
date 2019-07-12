use std::io::Write;

use ansi_term::Style;
use box_drawing;

/// Write text to stream, surrounded by a box, leaving the cursor just
/// beyond the bottom right corner.
pub fn write_boxed(
    text: &str,
    box_width: usize,
    line_style: Style,
    heavy: bool,
    writer: &mut Write,
) -> std::io::Result<()> {
    _write_boxed_partial(text, box_width, line_style, heavy, writer)?;
    let up_left = if heavy {
        box_drawing::heavy::UP_LEFT
    } else {
        box_drawing::light::UP_LEFT
    };
    write!(writer, "{}", line_style.paint(up_left))?;
    Ok(())
}

pub fn write_boxed_with_line(
    text: &str,
    box_width: usize,
    line_style: Style,
    heavy: bool,
    writer: &mut Write,
) -> std::io::Result<()> {
    _write_boxed_partial(text, box_width, line_style, heavy, writer)?;
    let up_horizontal = if heavy {
        box_drawing::heavy::UP_HORIZONTAL
    } else {
        box_drawing::light::UP_HORIZONTAL
    };
    write!(writer, "{}", line_style.paint(up_horizontal))?;
    Ok(())
}

fn _write_boxed_partial(
    text: &str,
    box_width: usize,
    line_style: Style,
    heavy: bool,
    writer: &mut Write,
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
        text,
        line_style.paint(vertical),
        line_style.paint(&horizontal_edge),
    )
}
