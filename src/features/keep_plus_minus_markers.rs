use crate::features::OptionValueFunction;

pub fn make_feature() -> Vec<(String, OptionValueFunction)> {
    builtin_feature!([(
        "keep-plus-minus-markers",
        bool,
        None,
        _opt => true
    )])
}
