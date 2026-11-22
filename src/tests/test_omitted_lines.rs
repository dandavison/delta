//! `omit` in `minus-style`, `plus-style` or `zero-style` hides the lines that style is for. The
//! lines that are drawn are numbered as they would be if the hidden lines were drawn.

use crate::tests::integration_test_utils::{
    make_config_from_args, run_delta_with_diff_line_metadata, DeltaTest,
};

const DIFF: &str = "\
diff --git a/a.txt b/a.txt
index 1111111..2222222 100644
--- a/a.txt
+++ b/a.txt
@@ -1,5 +1,4 @@
 one
-two
-three
+THREE
 four
-five
+FIVE
";

const LINE_NUMBERS: [&str; 5] = [
    "--line-numbers",
    "--line-numbers-left-format",
    "{nm:^4}⋮",
    "--line-numbers-right-format",
    "{np:^4}│",
];

fn with_line_numbers<'a>(args: &[&'a str]) -> Vec<&'a str> {
    LINE_NUMBERS
        .iter()
        .copied()
        .chain(args.iter().copied())
        .collect()
}

#[test]
fn minus_style_omit_hides_removed_lines() {
    DeltaTest::with_args(&with_line_numbers(&["--minus-style", "omit"]))
        .with_input(DIFF)
        .expect_after_header(
            r#"
            #indent_mark
              1 ⋮  1 │one
                ⋮  2 │THREE
              4 ⋮  3 │four
                ⋮  4 │FIVE"#,
        );
}

#[test]
fn plus_style_omit_hides_added_lines() {
    DeltaTest::with_args(&with_line_numbers(&["--plus-style", "omit"]))
        .with_input(DIFF)
        .expect_after_header(
            r#"
            #indent_mark
              1 ⋮  1 │one
              2 ⋮    │two
              3 ⋮    │three
              4 ⋮  3 │four
              5 ⋮    │five"#,
        );
}

#[test]
fn zero_style_omit_hides_unchanged_lines() {
    DeltaTest::with_args(&with_line_numbers(&["--zero-style", "omit"]))
        .with_input(DIFF)
        .expect_after_header(
            r#"
            #indent_mark
              2 ⋮    │two
              3 ⋮    │three
                ⋮  2 │THREE
              5 ⋮    │five
                ⋮  4 │FIVE"#,
        );
}

#[test]
fn minus_style_omit_side_by_side_leaves_the_left_panel_empty() {
    DeltaTest::with_args(&["--side-by-side", "--width", "40", "--minus-style", "omit"])
        .with_input(DIFF)
        .expect_after_header(
            r#"
            #indent_mark
            │  1 │one           │  1 │one
            │    │              │  2 │THREE
            │  4 │four          │  3 │four
            │    │              │  4 │FIVE"#,
        );
}

/// The OSC 1717 records of the lines drawn give their line numbers in the diff, counting the
/// lines hidden.
#[test]
fn records_of_lines_drawn_count_the_lines_hidden() {
    let records = |args: &[&str]| -> Vec<String> {
        let output = run_delta_with_diff_line_metadata(DIFF, &make_config_from_args(args));
        output
            .split("\x1b]1717;1;")
            .skip(1)
            .filter_map(|record| record.split_once(";a.txt\x1b\\").map(|(r, _)| r.to_owned()))
            .filter(|record| record.starts_with(['c', 'a', 'd']))
            .collect()
    };
    assert_eq!(
        records(&["--plus-style", "omit"]),
        ["c;1;", "d;2;2", "d;2;3", "c;3;", "d;4;5"]
    );
    assert_eq!(
        records(&["--zero-style", "omit"]),
        ["d;2;2", "d;2;3", "a;2;", "d;4;5", "a;4;"]
    );
}
