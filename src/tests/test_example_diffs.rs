#[cfg(test)]
mod tests {
    use console::strip_ansi_codes;

    use crate::tests::ansi_test_utils::ansi_test_utils;
    use crate::tests::integration_test_utils::integration_test_utils;

    #[test]
    fn test_added_file() {
        let options = integration_test_utils::get_command_line_options();
        let (output, _) = integration_test_utils::run_delta(ADDED_FILE_INPUT, options);
        let output = strip_ansi_codes(&output);
        assert!(output.contains("\nadded: a.py\n"));
        if false {
            // TODO: hline width
            assert_eq!(output, ADDED_FILE_EXPECTED_OUTPUT);
        }
    }

    #[test]
    #[ignore] // #128
    fn test_added_empty_file() {
        let options = integration_test_utils::get_command_line_options();
        let (output, _) = integration_test_utils::run_delta(ADDED_EMPTY_FILE, options);
        let output = strip_ansi_codes(&output);
        assert!(output.contains("\nadded: file\n"));
    }

    #[test]
    fn test_added_file_directory_path_containing_space() {
        let options = integration_test_utils::get_command_line_options();
        let (output, _) =
            integration_test_utils::run_delta(ADDED_FILES_DIRECTORY_PATH_CONTAINING_SPACE, options);
        let output = strip_ansi_codes(&output);
        assert!(output.contains("\nadded: with space/file1\n"));
        assert!(output.contains("\nadded: nospace/file2\n"));
    }

    #[test]
    fn test_renamed_file() {
        let options = integration_test_utils::get_command_line_options();
        let (output, _) = integration_test_utils::run_delta(RENAMED_FILE_INPUT, options);
        let output = strip_ansi_codes(&output);
        assert!(output.contains("\nrenamed: a.py ⟶   b.py\n"));
    }

    #[test]
    fn test_recognized_file_type() {
        // In addition to the background color, the code has language syntax highlighting.
        let options = integration_test_utils::get_command_line_options();
        let (output, config) = integration_test_utils::get_line_of_code_from_delta(
            &ADDED_FILE_INPUT,
            12,
            " class X:",
            options,
        );
        ansi_test_utils::assert_has_color_other_than_plus_color(&output, &config);
    }

    #[test]
    fn test_unrecognized_file_type_with_theme() {
        // In addition to the background color, the code has the foreground color using the default
        // .txt syntax under the theme.
        let options = integration_test_utils::get_command_line_options();
        let input = ADDED_FILE_INPUT.replace("a.py", "a");
        let (output, config) =
            integration_test_utils::get_line_of_code_from_delta(&input, 12, " class X:", options);
        ansi_test_utils::assert_has_color_other_than_plus_color(&output, &config);
    }

    #[test]
    fn test_unrecognized_file_type_no_theme() {
        // The code has the background color only. (Since there is no theme, the code has no
        // foreground ansi color codes.)
        let mut options = integration_test_utils::get_command_line_options();
        options.theme = Some("none".to_string());
        let input = ADDED_FILE_INPUT.replace("a.py", "a");
        let (output, config) =
            integration_test_utils::get_line_of_code_from_delta(&input, 12, " class X:", options);
        ansi_test_utils::assert_has_plus_color_only(&output, &config);
    }

    #[test]
    fn test_diff_unified_two_files() {
        let options = integration_test_utils::get_command_line_options();
        let (output, _) = integration_test_utils::run_delta(DIFF_UNIFIED_TWO_FILES, options);
        let output = strip_ansi_codes(&output);
        let mut lines = output.split('\n');

        // Header
        assert_eq!(lines.nth(1).unwrap(), "comparing: one.rs ⟶   src/two.rs");
        // Line
        assert_eq!(lines.nth(2).unwrap(), "5");
        // Change
        assert_eq!(lines.nth(2).unwrap(), " println!(\"Hello ruster\");");
        // Next chunk
        assert_eq!(lines.nth(2).unwrap(), "43");
        // Unchanged in second chunk
        assert_eq!(lines.nth(2).unwrap(), " Unchanged");
    }

    #[test]
    fn test_diff_unified_two_directories() {
        let options = integration_test_utils::get_command_line_options();
        let (output, _) = integration_test_utils::run_delta(DIFF_UNIFIED_TWO_DIRECTORIES, options);
        let output = strip_ansi_codes(&output);
        let mut lines = output.split('\n');

        // Header
        assert_eq!(
            lines.nth(1).unwrap(),
            "comparing: a/different ⟶   b/different"
        );
        // Line number
        assert_eq!(lines.nth(2).unwrap(), "1");
        // Change
        assert_eq!(lines.nth(2).unwrap(), " This is different from b");
        // File uniqueness
        assert_eq!(lines.nth(2).unwrap(), "Only in a/: just_a");
        // FileMeta divider
        assert!(lines.next().unwrap().starts_with("───────"));
        // Next hunk
        assert_eq!(
            lines.nth(4).unwrap(),
            "comparing: a/more_difference ⟶   b/more_difference"
        );
    }

    #[test]
    #[ignore] // Ideally, delta would make this test pass. See #121.
    fn test_delta_ignores_non_diff_input() {
        let options = integration_test_utils::get_command_line_options();
        let (output, _) = integration_test_utils::run_delta(NOT_A_DIFF_OUTPUT, options);
        let output = strip_ansi_codes(&output);
        assert_eq!(output, NOT_A_DIFF_OUTPUT.to_owned() + "\n");
    }

    #[test]
    fn test_delta_paints_diff_when_there_is_unrecognized_initial_content() {
        for input in vec![
            DIFF_WITH_UNRECOGNIZED_PRECEDING_MATERIAL_1,
            DIFF_WITH_UNRECOGNIZED_PRECEDING_MATERIAL_2,
        ] {
            let mut options = integration_test_utils::get_command_line_options();
            options.color_only = true;
            let (output, _) = integration_test_utils::run_delta(input, options);
            assert_eq!(strip_ansi_codes(&output), input);
            assert_ne!(output, input);
        }
    }

    #[test]
    fn test_diff_with_merge_conflict_is_not_truncated() {
        let options = integration_test_utils::get_command_line_options();
        let (output, _) = integration_test_utils::run_delta(DIFF_WITH_MERGE_CONFLICT, options);
        // TODO: The + in the first column is being removed.
        assert!(strip_ansi_codes(&output).contains("+>>>>>>> Stashed changes"));
        assert_eq!(output.split('\n').count(), 46);
    }

    #[test]
    fn test_diff_with_merge_conflict_is_passed_on_unchanged_under_color_only() {
        let mut options = integration_test_utils::get_command_line_options();
        options.color_only = true;
        let (output, _) = integration_test_utils::run_delta(DIFF_WITH_MERGE_CONFLICT, options);
        assert_eq!(strip_ansi_codes(&output), DIFF_WITH_MERGE_CONFLICT);
    }

    #[test]
    fn test_submodule_contains_untracked_content() {
        let options = integration_test_utils::get_command_line_options();
        let (output, _) =
            integration_test_utils::run_delta(SUBMODULE_CONTAINS_UNTRACKED_CONTENT_INPUT, options);
        let output = strip_ansi_codes(&output);
        assert!(output.contains("\nSubmodule x/y/z contains untracked content\n"));
    }

    #[test]
    fn test_triple_dash_at_beginning_of_line_in_code() {
        let options = integration_test_utils::get_command_line_options();
        let (output, _) =
            integration_test_utils::run_delta(TRIPLE_DASH_AT_BEGINNING_OF_LINE_IN_CODE, options);
        let output = strip_ansi_codes(&output);
        assert!(
            output.contains(" -- instance (Category p, Category q) => Category (p ∧ q) where\n")
        );
    }

    #[test]
    fn test_binary_files_differ() {
        let options = integration_test_utils::get_command_line_options();
        let (output, _) = integration_test_utils::run_delta(BINARY_FILES_DIFFER, options);
        let output = strip_ansi_codes(&output);
        assert!(output.contains("Binary files /dev/null and b/foo differ\n"));
    }

    #[test]
    fn test_diff_in_diff() {
        let options = integration_test_utils::get_command_line_options();
        let (output, _) = integration_test_utils::run_delta(DIFF_IN_DIFF, options);
        let output = strip_ansi_codes(&output);
        assert!(output.contains("\n ---\n"));
        assert!(output.contains("\n Subject: [PATCH] Init\n"));
    }

    #[test]
    fn test_commit_style_plain() {
        let mut options = integration_test_utils::get_command_line_options();
        options.commit_style = "plain".to_string();
        // TODO: --commit-color has no effect in conjunction with --commit-style plain
        let (output, _) = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, options);
        ansi_test_utils::assert_line_has_no_color(
            &output,
            0,
            "commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e",
        );
        let output = strip_ansi_codes(&output);
        assert!(output.contains(
            "\
commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e
"
        ));
    }

    #[test]
    fn test_commit_style_box() {
        let mut options = integration_test_utils::get_command_line_options();
        options.commit_style = "box".to_string();
        options.deprecated_commit_color = Some("blue".to_string());
        let (output, config) = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, options);
        ansi_test_utils::assert_line_has_foreground_color(
            &output,
            0,
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓",
            "blue",
            &config,
        );
        ansi_test_utils::assert_line_has_foreground_color(
            &output,
            1,
            "commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e ┃",
            "blue",
            &config,
        );
        ansi_test_utils::assert_line_has_foreground_color(
            &output,
            2,
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━",
            "blue",
            &config,
        );
        let output = strip_ansi_codes(&output);
        assert!(output.contains(
            "\
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e ┃
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━"
        ));
    }

    #[test]
    fn test_commit_style_underline() {
        let mut options = integration_test_utils::get_command_line_options();
        options.commit_style = "underline".to_string();
        options.deprecated_commit_color = Some("yellow".to_string());
        let (output, config) = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, options);
        ansi_test_utils::assert_line_has_foreground_color(
            &output,
            0,
            "commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e",
            "yellow",
            &config,
        );
        ansi_test_utils::assert_line_has_foreground_color(
            &output,
            1,
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
            "yellow",
            &config,
        );
        let output = strip_ansi_codes(&output);
        assert!(output.contains(
            "\
commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        ));
    }

    #[test]
    fn test_file_style_plain() {
        let mut options = integration_test_utils::get_command_line_options();
        options.file_style = "plain".to_string();
        // TODO: --file-color has no effect in conjunction with --file-style plain
        let (output, _) = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, options);
        for (i, line) in vec![
            "diff --git a/src/align.rs b/src/align.rs",
            "index 8e37a9e..6ce4863 100644",
            "--- a/src/align.rs",
            "+++ b/src/align.rs",
        ]
        .iter()
        .enumerate()
        {
            ansi_test_utils::assert_line_has_no_color(&output, 6 + i, line);
        }
        let output = strip_ansi_codes(&output);
        assert!(output.contains(
            "
diff --git a/src/align.rs b/src/align.rs
index 8e37a9e..6ce4863 100644
--- a/src/align.rs
+++ b/src/align.rs
"
        ));
    }

    #[test]
    fn test_file_style_box() {
        let mut options = integration_test_utils::get_command_line_options();
        options.file_style = "box".to_string();
        options.deprecated_file_color = Some("green".to_string());
        let (output, config) = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, options);
        ansi_test_utils::assert_line_has_foreground_color(
            &output,
            7,
            "─────────────┐",
            "green",
            &config,
        );
        ansi_test_utils::assert_line_has_foreground_color(
            &output,
            8,
            "src/align.rs │",
            "green",
            &config,
        );
        ansi_test_utils::assert_line_has_foreground_color(
            &output,
            9,
            "─────────────┴─",
            "green",
            &config,
        );
        let output = strip_ansi_codes(&output);
        assert!(output.contains(
            "
─────────────┐
src/align.rs │
─────────────┴─"
        ));
    }

    #[test]
    fn test_file_style_underline() {
        let mut options = integration_test_utils::get_command_line_options();
        options.file_style = "underline".to_string();
        options.deprecated_file_color = Some("magenta".to_string());
        let (output, config) = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, options);
        ansi_test_utils::assert_line_has_foreground_color(
            &output,
            7,
            "src/align.rs",
            "magenta",
            &config,
        );
        ansi_test_utils::assert_line_has_foreground_color(
            &output,
            8,
            "────────────",
            "magenta",
            &config,
        );
        let output = strip_ansi_codes(&output);
        assert!(output.contains(
            "
src/align.rs
────────────"
        ));
    }

    #[test]
    fn test_hunk_style_plain() {
        let mut options = integration_test_utils::get_command_line_options();
        options.deprecated_hunk_style = Some("plain".to_string());
        // TODO: --hunk-color has no effect in conjunction with --hunk-style plain
        let (output, _) = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, options);
        ansi_test_utils::assert_line_has_no_color(
            &output,
            9,
            "@@ -71,11 +71,8 @@ impl<'a> Alignment<'a> {",
        );
        let output = strip_ansi_codes(&output);
        assert!(output.contains("@@ -71,11 +71,8 @@ impl<'a> Alignment<'a> {"));
    }

    #[test]
    fn test_hunk_style_box() {
        let mut options = integration_test_utils::get_command_line_options();
        options.deprecated_hunk_style = Some("box".to_string());
        options.deprecated_hunk_color = Some("white".to_string());
        let (output, config) = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, options);
        ansi_test_utils::assert_line_has_foreground_color(
            &output,
            9,
            "──────────────────────────┐",
            "white",
            &config,
        );
        ansi_test_utils::assert_line_has_foreground_color(
            &output,
            11,
            "──────────────────────────┘",
            "white",
            &config,
        );
        let output = strip_ansi_codes(&output);
        assert!(output.contains(
            "
──────────────────────────┐
 impl<'a> Alignment<'a> { │
──────────────────────────┘
"
        ));
    }

    #[test]
    fn test_hunk_style_underline() {
        let mut options = integration_test_utils::get_command_line_options();
        options.deprecated_hunk_style = Some("underline".to_string());
        options.deprecated_hunk_color = Some("black".to_string());
        let (output, config) = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, options);
        ansi_test_utils::assert_line_has_foreground_color(
            &output,
            10,
            "─────────────────────────",
            "black",
            &config,
        );
        let output = strip_ansi_codes(&output);
        assert!(output.contains(
            "
 impl<'a> Alignment<'a> {
─────────────────────────"
        ));
    }

    const GIT_DIFF_SINGLE_HUNK: &str = "\
commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e
Author: Dan Davison <dandavison7@gmail.com>
Date:   Thu May 14 11:13:17 2020 -0400

    rustfmt

diff --git a/src/align.rs b/src/align.rs
index 8e37a9e..6ce4863 100644
--- a/src/align.rs
+++ b/src/align.rs
@@ -71,11 +71,8 @@ impl<'a> Alignment<'a> {

         for (i, x_i) in self.x.iter().enumerate() {
             for (j, y_j) in self.y.iter().enumerate() {
-                let (left, diag, up) = (
-                    self.index(i, j + 1),
-                    self.index(i, j),
-                    self.index(i + 1, j),
-                );
+                let (left, diag, up) =
+                    (self.index(i, j + 1), self.index(i, j), self.index(i + 1, j));
                 let candidates = [
                     Cell {
                         parent: left,
";

    const DIFF_IN_DIFF: &str = "\
diff --git a/0001-Init.patch b/0001-Init.patch
deleted file mode 100644
index 5e35a67..0000000
--- a/0001-Init.patch
+++ /dev/null
@@ -1,22 +0,0 @@
-From d3a8fe3e62be67484729c19e9d8db071f8b1d60c Mon Sep 17 00:00:00 2001
-From: Maximilian Bosch <maximilian@mbosch.me>
-Date: Sat, 28 Dec 2019 15:51:48 +0100
-Subject: [PATCH] Init
-
----
- README.md | 3 +++
- 1 file changed, 3 insertions(+)
- create mode 100644 README.md
-
-diff --git a/README.md b/README.md
-new file mode 100644
-index 0000000..2e6ca05
---- /dev/null
-+++ b/README.md
-@@ -0,0 +1,3 @@
-+# Test
-+
-+abc
---
-2.23.1
-
diff --git a/README.md b/README.md
index 2e6ca05..8ae0569 100644
--- a/README.md
+++ b/README.md
@@ -1,3 +1 @@
 # Test
-
-abc
";

    const ADDED_FILE_INPUT: &str = "\
commit d28dc1ac57e53432567ec5bf19ad49ff90f0f7a5
Author: Dan Davison <dandavison7@gmail.com>
Date:   Thu Jul 11 10:41:11 2019 -0400

    .

diff --git a/a.py b/a.py
new file mode 100644
index 0000000..8c55b7d
--- /dev/null
+++ b/a.py
@@ -0,0 +1,3 @@
+# hello
+class X:
+    pass";

    const ADDED_FILE_EXPECTED_OUTPUT: &str = "\
commit d28dc1ac57e53432567ec5bf19ad49ff90f0f7a5
Author: Dan Davison <dandavison7@gmail.com>
Date:   Thu Jul 11 10:41:11 2019 -0400

    .

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
added: a.py
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
────────────────────────────────────────────────────────────────────────────────

────────────────────────────────────────────────────────────────────────────────
 # hello
 class X:
     pass
";

    const ADDED_EMPTY_FILE: &str = "
commit c0a18433cb6e0ca8f796bfae9e31d95b06b91597 (HEAD -> master)
Author: Dan Davison <dandavison7@gmail.com>
Date:   Sun Apr 26 16:32:58 2020 -0400

    Initial commit

diff --git a/file b/file
new file mode 100644
index 0000000..e69de29
";

    const ADDED_FILES_DIRECTORY_PATH_CONTAINING_SPACE: &str = "
commit 654e180c8d5329904d584c44b661149f68bd2911 (HEAD -> master)
Author: Dan Davison <dandavison7@gmail.com>
Date:   Sun Apr 26 16:30:58 2020 -0400

    Initial commit

diff --git a/nospace/file2 b/nospace/file2
new file mode 100644
index 0000000..af1b8ae
--- /dev/null
+++ b/nospace/file2
@@ -0,0 +1 @@
+file2 contents
diff --git a/with space/file1 b/with space/file1
new file mode 100644
index 0000000..84d55c5
--- /dev/null
+++ b/with space/file1
@@ -0,0 +1 @@
+file1 contents
";

    const RENAMED_FILE_INPUT: &str = "\
commit 1281650789680f1009dfff2497d5ccfbe7b96526
Author: Dan Davison <dandavison7@gmail.com>
Date:   Wed Jul 17 20:40:23 2019 -0400

    rename

diff --git a/a.py b/b.py
similarity index 100%
rename from a.py
rename to b.py
";

    const DIFF_UNIFIED_TWO_FILES: &str = "\
--- one.rs	2019-11-20 06:16:08.000000000 +0100
+++ src/two.rs	2019-11-18 18:41:16.000000000 +0100
@@ -5,3 +5,3 @@
 println!(\"Hello world\");
-println!(\"Hello rust\");
+println!(\"Hello ruster\");

@@ -43,6 +43,6 @@
 // Some more changes
-Change one
 Unchanged
+Change two
 Unchanged
-Change three
+Change four
 Unchanged
";

    const DIFF_UNIFIED_TWO_DIRECTORIES: &str = "\
diff -u a/different b/different
--- a/different	2019-11-20 06:47:56.000000000 +0100
+++ b/different	2019-11-20 06:47:56.000000000 +0100
@@ -1,3 +1,3 @@
 A simple file for testing
 the diff command in unified mode
-This is different from b
+This is different from a
Only in a/: just_a
Only in b/: just_b
--- a/more_difference	2019-11-20 06:47:56.000000000 +0100
+++ b/more_difference	2019-11-20 06:47:56.000000000 +0100
@@ -1,3 +1,3 @@
 Another different file
 with a name that start with 'm' making it come after the 'Only in'
-This is different from b
+This is different from a
";

    const NOT_A_DIFF_OUTPUT: &str = "\
Hello world
This is a regular file that contains:
--- some/file/here 06:47:56.000000000 +0100
+++ some/file/there 06:47:56.000000000 +0100
 Some text here
-Some text with a minus
+Some text with a plus
";

    const SUBMODULE_CONTAINS_UNTRACKED_CONTENT_INPUT: &str = "\
--- a
+++ b
@@ -2,3 +2,4 @@
 x
 y
 z
-a
+b
 z
 y
 x
Submodule x/y/z contains untracked content
";

    const TRIPLE_DASH_AT_BEGINNING_OF_LINE_IN_CODE: &str = "\
commit d481eaa8a249c6daecb05a97e8af1b926b0c02be
Author: FirstName LastName <me@gmail.com>
Date:   Thu Feb 6 14:02:49 2020 -0500

    Reorganize

diff --git a/src/Category/Coproduct.hs b/src/Category/Coproduct.hs
deleted file mode 100644
index ba28bfd..0000000
--- a/src/Category/Coproduct.hs
+++ /dev/null
@@ -1,18 +0,0 @@
-{-# LANGUAGE InstanceSigs #-}
-module Category.Coproduct where
-
-import Prelude hiding ((.), id)
-
-import Control.Category
-
-import Category.Hacks
-
--- data (p ∨ q) (a :: (k, k)) (b :: (k, k)) where
---   (:<:) :: p a b -> (∨) p q '(a, c) '(b, d)
---   (:>:) :: q c d -> (∨) p q '(a, c) '(b, d)
---
--- instance (Category p, Category q) => Category (p ∧ q) where
---   (p1 :×: q1) . (p2 :×: q2) = (p1 . p2) :×: (q1 . q2)
---
---   id :: forall a. (p ∧ q) a a
---   id | IsTup <- isTup @a  = id :×: id
";

    const BINARY_FILES_DIFFER: &str = "
commit ad023698217b086f1bef934be62b4523c95f64d9 (HEAD -> master)
Author: Dan Davison <dandavison7@gmail.com>
Date:   Wed Feb 12 08:05:53 2020 -0600

    .

diff --git a/foo b/foo
new file mode 100644
index 0000000..b572921
Binary files /dev/null and b/foo differ
";

    // git --no-pager show -p --cc --format=  --numstat --stat
    // #121
    const DIFF_WITH_UNRECOGNIZED_PRECEDING_MATERIAL_1: &str = "
1	5	src/delta.rs
 src/delta.rs | 6 +-----
 1 file changed, 1 insertion(+), 5 deletions(-)

diff --git a/src/delta.rs b/src/delta.rs
index da10d2b..39cff42 100644
--- a/src/delta.rs
+++ b/src/delta.rs
@@ -67,11 +67,6 @@ where
     let source = detect_source(&mut lines_peekable);

     for raw_line in lines_peekable {
-        if source == Source::Unknown {
-            writeln!(painter.writer, \"{}\", raw_line)?;
-            continue;
-        }
-
         let line = strip_ansi_codes(&raw_line).to_string();
         if line.starts_with(\"commit \") {
             painter.paint_buffered_lines();
@@ -674,6 +669,7 @@ mod tests {
     }

     #[test]
+    #[ignore] // Ideally, delta would make this test pass.
     fn test_delta_ignores_non_diff_input() {
         let options = get_command_line_options();
         let output = strip_ansi_codes(&run_delta(NOT_A_DIFF_OUTPUT, &options)).to_string();
";

    // git stash show --stat --patch
    // #100
    const DIFF_WITH_UNRECOGNIZED_PRECEDING_MATERIAL_2: &str = "
 src/cli.rs    | 2 ++
 src/config.rs | 4 +++-
 2 files changed, 5 insertions(+), 1 deletion(-)

diff --git a/src/cli.rs b/src/cli.rs
index bd5f1d5..55ba315 100644
--- a/src/cli.rs
+++ b/src/cli.rs
@@ -286,6 +286,8 @@ pub fn process_command_line_arguments<'a>(
         }
     };

+    println!(\"true_color is {}\", true_color);
+
     config::get_config(
         opt,
         &assets.syntax_set,
diff --git a/src/config.rs b/src/config.rs
index cba6064..ba1a4de 100644
--- a/src/config.rs
+++ b/src/config.rs
@@ -181,7 +181,9 @@ fn color_from_rgb_or_ansi_code(s: &str) -> Color {
         process::exit(1);
     };
     if s.starts_with(\"#\") {
-        Color::from_str(s).unwrap_or_else(|_| die())
+        let col = Color::from_str(s).unwrap_or_else(|_| die());
+        println!(\"{} => {} {} {} {}\", s, col.r, col.g, col.b, col.a);
+        col
     } else {
         s.parse::<u8>()
             .ok()
";

    const DIFF_WITH_MERGE_CONFLICT: &str = r#"
diff --cc Makefile
index 759070d,3daf9eb..0000000
--- a/Makefile
+++ b/Makefile
@@@ -4,13 -4,16 +4,37 @@@ build
  lint:
  	cargo clippy

++<<<<<<< Updated upstream
 +test: unit-test end-to-end-test
 +
 +unit-test:
 +	cargo test
 +
 +end-to-end-test: build
 +	bash -c "diff -u <(git log -p) <(git log -p | target/release/delta --color-only | perl -pe 's/\e\[[0-9;]*m//g')"
++||||||| constructed merge base
++test:
++	cargo test
++	bash -c "diff -u <(git log -p) \
++                     <(git log -p | delta --width variable \
++                                          --tabs 0 \
++	                                      --retain-plus-minus-markers \
++                                          --commit-style plain \
++                                          --file-style plain \
++                                          --hunk-style plain \
++                                  | ansifilter)"
++=======
+ test:
+ 	cargo test --release
+ 	bash -c "diff -u <(git log -p) \
+                      <(git log -p | target/release/delta --width variable \
+                                           --tabs 0 \
+ 	                                      --retain-plus-minus-markers \
+                                           --commit-style plain \
+                                           --file-style plain \
+                                           --hunk-style plain \
+                                   | ansifilter)" > /dev/null
++>>>>>>> Stashed changes

  release:
  	@make -f release.Makefile release
"#;
}
