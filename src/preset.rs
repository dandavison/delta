use std::collections::HashMap;

use crate::cli;
use crate::git_config::GitConfig;

type PresetValueFunction<T> = Box<dyn Fn(&cli::Opt, &Option<GitConfig>) -> T>;
pub type BuiltinPreset<T> = HashMap<String, PresetValueFunction<T>>;

// Currently the builtin presets only have String values. This default implementation is used by all
// other types.
pub trait GetValueFunctionFromBuiltinPreset {
    fn get_value_function_from_builtin_preset<'a>(
        _option_name: &str,
        _builtin_preset: &'a BuiltinPreset<String>,
    ) -> Option<&'a PresetValueFunction<Self>>
    where
        Self: Sized,
    {
        None
    }
}

impl GetValueFunctionFromBuiltinPreset for String {
    fn get_value_function_from_builtin_preset<'a>(
        option_name: &str,
        builtin_preset: &'a BuiltinPreset<String>,
    ) -> Option<&'a PresetValueFunction<String>> {
        builtin_preset.get(option_name)
    }
}

impl GetValueFunctionFromBuiltinPreset for bool {}
impl GetValueFunctionFromBuiltinPreset for i64 {}
impl GetValueFunctionFromBuiltinPreset for usize {}
impl GetValueFunctionFromBuiltinPreset for f64 {}
impl GetValueFunctionFromBuiltinPreset for Option<String> {}

// Construct a 2-level hash map: (preset name) -> (option name) -> (value function). A value
// function is a function that takes an Opt struct, and a git Config struct, and returns the value
// for the option.
pub fn make_builtin_presets() -> HashMap<String, BuiltinPreset<String>> {
    vec![
        (
            "diff-highlight".to_string(),
            make_diff_highlight_preset().into_iter().collect(),
        ),
        (
            "diff-so-fancy".to_string(),
            make_diff_so_fancy_preset().into_iter().collect(),
        ),
    ]
    .into_iter()
    .collect()
}

fn _make_diff_highlight_preset<'a>(bold: bool) -> Vec<(String, PresetValueFunction<String>)> {
    vec![
        (
            "minus-style".to_string(),
            Box::new(move |_opt: &cli::Opt, git_config: &Option<GitConfig>| {
                match git_config {
                    Some(git_config) => git_config.get::<String>("color.diff.old"),
                    None => None,
                }
                .unwrap_or_else(|| (if bold { "bold red" } else { "red" }).to_string())
            }),
        ),
        (
            "minus-non-emph-style".to_string(),
            Box::new(|opt: &cli::Opt, git_config: &Option<GitConfig>| {
                match git_config {
                    Some(git_config) => git_config.get::<String>("color.diff-highlight.oldNormal"),
                    None => None,
                }
                .unwrap_or_else(|| opt.minus_style.clone())
            }),
        ),
        (
            "minus-emph-style".to_string(),
            Box::new(|opt: &cli::Opt, git_config: &Option<GitConfig>| {
                match git_config {
                    Some(git_config) => {
                        git_config.get::<String>("color.diff-highlight.oldHighlight")
                    }
                    None => None,
                }
                .unwrap_or_else(|| format!("{} reverse", opt.minus_style))
            }),
        ),
        (
            "zero-style".to_string(),
            Box::new(|_opt: &cli::Opt, _git_config: &Option<GitConfig>| "normal".to_string()),
        ),
        (
            "plus-style".to_string(),
            Box::new(move |_opt: &cli::Opt, git_config: &Option<GitConfig>| {
                match git_config {
                    Some(git_config) => git_config.get::<String>("color.diff.new"),
                    None => None,
                }
                .unwrap_or_else(|| (if bold { "bold green" } else { "green" }).to_string())
            }),
        ),
        (
            "plus-non-emph-style".to_string(),
            Box::new(|opt: &cli::Opt, git_config: &Option<GitConfig>| {
                match git_config {
                    Some(git_config) => git_config.get::<String>("color.diff-highlight.newNormal"),
                    None => None,
                }
                .unwrap_or_else(|| opt.plus_style.clone())
            }),
        ),
        (
            "plus-emph-style".to_string(),
            Box::new(|opt: &cli::Opt, git_config: &Option<GitConfig>| {
                match git_config {
                    Some(git_config) => {
                        git_config.get::<String>("color.diff-highlight.newHighlight")
                    }
                    None => None,
                }
                .unwrap_or_else(|| format!("{} reverse", opt.plus_style))
            }),
        ),
    ]
}

fn make_diff_highlight_preset() -> Vec<(String, PresetValueFunction<String>)> {
    _make_diff_highlight_preset(false)
}

fn make_diff_so_fancy_preset() -> Vec<(String, PresetValueFunction<String>)> {
    let mut preset = _make_diff_highlight_preset(true);
    preset.push((
        "commit-style".to_string(),
        Box::new(|_opt: &cli::Opt, _git_config: &Option<GitConfig>| "bold yellow".to_string()),
    ));
    preset.push((
        "commit-decoration-style".to_string(),
        Box::new(|_opt: &cli::Opt, _git_config: &Option<GitConfig>| "none".to_string()),
    ));
    preset.push((
        "file-style".to_string(),
        Box::new(|_opt: &cli::Opt, git_config: &Option<GitConfig>| {
            match git_config {
                Some(git_config) => git_config.get::<String>("color.diff.meta"),
                None => None,
            }
            .unwrap_or_else(|| "11".to_string())
        }),
    ));
    preset.push((
        "file-decoration-style".to_string(),
        Box::new(|_opt: &cli::Opt, _git_config: &Option<GitConfig>| {
            "bold yellow ul ol".to_string()
        }),
    ));
    preset.push((
        "hunk-header-style".to_string(),
        Box::new(|_opt: &cli::Opt, git_config: &Option<GitConfig>| {
            match git_config {
                Some(git_config) => git_config.get::<String>("color.diff.frag"),
                None => None,
            }
            .unwrap_or_else(|| "bold syntax".to_string())
        }),
    ));
    preset.push((
        "hunk-header-decoration-style".to_string(),
        Box::new(|_opt: &cli::Opt, _git_config: &Option<GitConfig>| "magenta box".to_string()),
    ));
    preset
}

#[cfg(test)]
mod tests {
    use std::fs::{remove_file, File};
    use std::io::Write;
    use std::path::Path;

    use itertools;

    use crate::config;
    use crate::git_config::GitConfig;
    use crate::style::{DecorationStyle, Style};

    #[test]
    fn test_main_section() {
        let git_config_contents = b"
[delta]
    minus-style = blue
";
        let git_config_path = "delta__test_main_section.gitconfig";

        // First check that it doesn't default to blue, because that's going to be used to signal
        // that gitconfig has set the style.
        assert_ne!(make_config(&[], None, None).minus_style, make_style("blue"));

        // Check that --minus-style is honored as we expect.
        assert_eq!(
            make_config(&["--minus-style", "red"], None, None).minus_style,
            make_style("red")
        );

        // Check that gitconfig does not override a command line argument
        assert_eq!(
            make_config(
                &["--minus-style", "red"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("red")
        );

        // Finally, check that gitconfig is honored when not overridden by a command line argument.
        assert_eq!(
            make_config(&[], Some(git_config_contents), Some(git_config_path)).minus_style,
            make_style("blue")
        );

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_preset() {
        let git_config_contents = b"
[delta]
    minus-style = blue

[delta \"my-preset\"]
    minus-style = green
";
        let git_config_path = "delta__test_preset.gitconfig";

        // Without --presets the main section takes effect
        assert_eq!(
            make_config(&[], Some(git_config_contents), Some(git_config_path)).minus_style,
            make_style("blue")
        );

        // With --presets the preset takes effect
        assert_eq!(
            make_config(
                &["--presets", "my-preset"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("green")
        );
        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_multiple_presets() {
        let git_config_contents = b"
[delta]
    minus-style = blue

[delta \"my-preset-1\"]
    minus-style = green

[delta \"my-preset-2\"]
    minus-style = yellow
";
        let git_config_path = "delta__test_multiple_presets.gitconfig";

        assert_eq!(
            make_config(
                &["--presets", "my-preset-1"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("green")
        );

        assert_eq!(
            make_config(
                &["--presets", "my-preset-1 my-preset-2"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("yellow")
        );

        assert_eq!(
            make_config(
                &["--presets", "my-preset-2 my-preset-1"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("green")
        );

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_invalid_presets() {
        let git_config_contents = b"
[delta]
    minus-style = blue

[delta \"my-preset-1\"]
    minus-style = green

[delta \"my-preset-2\"]
    minus-style = yellow
";
        let git_config_path = "delta__test_invalid_presets.gitconfig";

        assert_eq!(
            make_config(
                &["--presets", "my-preset-1"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("green")
        );

        assert_eq!(
            make_config(
                &["--presets", "my-preset-x"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("blue")
        );

        assert_eq!(
            make_config(
                &["--presets", "my-preset-1 my-preset-x"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("green")
        );

        assert_eq!(
            make_config(
                &["--presets", "my-preset-x my-preset-2 my-preset-x"],
                Some(git_config_contents),
                Some(git_config_path),
            )
            .minus_style,
            make_style("yellow")
        );

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_diff_highlight_defaults() {
        let config = make_config(&["--presets", "diff-highlight"], None, None);

        assert_eq!(config.minus_style, make_style("red"));
        assert_eq!(config.minus_non_emph_style, make_style("red"));
        assert_eq!(config.minus_emph_style, make_emph_style("red reverse"));
        assert_eq!(config.zero_style, make_style(""));
        assert_eq!(config.plus_style, make_style("green"));
        assert_eq!(config.plus_non_emph_style, make_style("green"));
        assert_eq!(config.plus_emph_style, make_emph_style("green reverse"));
    }

    #[test]
    fn test_diff_highlight_respects_gitconfig() {
        let git_config_contents = b"
[color \"diff\"]
    old = red bold
    new = green bold

[color \"diff-highlight\"]
    oldNormal = ul red bold
    oldHighlight = red bold 52
    newNormal = ul green bold
    newHighlight = green bold 22
";
        let git_config_path = "delta__test_diff_highlight.gitconfig";

        let config = make_config(
            &["--presets", "diff-highlight"],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(config.minus_style, make_style("red bold"));
        assert_eq!(config.minus_non_emph_style, make_style("ul red bold"));
        assert_eq!(config.minus_emph_style, make_emph_style("red bold 52"));
        assert_eq!(config.zero_style, make_style(""));
        assert_eq!(config.plus_style, make_style("green bold"));
        assert_eq!(config.plus_non_emph_style, make_style("ul green bold"));
        assert_eq!(config.plus_emph_style, make_emph_style("green bold 22"));

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_diff_so_fancy_defaults() {
        let config = make_config(&["--presets", "diff-so-fancy"], None, None);

        assert_eq!(
            config.commit_style.ansi_term_style,
            make_style("bold yellow").ansi_term_style
        );
        assert_eq!(
            config.commit_style.decoration_style,
            make_decoration_style("none")
        );

        assert_eq!(
            config.file_style.ansi_term_style,
            make_style("11").ansi_term_style
        );
        assert_eq!(
            config.file_style.decoration_style,
            make_decoration_style("bold yellow ul ol")
        );

        assert_eq!(
            config.hunk_header_style.ansi_term_style,
            make_style("bold syntax").ansi_term_style
        );
        assert_eq!(
            config.hunk_header_style.decoration_style,
            make_decoration_style("magenta box")
        );
    }

    #[test]
    fn test_diff_so_fancy_respects_git_config() {
        let git_config_contents = b"
[color \"diff\"]
    meta = 11
    frag = magenta bold
    commit = yellow bold
    old = red bold
    new = green bold
    whitespace = red reverse
";
        let git_config_path = "delta__test_diff_so_fancy.gitconfig";

        let config = make_config(
            &["--presets", "diff-so-fancy some-other-preset"],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(
            config.commit_style.ansi_term_style,
            make_style("yellow bold").ansi_term_style
        );
        assert_eq!(
            config.file_style.ansi_term_style,
            make_style("11").ansi_term_style
        );
        assert_eq!(
            config.hunk_header_style.ansi_term_style,
            make_style("magenta bold").ansi_term_style
        );
        assert_eq!(
            config.commit_style.decoration_style,
            make_decoration_style("none")
        );
        assert_eq!(
            config.file_style.decoration_style,
            make_decoration_style("yellow bold ul ol")
        );
        assert_eq!(
            config.hunk_header_style.decoration_style,
            make_decoration_style("magenta box")
        );

        remove_file(git_config_path).unwrap();
    }

    #[test]
    fn test_diff_so_fancy_obeys_preset_precedence_rules() {
        let git_config_contents = b"
[color \"diff\"]
    meta = 11
    frag = magenta bold
    commit = yellow bold
    old = red bold
    new = green bold
    whitespace = red reverse

[delta \"decorations\"]
    commit-decoration-style = bold box ul
    file-style = bold 19 ul
    file-decoration-style = none
";
        let git_config_path = "delta__test_diff_so_fancy_obeys_preset_precedence_rules.gitconfig";

        let config = make_config(
            &["--presets", "decorations diff-so-fancy"],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(
            config.file_style.ansi_term_style,
            make_style("11").ansi_term_style
        );

        assert_eq!(
            config.file_style.decoration_style,
            make_decoration_style("yellow bold ul ol")
        );

        let config = make_config(
            &["--presets", "diff-so-fancy decorations"],
            Some(git_config_contents),
            Some(git_config_path),
        );

        assert_eq!(
            config.file_style.ansi_term_style,
            make_style("ul bold 19").ansi_term_style
        );

        assert_eq!(
            config.file_style.decoration_style,
            make_decoration_style("none")
        );

        remove_file(git_config_path).unwrap();
    }

    fn make_style(s: &str) -> Style {
        _make_style(s, false)
    }

    fn make_emph_style(s: &str) -> Style {
        _make_style(s, true)
    }

    fn _make_style(s: &str, is_emph: bool) -> Style {
        Style::from_str(s, None, None, None, true, is_emph)
    }

    fn make_decoration_style(s: &str) -> DecorationStyle {
        DecorationStyle::from_str(s, true)
    }

    fn make_git_config(contents: &[u8], path: &str) -> GitConfig {
        let path = Path::new(path);
        let mut file = File::create(path).unwrap();
        file.write_all(contents).unwrap();
        GitConfig::from_path(&path)
    }

    fn make_config<'a>(
        args: &[&str],
        git_config_contents: Option<&[u8]>,
        path: Option<&str>,
    ) -> config::Config<'a> {
        let args: Vec<&str> = itertools::chain(
            &["/dev/null", "/dev/null", "--24-bit-color", "always"],
            args,
        )
        .map(|s| *s)
        .collect();
        let mut git_config = match (git_config_contents, path) {
            (Some(contents), Some(path)) => Some(make_git_config(contents, path)),
            _ => None,
        };
        config::Config::from_args(&args, &mut git_config)
    }
}
