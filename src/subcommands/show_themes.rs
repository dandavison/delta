use std::io::{self, ErrorKind, Read};

use crate::cli;
use crate::config;
use crate::delta;
use crate::env::DeltaEnv;
use crate::git_config;
use crate::options::get::get_themes;
use crate::utils::bat::output::{OutputType, PagingMode};

pub fn show_themes(dark: bool, light: bool, computed_theme_is_light: bool) -> std::io::Result<()> {
    use std::io::BufReader;

    use bytelines::ByteLines;

    use super::sample_diff::DIFF;

    let mut input = DIFF.to_vec();

    if !atty::is(atty::Stream::Stdin) {
        let mut buf = Vec::new();
        io::stdin().lock().read_to_end(&mut buf)?;
        if !buf.is_empty() {
            input = buf;
        }
    };

    let env = DeltaEnv::default();
    let git_config = git_config::GitConfig::try_create(&env);
    let opt = cli::Opt::from_iter_and_git_config(
        env.clone(),
        &["", "", "--navigate", "--show-themes"],
        git_config,
    );
    let mut output_type =
        OutputType::from_mode(&env, PagingMode::Always, None, &config::Config::from(opt)).unwrap();
    let title_style = ansi_term::Style::new().bold();
    let writer = output_type.handle().unwrap();

    for theme in &get_themes(git_config::GitConfig::try_create(&env)) {
        let git_config = git_config::GitConfig::try_create(&env);
        let opt = cli::Opt::from_iter_and_git_config(
            env.clone(),
            &["", "", "--features", theme],
            git_config,
        );
        let is_dark_theme = opt.dark;
        let is_light_theme = opt.light;
        let config = config::Config::from(opt);

        if (!computed_theme_is_light && is_dark_theme)
            || (computed_theme_is_light && is_light_theme)
            || (dark && light)
        {
            writeln!(writer, "\n\nTheme: {}\n", title_style.paint(theme))?;

            if let Err(error) =
                delta::delta(ByteLines::new(BufReader::new(&input[0..])), writer, &config)
            {
                match error.kind() {
                    ErrorKind::BrokenPipe => std::process::exit(0),
                    _ => eprintln!("{}", error),
                }
            }
        }
    }

    Ok(())
}
