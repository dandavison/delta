use crate::features::OptionValueFunction;
use crate::options::theme;

pub fn make_feature() -> Vec<(String, OptionValueFunction)> {
    builtin_feature!([
        (
            "theme",
            String,
            None,
            _opt => theme::DEFAULT_DARK_SYNTAX_THEME
        )
    ])
}
