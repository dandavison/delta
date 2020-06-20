use crate::presets::diff_highlight;
use crate::presets::PresetValueFunction;

pub fn make_preset() -> Vec<(String, PresetValueFunction)> {
    let mut preset = diff_highlight::_make_preset(true);
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
