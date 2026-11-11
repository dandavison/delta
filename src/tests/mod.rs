pub mod ansi_test_utils;
pub mod integration_test_utils;
pub mod test_example_diffs;
#[cfg(test)]
mod test_switch_and_style_features;
pub mod test_utils;

#[cfg(not(test))]
pub const TESTING: bool = false;

#[cfg(test)]
pub const TESTING: bool = true;

#[test]
#[allow(clippy::assertions_on_constants)]
fn am_testing() {
    assert!(TESTING);
}
