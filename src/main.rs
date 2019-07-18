#[macro_use]
extern crate error_chain;

mod bat;
mod cli;
mod config;
mod delta;
mod draw;
mod edits;
mod paint;
mod parse;
mod style;

use std::io::{self, BufRead, ErrorKind, Read, Write};
use std::process;

use ansi_term;
use atty;
use structopt::StructOpt;

use crate::bat::assets::{list_languages, HighlightingAssets};
use crate::bat::output::{OutputType, PagingMode};
use crate::delta::delta;

mod errors {
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
            SyntectError(::syntect::LoadingError);
            ParseIntError(::std::num::ParseIntError);
        }
    }
}

fn main() -> std::io::Result<()> {
    let opt = cli::Opt::from_args();

    let assets = HighlightingAssets::new();

    if opt.list_languages {
        list_languages()?;
        process::exit(0);
    } else if opt.list_themes {
        list_themes()?;
        process::exit(0);
    } else if opt.compare_themes {
        compare_themes(&assets)?;
        process::exit(0);
    }

    let paint_config = cli::process_command_line_arguments(&assets, &opt);

    let mut output_type =
        OutputType::from_mode(PagingMode::QuitIfOneScreen, Some(paint_config.pager)).unwrap();
    let mut writer = output_type.handle().unwrap();

    match delta(
        io::stdin().lock().lines().map(|l| l.unwrap()),
        &paint_config,
        &assets,
        &mut writer,
    ) {
        Err(error) => match error.kind() {
            ErrorKind::BrokenPipe => process::exit(0),
            _ => eprintln!("{}", error),
        },
        _ => (),
    };
    Ok(())
}

fn compare_themes(assets: &HighlightingAssets) -> std::io::Result<()> {
    let mut opt = cli::Opt::from_args();
    let mut input = String::new();
    if atty::is(atty::Stream::Stdin) {
        input = "\
diff --git a/tests/data/hello.c b/tests/data/hello.c
index 541e930..e23bef1 100644
--- a/tests/data/hello.c
+++ b/tests/data/hello.c
@@ -1,5 +1,5 @@
 #include <stdio.h>

 int main(int argc, char **argv) {
-    printf(\"Hello!\\n\");
+    printf(\"Hello world!\\n\");
 }"
        .to_string()
    } else {
        io::stdin().read_to_string(&mut input)?;
    }

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let mut config: config::Config;
    let style = ansi_term::Style::new().bold();

    for (theme, _) in assets.theme_set.themes.iter() {
        if opt.light && !style::is_light_theme(theme) || opt.dark && style::is_light_theme(theme) {
            continue;
        }

        writeln!(stdout, "\nTheme: {}\n", style.paint(theme))?;
        opt.theme = Some(theme.to_string());
        config = cli::process_command_line_arguments(&assets, &opt);
        let mut output_type =
            OutputType::from_mode(PagingMode::QuitIfOneScreen, Some(config.pager)).unwrap();
        let mut writer = output_type.handle().unwrap();

        delta(
            input.split("\n").map(String::from),
            &config,
            &assets,
            &mut writer,
        )?;
    }

    Ok(())
}

pub fn list_themes() -> std::io::Result<()> {
    let assets = HighlightingAssets::new();
    let themes = &assets.theme_set.themes;
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    writeln!(stdout, "Light themes:")?;
    for (theme, _) in themes.iter() {
        if style::is_light_theme(theme) {
            writeln!(stdout, "    {}", theme)?;
        }
    }
    writeln!(stdout, "Dark themes:")?;
    for (theme, _) in themes.iter() {
        if !style::is_light_theme(theme) {
            writeln!(stdout, "    {}", theme)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use console::strip_ansi_codes;

    #[test]
    fn test_added_file() {
        let input = "\
commit d28dc1ac57e53432567ec5bf19ad49ff90f0f7a5
Author: Dan Davison <dandavison7@gmail.com>
Date:   Thu Jul 11 10:41:11 2019 -0400

    .

diff --git a/a.py b/a.py
new file mode 100644
index 0000000..8c55b7d
--- /dev/null
+++ b/a.py
@@ -0,0 +1,3 @@
+# hello
+class X:
+    pass";

        let expected_output = "\
commit d28dc1ac57e53432567ec5bf19ad49ff90f0f7a5
Author: Dan Davison <dandavison7@gmail.com>
Date:   Thu Jul 11 10:41:11 2019 -0400

    .

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
added: a.py
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
────────────────────────────────────────────────────────────────────────────────

────────────────────────────────────────────────────────────────────────────────
 # hello
 class X:
     pass
";

        let mut opt = cli::Opt::from_args();
        opt.width = Some("variable".to_string());
        let assets = HighlightingAssets::new();
        let paint_config = cli::process_command_line_arguments(&assets, &opt);
        let mut writer: Vec<u8> = Vec::new();
        delta(
            input.split("\n").map(String::from),
            &paint_config,
            &assets,
            &mut writer,
        )
        .unwrap();
        let output = strip_ansi_codes(&String::from_utf8(writer).unwrap()).to_string();
        assert!(output.contains("\nadded: a.py\n"));
        if false {
            // TODO: hline width
            assert_eq!(output, expected_output);
        }
    }

    #[test]
    fn test_renamed_file() {
        let input = "\
commit 1281650789680f1009dfff2497d5ccfbe7b96526
Author: Dan Davison <dandavison7@gmail.com>
Date:   Wed Jul 17 20:40:23 2019 -0400

    rename

diff --git a/a.py b/b.py
similarity index 100%
rename from a.py
rename to b.py
";

        let mut opt = cli::Opt::from_args();
        opt.width = Some("variable".to_string());
        let assets = HighlightingAssets::new();
        let paint_config = cli::process_command_line_arguments(&assets, &opt);
        let mut writer: Vec<u8> = Vec::new();
        delta(
            input.split("\n").map(String::from),
            &paint_config,
            &assets,
            &mut writer,
        )
        .unwrap();
        let output = strip_ansi_codes(&String::from_utf8(writer).unwrap()).to_string();
        assert!(output.contains("\nrenamed: a.py ⟶   b.py\n"));
    }
}
