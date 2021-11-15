use std::io::{self, ErrorKind, Read, Write};

use structopt::StructOpt;

use crate::cli;
use crate::config;
use crate::delta;
use crate::options::theme::is_light_syntax_theme;
use crate::utils::bat::assets::HighlightingAssets;
use crate::utils::bat::output::{OutputType, PagingMode};

#[cfg(not(tarpaulin_include))]
pub fn show_syntax_themes() -> std::io::Result<()> {
    let assets = HighlightingAssets::new();
    let mut output_type = OutputType::from_mode(
        PagingMode::QuitIfOneScreen,
        None,
        &config::Config::from(cli::Opt::from_args()),
    )
    .unwrap();
    let mut writer = output_type.handle().unwrap();

    let stdin_data = if !atty::is(atty::Stream::Stdin) {
        let mut buf = Vec::new();
        io::stdin().lock().read_to_end(&mut buf)?;
        if !buf.is_empty() {
            Some(buf)
        } else {
            None
        }
    } else {
        None
    };

    let make_opt = || {
        let mut opt = cli::Opt::from_args();
        opt.computed.syntax_set = assets.syntax_set.clone();
        opt
    };
    let opt = make_opt();

    if !(opt.dark || opt.light) {
        _show_syntax_themes(opt, false, &mut writer, stdin_data.as_ref())?;
        _show_syntax_themes(make_opt(), true, &mut writer, stdin_data.as_ref())?;
    } else if opt.light {
        _show_syntax_themes(opt, true, &mut writer, stdin_data.as_ref())?;
    } else {
        _show_syntax_themes(opt, false, &mut writer, stdin_data.as_ref())?
    };
    Ok(())
}

fn _show_syntax_themes(
    mut opt: cli::Opt,
    is_light_mode: bool,
    writer: &mut dyn Write,
    stdin: Option<&Vec<u8>>,
) -> std::io::Result<()> {
    use bytelines::ByteLines;
    use std::io::BufReader;
    let input = match stdin {
        Some(stdin_data) => &stdin_data[..],
        None => {
            b"\
diff --git a/example.rs b/example.rs
index f38589a..0f1bb83 100644
--- a/example.rs
+++ b/example.rs
@@ -1,5 +1,5 @@
-// Output the square of a number.
-fn print_square(num: f64) {
-    let result = f64::powf(num, 2.0);
-    println!(\"The square of {:.2} is {:.2}.\", num, result);
+// Output the cube of a number.
+fn print_cube(num: f64) {
+    let result = f64::powf(num, 3.0);
+    println!(\"The cube of {:.2} is {:.2}.\", num, result);
"
        }
    };

    opt.computed.is_light_mode = is_light_mode;
    let mut config = config::Config::from(opt);
    let title_style = ansi_term::Style::new().bold();
    let assets = HighlightingAssets::new();

    for syntax_theme in assets
        .theme_set
        .themes
        .iter()
        .filter(|(t, _)| is_light_syntax_theme(t) == is_light_mode)
        .map(|(t, _)| t)
    {
        writeln!(
            writer,
            "\n\nSyntax theme: {}\n",
            title_style.paint(syntax_theme)
        )?;
        config.syntax_theme = Some(assets.theme_set.themes[syntax_theme.as_str()].clone());
        if let Err(error) =
            delta::delta(ByteLines::new(BufReader::new(&input[0..])), writer, &config)
        {
            match error.kind() {
                ErrorKind::BrokenPipe => std::process::exit(0),
                _ => eprintln!("{}", error),
            }
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Seek, SeekFrom};

    use super::*;
    use crate::ansi;
    use crate::tests::integration_test_utils;

    #[test]
    #[ignore] // Not working (timing out) when run by tarpaulin, presumably due to stdin detection.
    fn test_show_syntax_themes() {
        let opt = integration_test_utils::make_options_from_args(&[]);

        let mut writer = Cursor::new(vec![0; 1024]);
        _show_syntax_themes(opt, true, &mut writer, None).unwrap();
        let mut s = String::new();
        writer.seek(SeekFrom::Start(0)).unwrap();
        writer.read_to_string(&mut s).unwrap();
        let s = ansi::strip_ansi_codes(&s);
        assert!(s.contains("\nSyntax theme: gruvbox-light\n"));
        println!("{}", s);
        assert!(s.contains("\nfn print_cube(num: f64) {\n"));
    }
}
