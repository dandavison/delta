use std::collections::HashMap;

use crate::cli;
use crate::git_config::GitConfig;

/// A preset is a named set of command line (option, value) pairs, supplied in a git config file.
/// I.e. it might look like
///
/// [delta "decorations"]
///     commit-decoration-style = bold box ul
///     file-style = bold 19 ul
///     file-decoration-style = none
///
/// A builtin preset is a named set of command line (option, value) pairs that is built in to
/// delta. The implementation stores each value as a function, which allows the value (a) to depend
/// dynamically on the value of other command line options, and (b) to be taken from git config.
pub type BuiltinPreset = HashMap<String, PresetValueFunction>;

type PresetValueFunction = Box<dyn Fn(&cli::Opt, &Option<GitConfig>) -> OptionValue>;

pub enum OptionValue {
    Boolean(bool),
    Float(f64),
    OptionString(Option<String>),
    String(String),
    Int(usize),
}

// Construct a 2-level hash map: (preset name) -> (option name) -> (value function). A value
// function is a function that takes an Opt struct, and a git Config struct, and returns the value
// for the option.
pub fn make_builtin_presets() -> HashMap<String, BuiltinPreset> {
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

/// The macro permits the values of a builtin preset to be specified as either (a) a git config
/// entry or (b) a value, which may be computed from the other command line options (cli::Opt).
macro_rules! builtin_preset {
    ([$( ($option_name:expr, $type:ty, $git_config_key:expr, $opt:ident => $value:expr) ),*]) => {
        vec![$(
            (
                $option_name.to_string(),
                Box::new(move |$opt: &cli::Opt, git_config: &Option<GitConfig>| {
                    match (git_config, $git_config_key) {
                        (Some(git_config), Some(git_config_key)) => match git_config.get::<$type>(git_config_key) {
                            Some(value) => Some(value.into()),
                            _ => None,
                        },
                        _ => None,
                    }
                    .unwrap_or_else(|| $value.into())
                }) as PresetValueFunction
            )
        ),*]
    }
}

fn _make_diff_highlight_preset<'a>(bold: bool) -> Vec<(String, PresetValueFunction)> {
    builtin_preset!([
        (
            "minus-style",
            String,
            Some("color.diff.old"),
            _opt => if bold { "bold red" } else { "red" }
        ),
        (
            "minus-non-emph-style",
            String,
            Some("color.diff-highlight.oldNormal"),
            opt => opt.minus_style.clone()
        ),
        (
            "minus-emph-style",
            String,
            Some("color.diff-highlight.oldHighlight"),
            opt => format!("{} reverse", opt.minus_style)
        ),
        (
            "zero-style",
            String,
            None,
            _opt => "normal"
        ),
        (
            "plus-style",
            String,
            Some("color.diff.new"),
            _opt => if bold { "bold green" } else { "green" }
        ),
        (
            "plus-non-emph-style",
            String,
            Some("color.diff-highlight.newNormal"),
            opt => opt.plus_style.clone()
        ),
        (
            "plus-emph-style",
            String,
            Some("color.diff-highlight.newHighlight"),
            opt => format!("{} reverse", opt.plus_style)
        )
    ])
}

fn make_diff_highlight_preset() -> Vec<(String, PresetValueFunction)> {
    _make_diff_highlight_preset(false)
}

fn make_diff_so_fancy_preset() -> Vec<(String, PresetValueFunction)> {
    let mut preset = _make_diff_highlight_preset(true);
    preset.extend(builtin_preset!([
        (
            "minus-emph-style",
            String,
            Some("color.diff-highlight.oldHighlight"),
            _opt => "bold red 52"
        ),
        (
            "plus-emph-style",
            String,
            Some("color.diff-highlight.newHighlight"),
            _opt => "bold green 22"
        ),
        (
            "commit-style",
            String,
            None,
            _opt => "bold yellow"
        ),
        (
            "commit-decoration-style",
            String,
            None,
            _opt => "none"
        ),
        (
            "file-style",
            String,
            Some("color.diff.meta"),
            _opt => "11"
        ),
        (
            "file-decoration-style",
            String,
            None,
            _opt => "bold yellow ul ol"
        ),
        (
            "hunk-header-style",
            String,
            Some("color.diff.frag"),
            _opt => "bold syntax"
        ),
        (
            "hunk-header-decoration-style",
            String,
            None,
            _opt => "magenta box"
        )
    ]));
    preset
}

impl From<bool> for OptionValue {
    fn from(value: bool) -> Self {
        OptionValue::Boolean(value)
    }
}

impl From<OptionValue> for bool {
    fn from(value: OptionValue) -> Self {
        match value {
            OptionValue::Boolean(value) => value,
            _ => panic!(),
        }
    }
}

impl From<f64> for OptionValue {
    fn from(value: f64) -> Self {
        OptionValue::Float(value)
    }
}

impl From<OptionValue> for f64 {
    fn from(value: OptionValue) -> Self {
        match value {
            OptionValue::Float(value) => value,
            _ => panic!(),
        }
    }
}

impl From<Option<String>> for OptionValue {
    fn from(value: Option<String>) -> Self {
        OptionValue::OptionString(value)
    }
}

impl From<OptionValue> for Option<String> {
    fn from(value: OptionValue) -> Self {
        match value {
            OptionValue::OptionString(value) => value,
            _ => panic!(),
        }
    }
}

impl From<String> for OptionValue {
    fn from(value: String) -> Self {
        OptionValue::String(value)
    }
}

impl From<&str> for OptionValue {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}

impl From<OptionValue> for String {
    fn from(value: OptionValue) -> Self {
        match value {
            OptionValue::String(value) => value,
            _ => panic!(),
        }
    }
}

impl From<usize> for OptionValue {
    fn from(value: usize) -> Self {
        OptionValue::Int(value)
    }
}

impl From<OptionValue> for usize {
    fn from(value: OptionValue) -> Self {
        match value {
            OptionValue::Int(value) => value,
            _ => panic!(),
        }
    }
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
    fn test_file_modified_label() {
        let git_config_contents = b"
[delta]
    navigate = true
    file-modified-label = \"modified: \"
";
        let git_config_path = "delta__test_file_modified_label.gitconfig";

        assert_eq!(make_config(&[], None, None).file_modified_label, "");
        assert_eq!(
            make_config(&["--navigate"], None, None).file_modified_label,
            "Δ"
        );
        assert_eq!(
            make_config(&[], Some(git_config_contents), Some(git_config_path)).file_modified_label,
            "modified: "
        );

        let git_config_contents = b"
[delta \"my-navigate-preset\"]
    navigate = true
    file-modified-label = \"modified: \"
";

        assert_eq!(
            make_config(&[], Some(git_config_contents), Some(git_config_path)).file_modified_label,
            ""
        );
        assert_eq!(
            make_config(
                &["--presets", "my-navigate-preset"],
                Some(git_config_contents),
                Some(git_config_path)
            )
            .file_modified_label,
            "modified: "
        );
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

    #[test]
    fn test_whitespace_error_style() {
        let git_config_contents = b"
[color \"diff\"]
    whitespace = yellow dim ul magenta
";
        let git_config_path = "delta__test_whitespace_error_style.gitconfig";

        // Git config disabled: hard-coded delta default wins
        assert_eq!(
            make_config(&[], None, None).whitespace_error_style,
            make_style("magenta reverse")
        );

        // Unspecified by user: color.diff.whitespace wins
        assert_eq!(
            make_config(&[], Some(git_config_contents), Some(git_config_path))
                .whitespace_error_style,
            make_style("yellow dim ul magenta")
        );

        // Command line argument wins
        assert_eq!(
            make_config(
                &["--whitespace-error-style", "red reverse"],
                Some(git_config_contents),
                Some(git_config_path)
            )
            .whitespace_error_style,
            make_style("reverse red")
        );

        let git_config_contents = b"
[color \"diff\"]
    whitespace = yellow dim ul magenta

[delta]
    whitespace-error-style = blue reverse

[delta \"my-whitespace-error-style-preset\"]
    whitespace-error-style = green reverse
";

        // Command line argument wins
        assert_eq!(
            make_config(
                &["--whitespace-error-style", "red reverse"],
                Some(git_config_contents),
                Some(git_config_path)
            )
            .whitespace_error_style,
            make_style("reverse red")
        );

        // No command line argument; main [delta] section wins
        assert_eq!(
            make_config(&[], Some(git_config_contents), Some(git_config_path))
                .whitespace_error_style,
            make_style("blue reverse")
        );

        // No command line argument; preset section wins
        assert_eq!(
            make_config(
                &["--presets", "my-whitespace-error-style-preset"],
                Some(git_config_contents),
                Some(git_config_path)
            )
            .whitespace_error_style,
            make_style("reverse green")
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

    fn make_config(
        args: &[&str],
        git_config_contents: Option<&[u8]>,
        path: Option<&str>,
    ) -> config::Config {
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
