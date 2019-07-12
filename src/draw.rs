use std::io::Write;

use ansi_term::Style;
use box_drawing;

/// Write text to stream, surrounded by a box, leaving the cursor just
/// beyond the bottom right corner.
pub fn write_boxed(
    text: &str,
    box_width: usize,
    box_style: Style,
    writer: &mut Write,
) -> std::io::Result<()> {
    _write_boxed_partial(text, box_width, box_style, writer)?;
    write!(writer, "{}", box_style.paint(box_drawing::light::UP_LEFT))?;
    Ok(())
}

pub fn write_boxed_with_line(
    text: &str,
    box_width: usize,
    box_style: Style,
    writer: &mut Write,
) -> std::io::Result<()> {
    _write_boxed_partial(text, box_width, box_style, writer)?;
    write!(
        writer,
        "{}",
        box_style.paint(box_drawing::light::UP_HORIZONTAL)
    )?;
    Ok(())
}

fn _write_boxed_partial(
    text: &str,
    box_width: usize,
    box_style: Style,
    writer: &mut Write,
) -> std::io::Result<()> {
    let horizontal_edge = box_drawing::light::HORIZONTAL.repeat(box_width);
    write!(
        writer,
        "{}{}\n{} {}\n{}",
        box_style.paint(&horizontal_edge),
        box_style.paint(box_drawing::light::DOWN_LEFT),
        text,
        box_style.paint(box_drawing::light::VERTICAL),
        box_style.paint(&horizontal_edge),
    )
}
