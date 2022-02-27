use std::io::{self, Write};

use itertools::Itertools;

use crate::{options::theme::is_light_syntax_theme, utils};

#[cfg(not(tarpaulin_include))]
pub fn list_syntax_themes() -> std::io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    if atty::is(atty::Stream::Stdout) {
        _list_syntax_themes_for_humans(&mut stdout)
    } else {
        _list_syntax_themes_for_machines(&mut stdout)
    }
}

pub fn _list_syntax_themes_for_humans(writer: &mut dyn Write) -> std::io::Result<()> {
    let assets = utils::bat::assets::load_highlighting_assets();

    writeln!(writer, "Light syntax themes:")?;
    for theme in assets.themes().filter(|t| is_light_syntax_theme(*t)) {
        writeln!(writer, "    {}", theme)?;
    }
    writeln!(writer, "\nDark syntax themes:")?;
    for theme in assets.themes().filter(|t| !is_light_syntax_theme(*t)) {
        writeln!(writer, "    {}", theme)?;
    }
    writeln!(
        writer,
        "\nUse delta --show-syntax-themes to demo the themes."
    )?;
    Ok(())
}

pub fn _list_syntax_themes_for_machines(writer: &mut dyn Write) -> std::io::Result<()> {
    let assets = utils::bat::assets::load_highlighting_assets();
    for theme in assets.themes().sorted_by_key(|t| is_light_syntax_theme(*t)) {
        writeln!(
            writer,
            "{}\t{}",
            if is_light_syntax_theme(theme) {
                "light"
            } else {
                "dark"
            },
            theme
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Read, Seek, SeekFrom};

    use super::*;

    #[test]
    fn test_list_syntax_themes_for_humans() {
        let mut writer = Cursor::new(vec![0; 512]);
        _list_syntax_themes_for_humans(&mut writer).unwrap();
        let mut s = String::new();
        writer.seek(SeekFrom::Start(0)).unwrap();
        writer.read_to_string(&mut s).unwrap();
        assert!(s.contains("Light syntax themes:\n"));
        assert!(s.contains("    GitHub\n"));
        assert!(s.contains("Dark syntax themes:\n"));
        assert!(s.contains("    Dracula\n"));
    }

    #[test]
    fn test_list_syntax_themes_for_machines() {
        let mut writer = Cursor::new(vec![0; 512]);
        _list_syntax_themes_for_machines(&mut writer).unwrap();
        let mut s = String::new();
        writer.seek(SeekFrom::Start(0)).unwrap();
        writer.read_to_string(&mut s).unwrap();
        assert!(s.contains("light	GitHub\n"));
        assert!(s.contains("dark	Dracula\n"));
    }
}
