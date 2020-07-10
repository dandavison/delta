use crate::features::OptionValueFunction;

pub fn make_feature() -> Vec<(String, OptionValueFunction)> {
    builtin_feature!([
        (
            "commit-decoration-style",
            String,
            None,
            _opt => "none"
        ),
        (
            "commit-style",
            String,
            None,
            _opt => "raw"
        ),
        (
            "file-decoration-style",
            String,
            None,
            _opt => "none"
        ),
        (
            "file-style",
            String,
            None,
            _opt => "raw"
        ),
        (
            "hunk-header-decoration-style",
            String,
            None,
            _opt => "none"
        ),
        (
            "hunk-header-style",
            String,
            None,
            _opt => "raw"
        ),
        (
            "minus-style",
            String,
            Some("color.diff.old"),
            _opt => "red"
        ),
        (
            "minus-emph-style",
            String,
            Some("color.diff.old"),
            _opt => "red"
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
            _opt => "green"
        ),
        (
            "plus-emph-style",
            String,
            Some("color.diff.new"),
            _opt => "green"
        ),
        (
            "keep-plus-minus-markers",
            bool,
            None,
            _opt => true
        ),
        (
            "tabs",
            usize,
            None,
            _opt => 0
        )
    ])
}
