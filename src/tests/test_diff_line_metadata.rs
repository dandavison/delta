#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use crate::tests::integration_test_utils::{
        make_config_from_args, run_delta, run_delta_with_diff_line_metadata,
    };

    const DIFF: &str = "\
diff --git a/f.txt b/f.txt
index 1111111..2222222 100644
--- a/f.txt
+++ b/f.txt
@@ -1,3 +1,3 @@
 a
-hello world
+hello there
 c
";

    const LOG: &str = "\
commit 8a9c3f2b1d4e5f60718293a4b5c6d7e8f9012345
Author: Someone <someone@example.com>
Date:   Mon Jan 1 00:00:00 2024 +0000

    A commit

diff --git a/f.txt b/f.txt
index 1111111..2222222 100644
--- a/f.txt
+++ b/f.txt
@@ -1,1 +1,1 @@
-hello world
+hello there
";

    /// Run delta with OSC 1717 metadata emission and render the output for
    /// asserting on the records: each `ESC ] 1717 ; … ESC \` becomes a visible
    /// `⟦…⟧`, colors are stripped.
    fn visible_metadata_records(input: &str, args: &[&str]) -> String {
        let config = make_config_from_args(args);
        let output = run_delta_with_diff_line_metadata(input, &config);
        crate::ansi::strip_ansi_codes(&output.replace("\x1b]1717;", "⟦").replace("\x1b\\", "⟧"))
    }

    #[test]
    fn test_nothing_is_emitted_when_no_host_negotiated() {
        let config = make_config_from_args(&["--width", "40"]);
        assert!(!run_delta(DIFF, &config).contains("1717"));
    }

    #[test]
    fn test_every_row_of_a_rendered_diff_carries_its_patch_identity() {
        // Content rows carry their type, new-file line and, for a deletion,
        // old-file line. The file header and the boxed hunk header each draw
        // several rows, and every one of them carries the header's record: a host
        // mapping rows to patch positions must not find a gap where a decoration
        // is drawn.
        assert_snapshot!(visible_metadata_records(DIFF, &["--width", "40"]), @"

        ⟦1;f;;;f.txt⟧f.txt
        ⟦1;f;;;f.txt⟧────────────────────────────────────────

        ⟦1;h;1;;f.txt⟧───┐
        ⟦1;h;1;;f.txt⟧1: │
        ⟦1;h;1;;f.txt⟧───┘
        ⟦1;c;1;;f.txt⟧a
        ⟦1;d;2;2;f.txt⟧hello world
        ⟦1;a;2;;f.txt⟧hello there
        ⟦1;c;3;;f.txt⟧c
        ");
    }

    #[test]
    fn test_side_by_side_cells_carry_the_line_each_displays() {
        // The two panels of one output row show two different patch lines, so the
        // row carries a record per cell: the left one's and the right one's. A
        // context line is shown in both panels, so its record appears twice.
        assert_snapshot!(
            visible_metadata_records(DIFF, &["--side-by-side", "--width", "60"]),
            @"

        ⟦1;f;;;f.txt⟧f.txt
        ⟦1;f;;;f.txt⟧────────────────────────────────────────────────────────────

        ⟦1;h;1;;f.txt⟧───┐
        ⟦1;h;1;;f.txt⟧1: │
        ⟦1;h;1;;f.txt⟧───┘
        ⟦1;c;1;;f.txt⟧│  1 │a                       ⟦1;c;1;;f.txt⟧│  1 │a
        ⟦1;d;2;2;f.txt⟧│  2 │hello world             ⟦1;a;2;;f.txt⟧│  2 │hello there
        ⟦1;c;3;;f.txt⟧│  3 │c                       ⟦1;c;3;;f.txt⟧│  3 │c
        "
        );
    }

    #[test]
    fn test_a_wrapped_line_carries_its_record_on_every_row() {
        // delta emits each row of a wrapped line separately, so a host sees them as
        // distinct rows and needs the identity on each; the counters must not
        // advance for the continuation rows.
        let diff = "\
diff --git a/f.txt b/f.txt
--- a/f.txt
+++ b/f.txt
@@ -1,1 +1,2 @@
 a
+one two three four five six seven eight nine ten
";
        assert_snapshot!(
            visible_metadata_records(diff, &["--side-by-side", "--width", "60"]),
            @"

        ⟦1;f;;;f.txt⟧f.txt
        ⟦1;f;;;f.txt⟧────────────────────────────────────────────────────────────

        ⟦1;h;1;;f.txt⟧───┐
        ⟦1;h;1;;f.txt⟧1: │
        ⟦1;h;1;;f.txt⟧───┘
        ⟦1;c;1;;f.txt⟧│  1 │a                       ⟦1;c;1;;f.txt⟧│  1 │a
        │    │                        ⟦1;a;2;;f.txt⟧│  2 │one two three four five↵
        │    │                        ⟦1;a;2;;f.txt⟧│    │ six seven eight nine t↵
        │    │                        ⟦1;a;2;;f.txt⟧│    │en
        "
        );
    }

    #[test]
    fn test_commit_rows_carry_the_commit_they_name() {
        assert_snapshot!(visible_metadata_records(LOG, &["--width", "60"]), @"
        ⟦1;C;;;8a9c3f2b1d4e5f60718293a4b5c6d7e8f9012345⟧commit 8a9c3f2b1d4e5f60718293a4b5c6d7e8f9012345
        Author: Someone <someone@example.com>
        Date:   Mon Jan 1 00:00:00 2024 +0000

            A commit


        ⟦1;f;;;f.txt⟧f.txt
        ⟦1;f;;;f.txt⟧────────────────────────────────────────────────────────────

        ⟦1;h;1;;f.txt⟧───┐
        ⟦1;h;1;;f.txt⟧1: │
        ⟦1;h;1;;f.txt⟧───┘
        ⟦1;d;1;1;f.txt⟧hello world
        ⟦1;a;1;;f.txt⟧hello there
        ");
    }

    #[test]
    fn test_a_decorated_commit_carries_its_record_on_every_row() {
        assert_snapshot!(
            visible_metadata_records(LOG, &["--width", "60", "--commit-decoration-style", "blue ol"]),
            @"
        ⟦1;C;;;8a9c3f2b1d4e5f60718293a4b5c6d7e8f9012345⟧────────────────────────────────────────────────────────────
        ⟦1;C;;;8a9c3f2b1d4e5f60718293a4b5c6d7e8f9012345⟧commit 8a9c3f2b1d4e5f60718293a4b5c6d7e8f9012345
        Author: Someone <someone@example.com>
        Date:   Mon Jan 1 00:00:00 2024 +0000

            A commit


        ⟦1;f;;;f.txt⟧f.txt
        ⟦1;f;;;f.txt⟧────────────────────────────────────────────────────────────

        ⟦1;h;1;;f.txt⟧───┐
        ⟦1;h;1;;f.txt⟧1: │
        ⟦1;h;1;;f.txt⟧───┘
        ⟦1;d;1;1;f.txt⟧hello world
        ⟦1;a;1;;f.txt⟧hello there
        "
        );
    }
}
