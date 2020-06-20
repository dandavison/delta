use crate::features::FeatureValueFunction;

pub fn make_feature() -> Vec<(String, FeatureValueFunction)> {
    _make_feature(false)
}

pub fn _make_feature(bold: bool) -> Vec<(String, FeatureValueFunction)> {
    builtin_feature!([
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
