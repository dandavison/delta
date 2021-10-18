pub mod ansi_test_utils;
pub mod integration_test_utils;
pub mod test_example_diffs;
pub mod test_utils;

#[cfg(not(test))]
pub const TESTING: bool = false;

#[cfg(test)]
pub const TESTING: bool = true;

#[test]
fn am_testing() {
    assert!(TESTING);
}
