use crate::features::OptionValueFunction;

pub fn make_feature() -> Vec<(String, OptionValueFunction)> {
    builtin_feature!([
        (
            "color-moved",
            bool,
            None,
            _opt => true
        ),
        (
            "color-moved-minus-style",
            bool,
            Some("color.diff.oldMoved"),
            _opt => "red black"
        ),
        (
            "color-moved-plus-style",
            bool,
            Some("color.diff.newMoved"),
            _opt => "green black"
        )
    ])
}
