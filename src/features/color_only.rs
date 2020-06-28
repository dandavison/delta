use std::collections::HashSet;

use crate::features::raw;
use crate::features::OptionValueFunction;

/// color-only is like raw but does not override these styles.
pub fn make_feature() -> Vec<(String, OptionValueFunction)> {
    let styles: HashSet<_> = [
        "minus-style",
        "minus-emph-style",
        "zero-style",
        "plus-style",
        "plus-emph-style",
    ]
    .iter()
    .collect();
    raw::make_feature()
        .into_iter()
        .filter(|(k, _)| !styles.contains(&k.as_str()))
        .collect()
}
