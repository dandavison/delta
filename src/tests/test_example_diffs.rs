#[cfg(test)]
mod tests {
    use console::strip_ansi_codes;

    use crate::ansi;
    use crate::delta::State;
    use crate::style;
    use crate::tests::ansi_test_utils::ansi_test_utils;
    use crate::tests::integration_test_utils::integration_test_utils;
    use crate::tests::test_utils::test_utils;

    #[test]
    fn test_added_file() {
        let config = integration_test_utils::make_config_from_args(&[]);
        let output = integration_test_utils::run_delta(ADDED_FILE_INPUT, &config);
        let output = strip_ansi_codes(&output);
        assert!(output.contains("\nadded: a.py\n"));
    }

    #[test]
    #[ignore] // #128
    fn test_added_empty_file() {
        let config = integration_test_utils::make_config_from_args(&[]);
        let output = integration_test_utils::run_delta(ADDED_EMPTY_FILE, &config);
        let output = strip_ansi_codes(&output);
        assert!(output.contains("\nadded: file\n"));
    }

    #[test]
    fn test_added_file_directory_path_containing_space() {
        let config = integration_test_utils::make_config_from_args(&[]);
        let output =
            integration_test_utils::run_delta(ADDED_FILES_DIRECTORY_PATH_CONTAINING_SPACE, &config);
        let output = strip_ansi_codes(&output);
        assert!(output.contains("\nadded: with space/file1\n"));
        assert!(output.contains("\nadded: nospace/file2\n"));
    }

    #[test]
    fn test_renamed_file() {
        let config = integration_test_utils::make_config_from_args(&[]);
        let output = integration_test_utils::run_delta(RENAMED_FILE_INPUT, &config);
        let output = strip_ansi_codes(&output);
        println!("{}", output);
        assert!(test_utils::contains_once(
            &output,
            "\nrenamed: a.py ⟶   b.py\n"
        ));
    }

    #[test]
    fn test_renamed_file_with_changes() {
        let config = integration_test_utils::make_config_from_args(&[]);
        let output = integration_test_utils::run_delta(RENAMED_FILE_WITH_CHANGES_INPUT, &config);
        let output = strip_ansi_codes(&output);
        println!("{}", output);
        assert!(test_utils::contains_once(
            &output,
            "\nrenamed: Casks/font-dejavusansmono-nerd-font.rb ⟶   Casks/font-dejavu-sans-mono-nerd-font.rb\n"));
    }

    #[test]
    fn test_recognized_file_type() {
        // In addition to the background color, the code has language syntax highlighting.
        let config = integration_test_utils::make_config_from_args(&[]);
        let output = integration_test_utils::get_line_of_code_from_delta(
            &ADDED_FILE_INPUT,
            12,
            "class X:",
            &config,
        );
        ansi_test_utils::assert_has_color_other_than_plus_color(&output, &config);
    }

    #[test]
    fn test_unrecognized_file_type_with_syntax_theme() {
        // In addition to the background color, the code has the foreground color using the default
        // .txt syntax under the theme.
        let config = integration_test_utils::make_config_from_args(&[]);
        let input = ADDED_FILE_INPUT.replace("a.py", "a");
        let output =
            integration_test_utils::get_line_of_code_from_delta(&input, 12, "class X:", &config);
        ansi_test_utils::assert_has_color_other_than_plus_color(&output, &config);
    }

    #[test]
    fn test_unrecognized_file_type_no_syntax_theme() {
        // The code has the background color only. (Since there is no theme, the code has no
        // foreground ansi color codes.)
        let config = integration_test_utils::make_config_from_args(&[
            "--syntax-theme",
            "none",
            "--width",
            "variable",
        ]);
        let input = ADDED_FILE_INPUT.replace("a.py", "a");
        let output =
            integration_test_utils::get_line_of_code_from_delta(&input, 12, "class X:", &config);
        ansi_test_utils::assert_has_plus_color_only(&output, &config);
    }

    #[test]
    fn test_diff_unified_two_files() {
        let config = integration_test_utils::make_config_from_args(&[]);
        let output = integration_test_utils::run_delta(DIFF_UNIFIED_TWO_FILES, &config);
        let output = strip_ansi_codes(&output);
        let mut lines = output.split('\n');

        // Header
        assert_eq!(lines.nth(1).unwrap(), "comparing: one.rs ⟶   src/two.rs");
        // Line
        assert_eq!(lines.nth(2).unwrap(), "5");
        // Change
        assert_eq!(lines.nth(2).unwrap(), "println!(\"Hello ruster\");");
        // Next chunk
        assert_eq!(lines.nth(2).unwrap(), "43");
        // Unchanged in second chunk
        assert_eq!(lines.nth(2).unwrap(), "Unchanged");
    }

    #[test]
    fn test_diff_unified_two_directories() {
        let config = integration_test_utils::make_config_from_args(&["--width", "80"]);
        let output = integration_test_utils::run_delta(DIFF_UNIFIED_TWO_DIRECTORIES, &config);
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
        assert_eq!(lines.nth(2).unwrap(), "This is different from b");
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
        let config = integration_test_utils::make_config_from_args(&[]);
        let output = integration_test_utils::run_delta(NOT_A_DIFF_OUTPUT, &config);
        let output = strip_ansi_codes(&output);
        assert_eq!(output, NOT_A_DIFF_OUTPUT.to_owned() + "\n");
    }

    #[test]
    fn test_certain_bugs_are_not_present() {
        for input in vec![
            DIFF_EXHIBITING_PARSE_FILE_NAME_BUG,
            DIFF_EXHIBITING_STATE_MACHINE_PARSER_BUG,
            DIFF_EXHIBITING_TRUNCATION_BUG,
        ] {
            let config = integration_test_utils::make_config_from_args(&["--raw"]);
            let output = integration_test_utils::run_delta(input, &config);
            assert_eq!(strip_ansi_codes(&output), input);
            assert_ne!(output, input);
        }
    }

    #[test]
    fn test_delta_paints_diff_when_there_is_unrecognized_initial_content() {
        for input in vec![
            DIFF_WITH_UNRECOGNIZED_PRECEDING_MATERIAL_1,
            DIFF_WITH_UNRECOGNIZED_PRECEDING_MATERIAL_2,
        ] {
            let config = integration_test_utils::make_config_from_args(&["--raw"]);
            let output = integration_test_utils::run_delta(input, &config);
            assert_eq!(strip_ansi_codes(&output), input);
            assert_ne!(output, input);
        }
    }

    #[test]
    fn test_diff_with_merge_conflict_is_not_truncated() {
        let config = integration_test_utils::make_config_from_args(&[]);
        let output = integration_test_utils::run_delta(DIFF_WITH_MERGE_CONFLICT, &config);
        // TODO: The + in the first column is being removed.
        assert!(strip_ansi_codes(&output).contains("+>>>>>>> Stashed changes"));
        assert_eq!(output.split('\n').count(), 47);
    }

    #[test]
    fn test_diff_with_merge_conflict_is_passed_on_unchanged_under_raw() {
        let config = integration_test_utils::make_config_from_args(&["--raw"]);
        let output = integration_test_utils::run_delta(DIFF_WITH_MERGE_CONFLICT, &config);
        assert_eq!(strip_ansi_codes(&output), DIFF_WITH_MERGE_CONFLICT);
    }

    #[test]
    fn test_submodule_contains_untracked_content() {
        let config = integration_test_utils::make_config_from_args(&[]);
        let output =
            integration_test_utils::run_delta(SUBMODULE_CONTAINS_UNTRACKED_CONTENT_INPUT, &config);
        let output = strip_ansi_codes(&output);
        assert!(output.contains("\nSubmodule x/y/z contains untracked content\n"));
    }

    #[test]
    fn test_triple_dash_at_beginning_of_line_in_code() {
        let config = integration_test_utils::make_config_from_args(&[]);
        let output =
            integration_test_utils::run_delta(TRIPLE_DASH_AT_BEGINNING_OF_LINE_IN_CODE, &config);
        let output = strip_ansi_codes(&output);
        assert!(output.contains("-- instance (Category p, Category q) => Category (p ∧ q) where\n"));
    }

    #[test]
    fn test_binary_files_differ() {
        let config = integration_test_utils::make_config_from_args(&[]);
        let output = integration_test_utils::run_delta(BINARY_FILES_DIFFER, &config);
        let output = strip_ansi_codes(&output);
        assert!(output.contains("Binary files /dev/null and b/foo differ\n"));
    }

    #[test]
    fn test_diff_in_diff() {
        let config = integration_test_utils::make_config_from_args(&[]);
        let output = integration_test_utils::run_delta(DIFF_IN_DIFF, &config);
        let output = strip_ansi_codes(&output);
        assert!(output.contains("\n---\n"));
        assert!(output.contains("\nSubject: [PATCH] Init\n"));
    }

    #[test]
    fn test_commit_style_raw_no_decoration() {
        let config = integration_test_utils::make_config_from_args(&[
            "--commit-style",
            "raw",
            "--commit-decoration-style",
            "omit",
        ]);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        ansi_test_utils::assert_line_has_no_color(
            &output,
            0,
            "commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e",
        );
        assert!(output.contains(
            "\
commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e
"
        ));
    }

    #[test]
    fn test_commit_style_colored_input_color_is_stripped_under_normal() {
        let config = integration_test_utils::make_config_from_args(&[
            "--commit-style",
            "normal",
            "--commit-decoration-style",
            "omit",
        ]);
        let output = integration_test_utils::run_delta(
            GIT_DIFF_SINGLE_HUNK_WITH_ANSI_ESCAPE_SEQUENCES,
            &config,
        );
        ansi_test_utils::assert_line_has_no_color(
            &output,
            0,
            "commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e",
        );
    }

    #[test]
    fn test_commit_style_colored_input_color_is_preserved_under_raw() {
        let config = integration_test_utils::make_config_from_args(&[
            "--commit-style",
            "raw",
            "--commit-decoration-style",
            "omit",
        ]);
        let output = integration_test_utils::run_delta(
            GIT_DIFF_SINGLE_HUNK_WITH_ANSI_ESCAPE_SEQUENCES,
            &config,
        );
        ansi_test_utils::assert_line_has_4_bit_color_style(
            &output,
            0,
            "commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e",
            "bold 31",
            &config,
        );
    }

    #[test]
    fn test_commit_decoration_style_omit() {
        _do_test_commit_style_no_decoration(&[
            "--commit-style",
            "blue",
            "--commit-decoration-style",
            "omit",
        ]);
    }

    #[test]
    fn test_commit_decoration_style_empty_string() {
        _do_test_commit_style_no_decoration(&[
            "--commit-style",
            "blue",
            "--commit-decoration-style",
            "",
        ]);
    }

    fn _do_test_commit_style_no_decoration(args: &[&str]) {
        let config = integration_test_utils::make_config_from_args(args);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        if false {
            // `--commit-style xxx` is not honored yet: always behaves like xxx=raw
            ansi_test_utils::assert_line_has_style(
                &output,
                0,
                "commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e",
                "blue",
                &config,
            );
        }
        let output = strip_ansi_codes(&output);
        assert!(output.contains("commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e"));
        assert!(!output.contains("commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e │"));
        assert!(!output.contains(
            "\
commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e
───────────────────────────────────────────────"
        ));
    }

    #[test]
    fn test_commit_style_omit() {
        let config = integration_test_utils::make_config_from_args(&["--commit-style", "omit"]);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        let output = strip_ansi_codes(&output);
        assert!(!output.contains(
            "\
commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e
"
        ));
    }

    #[test]
    fn test_commit_style_box() {
        _do_test_commit_style_box(&[
            "--commit-style",
            "blue",
            "--commit-decoration-style",
            "blue box",
        ]);
    }

    #[test]
    fn test_commit_style_box_ul() {
        _do_test_commit_style_box_ul(&[
            "--commit-style",
            "blue",
            "--commit-decoration-style",
            "blue box ul",
        ]);
    }

    #[ignore]
    #[test]
    fn test_commit_style_box_ol() {
        _do_test_commit_style_box_ol(&[
            "--commit-style",
            "blue",
            "--commit-decoration-style",
            "blue box ol",
        ]);
    }

    #[test]
    fn test_commit_style_box_ul_deprecated_options() {
        _do_test_commit_style_box_ul(&["--commit-color", "blue", "--commit-style", "box"]);
    }

    fn _do_test_commit_style_box(args: &[&str]) {
        let config = integration_test_utils::make_config_from_args(args);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        ansi_test_utils::assert_line_has_style(
            &output,
            0,
            "────────────────────────────────────────────────┐",
            "blue",
            &config,
        );
        ansi_test_utils::assert_line_has_style(
            &output,
            1,
            "commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e │",
            "blue",
            &config,
        );
        ansi_test_utils::assert_line_has_style(
            &output,
            2,
            "────────────────────────────────────────────────┘",
            "blue",
            &config,
        );
        let output = strip_ansi_codes(&output);
        assert!(output.contains(
            "\
────────────────────────────────────────────────┐
commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e │
────────────────────────────────────────────────┘
"
        ));
    }

    fn _do_test_commit_style_box_ul(args: &[&str]) {
        let config = integration_test_utils::make_config_from_args(args);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        ansi_test_utils::assert_line_has_style(
            &output,
            0,
            "────────────────────────────────────────────────┐",
            "blue",
            &config,
        );
        ansi_test_utils::assert_line_has_style(
            &output,
            1,
            "commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e │",
            "blue",
            &config,
        );
        ansi_test_utils::assert_line_has_style(
            &output,
            2,
            "────────────────────────────────────────────────┴─",
            "blue",
            &config,
        );
        let output = strip_ansi_codes(&output);
        assert!(output.contains(
            "\
────────────────────────────────────────────────┐
commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e │
────────────────────────────────────────────────┴─"
        ));
    }

    fn _do_test_commit_style_box_ol(args: &[&str]) {
        let config = integration_test_utils::make_config_from_args(args);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        ansi_test_utils::assert_line_has_style(
            &output,
            0,
            "────────────────────────────────────────────────┬─",
            "blue",
            &config,
        );
        ansi_test_utils::assert_line_has_style(
            &output,
            1,
            "commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e │",
            "blue",
            &config,
        );
        ansi_test_utils::assert_line_has_style(
            &output,
            2,
            "────────────────────────────────────────────────┘",
            "blue",
            &config,
        );
        let output = strip_ansi_codes(&output);
        assert!(output.contains(
            "\
────────────────────────────────────────────────┬─
commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e │
────────────────────────────────────────────────┘
"
        ));
    }

    #[test]
    fn test_commit_style_box_raw() {
        let config = integration_test_utils::make_config_from_args(&[
            "--commit-style",
            "raw",
            "--commit-decoration-style",
            "box ul",
        ]);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        ansi_test_utils::assert_line_has_no_color(
            &output,
            1,
            "commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e │",
        );
        assert!(output.contains(
            "\
────────────────────────────────────────────────┐
commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e │
────────────────────────────────────────────────┴─"
        ));
    }

    // TODO: test overline

    #[test]
    fn test_commit_style_underline() {
        _do_test_commit_style_underline(&[
            "--commit-style",
            "yellow",
            "--commit-decoration-style",
            "yellow underline",
        ]);
    }

    #[test]
    fn test_commit_style_underline_deprecated_options() {
        _do_test_commit_style_underline(&[
            "--commit-color",
            "yellow",
            "--commit-style",
            "underline",
        ]);
    }

    fn _do_test_commit_style_underline(args: &[&str]) {
        let config = integration_test_utils::make_config_from_args(args);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        ansi_test_utils::assert_line_has_style(
            &output,
            0,
            "commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e",
            "yellow",
            &config,
        );
        ansi_test_utils::assert_line_has_style(
            &output,
            1,
            "───────────────────────────────────────────────",
            "yellow",
            &config,
        );
        let output = strip_ansi_codes(&output);
        assert!(output.contains(
            "\
commit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e
───────────────────────────────────────────────"
        ));
    }

    #[test]
    fn test_file_style_raw_no_decoration() {
        let config = integration_test_utils::make_config_from_args(&[
            "--file-style",
            "raw",
            "--file-decoration-style",
            "omit",
        ]);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
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
    fn test_file_style_colored_input_color_is_stripped_under_normal() {
        let config = integration_test_utils::make_config_from_args(&[
            "--file-style",
            "normal",
            "--file-decoration-style",
            "omit",
        ]);
        let output = integration_test_utils::run_delta(
            GIT_DIFF_SINGLE_HUNK_WITH_ANSI_ESCAPE_SEQUENCES,
            &config,
        );
        ansi_test_utils::assert_line_has_no_color(&output, 7, "src/align.rs");
    }

    #[test]
    fn test_file_style_colored_input_color_is_preserved_under_raw() {
        let config = integration_test_utils::make_config_from_args(&[
            "--file-style",
            "raw",
            "--file-decoration-style",
            "omit",
        ]);
        let output = integration_test_utils::run_delta(
            GIT_DIFF_SINGLE_HUNK_WITH_ANSI_ESCAPE_SEQUENCES,
            &config,
        );
        for (i, line) in vec![
            "diff --git a/src/align.rs b/src/align.rs",
            "index 8e37a9e..6ce4863 100644",
            "--- a/src/align.rs",
            "+++ b/src/align.rs",
        ]
        .iter()
        .enumerate()
        {
            ansi_test_utils::assert_line_has_4_bit_color_style(&output, 6 + i, line, "31", &config)
        }
    }

    #[test]
    fn test_file_decoration_style_omit() {
        _do_test_file_style_no_decoration(&[
            "--file-style",
            "green",
            "--file-decoration-style",
            "omit",
        ]);
    }

    #[test]
    fn test_file_decoration_style_empty_string() {
        _do_test_file_style_no_decoration(&[
            "--file-style",
            "green",
            "--file-decoration-style",
            "",
        ]);
    }

    fn _do_test_file_style_no_decoration(args: &[&str]) {
        let config = integration_test_utils::make_config_from_args(args);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        ansi_test_utils::assert_line_has_style(&output, 7, "src/align.rs", "green", &config);
        let output = strip_ansi_codes(&output);
        assert!(output.contains("src/align.rs"));
        assert!(!output.contains("src/align.rs │"));
        assert!(!output.contains(
            "
src/align.rs
────────────"
        ));
    }

    #[test]
    fn test_file_style_omit() {
        let config = integration_test_utils::make_config_from_args(&["--file-style", "omit"]);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        assert!(!output.contains("src/align.rs"));
    }

    #[test]
    fn test_file_style_box() {
        _do_test_file_style_box(&[
            "--file-style",
            "green",
            "--file-decoration-style",
            "green box",
        ]);
    }

    #[test]
    fn test_file_style_box_ul() {
        _do_test_file_style_box_ul(&[
            "--file-style",
            "green",
            "--file-decoration-style",
            "green box ul",
        ]);
    }

    #[ignore]
    #[test]
    fn test_file_style_box_ol() {
        _do_test_file_style_box_ol(&[
            "--file-style",
            "green",
            "--file-decoration-style",
            "green box ol",
        ]);
    }

    #[test]
    fn test_file_style_box_ul_deprecated_options() {
        _do_test_file_style_box_ul(&["--file-color", "green", "--file-style", "box"]);
    }

    fn _do_test_file_style_box(args: &[&str]) {
        let config = integration_test_utils::make_config_from_args(args);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        ansi_test_utils::assert_line_has_style(&output, 7, "─────────────┐", "green", &config);
        ansi_test_utils::assert_line_has_style(&output, 8, "src/align.rs │", "green", &config);
        ansi_test_utils::assert_line_has_style(&output, 9, "─────────────┘", "green", &config);
        let output = strip_ansi_codes(&output);
        assert!(output.contains(
            "
─────────────┐
src/align.rs │
─────────────┘
"
        ));
    }

    fn _do_test_file_style_box_ul(args: &[&str]) {
        let config = integration_test_utils::make_config_from_args(args);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        ansi_test_utils::assert_line_has_style(&output, 7, "─────────────┐", "green", &config);
        ansi_test_utils::assert_line_has_style(&output, 8, "src/align.rs │", "green", &config);
        ansi_test_utils::assert_line_has_style(&output, 9, "─────────────┴─", "green", &config);
        let output = strip_ansi_codes(&output);
        assert!(output.contains(
            "
─────────────┐
src/align.rs │
─────────────┴─"
        ));
    }

    fn _do_test_file_style_box_ol(args: &[&str]) {
        let config = integration_test_utils::make_config_from_args(args);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        ansi_test_utils::assert_line_has_style(&output, 7, "─────────────┬─", "green", &config);
        ansi_test_utils::assert_line_has_style(&output, 8, "src/align.rs │", "green", &config);
        ansi_test_utils::assert_line_has_style(&output, 9, "─────────────┘", "green", &config);
        let output = strip_ansi_codes(&output);
        assert!(output.contains(
            "
─────────────┬─
src/align.rs │
─────────────┘
"
        ));
    }

    #[test]
    fn test_file_style_box_raw() {
        let config = integration_test_utils::make_config_from_args(&[
            "--file-style",
            "raw",
            "--file-decoration-style",
            "box ul",
        ]);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        ansi_test_utils::assert_line_has_no_color(&output, 8, "src/align.rs │");
        assert!(output.contains(
            "
─────────────┐
src/align.rs │
─────────────┴─"
        ));
    }

    #[test]
    fn test_file_style_underline() {
        _do_test_file_style_underline(&[
            "--file-style",
            "magenta",
            "--file-decoration-style",
            "magenta underline",
        ]);
    }

    #[test]
    fn test_file_style_underline_deprecated_options() {
        _do_test_file_style_underline(&["--file-color", "magenta", "--file-style", "underline"]);
    }

    fn _do_test_file_style_underline(args: &[&str]) {
        let config = integration_test_utils::make_config_from_args(args);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        ansi_test_utils::assert_line_has_style(&output, 7, "src/align.rs", "magenta", &config);
        ansi_test_utils::assert_line_has_style(&output, 8, "────────────", "magenta", &config);
        let output = strip_ansi_codes(&output);
        assert!(output.contains(
            "
src/align.rs
────────────"
        ));
    }

    #[test]
    fn test_hunk_header_style_raw_no_decoration() {
        let config = integration_test_utils::make_config_from_args(&[
            "--hunk-header-style",
            "raw",
            "--hunk-header-decoration-style",
            "omit",
        ]);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        ansi_test_utils::assert_line_has_no_color(
            &output,
            9,
            "@@ -71,11 +71,8 @@ impl<'a> Alignment<'a> {",
        );
        assert!(output.contains("@@ -71,11 +71,8 @@ impl<'a> Alignment<'a> {"));
    }

    #[test]
    fn test_hunk_header_style_raw_no_decoration_with_line_numbers() {
        let config = integration_test_utils::make_config_from_args(&[
            "--hunk-header-style",
            "raw",
            "--hunk-header-decoration-style",
            "omit",
            "--line-numbers",
        ]);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        assert!(output.contains("
@@ -71,11 +71,8 @@ impl<'a> Alignment<'a> {"));
        assert!(!output.contains("

@@ -71,11 +71,8 @@ impl<'a> Alignment<'a> {"));
        ansi_test_utils::assert_line_has_no_color(
            &output,
            9,
            "@@ -71,11 +71,8 @@ impl<'a> Alignment<'a> {",
        );
    }

    #[test]
    fn test_hunk_header_style_colored_input_color_is_stripped_under_normal() {
        let config = integration_test_utils::make_config_from_args(&[
            "--hunk-header-style",
            "normal",
            "--hunk-header-decoration-style",
            "omit",
        ]);
        let output = integration_test_utils::run_delta(
            GIT_DIFF_SINGLE_HUNK_WITH_ANSI_ESCAPE_SEQUENCES,
            &config,
        );
        // An additional newline is inserted under anything other than `style=raw,
        // decoration-style=omit`, to better separate the hunks. Hence 9 + 1.
        ansi_test_utils::assert_line_has_no_color(&output, 9 + 1, "impl<'a> Alignment<'a> {");
    }

    #[test]
    fn test_hunk_header_style_colored_input_color_is_preserved_under_raw() {
        let config = integration_test_utils::make_config_from_args(&[
            "--hunk-header-style",
            "raw",
            "--hunk-header-decoration-style",
            "omit",
        ]);
        let output = integration_test_utils::run_delta(
            GIT_DIFF_SINGLE_HUNK_WITH_ANSI_ESCAPE_SEQUENCES,
            &config,
        );
        ansi_test_utils::assert_line_has_4_bit_color_style(
            &output,
            9,
            "@@ -71,11 +71,8 @@ impl<'a> Alignment<'a> {",
            "bold 31",
            &config,
        );
    }

    #[test]
    fn test_hunk_header_decoration_style_omit() {
        _do_test_hunk_header_style_no_decoration(&["--hunk-header-decoration-style", "omit"]);
    }

    #[test]
    fn test_hunk_header_decoration_style_none() {
        _do_test_hunk_header_style_no_decoration(&["--hunk-header-decoration-style", "none"]);
    }

    #[test]
    fn test_hunk_header_decoration_style_empty_string() {
        _do_test_hunk_header_style_no_decoration(&["--hunk-header-decoration-style", ""]);
    }

    fn _do_test_hunk_header_style_no_decoration(args: &[&str]) {
        let config = integration_test_utils::make_config_from_args(args);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        let output = strip_ansi_codes(&output);
        assert!(output.contains("impl<'a> Alignment<'a> {"));
        assert!(!output.contains("impl<'a> Alignment<'a> { │"));
        assert!(!output.contains(
            "
impl<'a> Alignment<'a> {
────────────────────────"
        ));
    }

    #[test]
    fn test_hunk_header_style_omit() {
        let config =
            integration_test_utils::make_config_from_args(&["--hunk-header-style", "omit"]);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        let output = strip_ansi_codes(&output);
        assert!(!output.contains("impl<'a> Alignment<'a> {"));
    }

    #[test]
    fn test_hunk_header_style_empty_string() {
        _do_test_hunk_header_empty_style(&["--hunk-header-style", ""]);
    }

    #[test]
    fn test_hunk_header_style_none() {
        _do_test_hunk_header_empty_style(&["--hunk-header-style", "None"]);
    }

    fn _do_test_hunk_header_empty_style(args: &[&str]) {
        let config = integration_test_utils::make_config_from_args(args);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        assert!(output.contains("impl<'a> Alignment<'a> {"));
        assert!(!output.contains("@@"));
    }

    #[test]
    fn test_hunk_header_style_box() {
        _do_test_hunk_header_style_box(&["--hunk-header-decoration-style", "white box"]);
    }

    #[test]
    fn test_hunk_header_style_box_deprecated_options() {
        _do_test_hunk_header_style_box(&["--hunk-color", "white", "--hunk-style", "box"]);
    }

    fn _do_test_hunk_header_style_box(args: &[&str]) {
        let config = integration_test_utils::make_config_from_args(args);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        ansi_test_utils::assert_line_has_style(
            &output,
            10,
            "─────────────────────────┐",
            "white",
            &config,
        );
        ansi_test_utils::assert_line_has_style(
            &output,
            12,
            "─────────────────────────┘",
            "white",
            &config,
        );
        let output = strip_ansi_codes(&output);
        assert!(output.contains(
            "
─────────────────────────┐
impl<'a> Alignment<'a> { │
─────────────────────────┘
"
        ));
    }

    #[test]
    fn test_hunk_header_style_box_raw() {
        let config = integration_test_utils::make_config_from_args(&[
            "--hunk-header-style",
            "raw",
            "--hunk-header-decoration-style",
            "box",
        ]);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        ansi_test_utils::assert_line_has_no_color(
            &output,
            11,
            "@@ -71,11 +71,8 @@ impl<'a> Alignment<'a> { │",
        );
        assert!(output.contains(
            "
────────────────────────────────────────────┐
@@ -71,11 +71,8 @@ impl<'a> Alignment<'a> { │
────────────────────────────────────────────┘
"
        ));
    }

    #[test]
    fn test_hunk_header_style_underline() {
        _do_test_hunk_header_style_underline(&[
            "--hunk-header-decoration-style",
            "black underline",
        ]);
    }

    #[test]
    fn test_hunk_header_style_underline_deprecated_options() {
        _do_test_hunk_header_style_underline(&[
            "--hunk-color",
            "black",
            "--hunk-style",
            "underline",
        ]);
    }

    fn _do_test_hunk_header_style_underline(args: &[&str]) {
        let config = integration_test_utils::make_config_from_args(args);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        ansi_test_utils::assert_line_has_style(
            &output,
            11,
            "─────────────────────────",
            "black",
            &config,
        );
        let output = strip_ansi_codes(&output);
        assert!(output.contains(
            "
impl<'a> Alignment<'a> { 
────────────────────────"
        ));
    }

    #[test]
    fn test_hunk_header_style_box_with_syntax_highlighting() {
        // For this test we are currently forced to disable styling of the decoration, since
        // otherwise it will confuse assert_line_is_syntax_highlighted.
        let config = integration_test_utils::make_config_from_args(&[
            "--hunk-header-style",
            "syntax",
            "--hunk-header-decoration-style",
            "box",
        ]);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        ansi_test_utils::assert_line_has_no_color(&output, 10, "─────────────────────────┐");
        ansi_test_utils::assert_line_is_syntax_highlighted(
            &output,
            11,
            "impl<'a> Alignment<'a> { ",
            "rs",
            State::HunkHeader,
            &config,
        );
        ansi_test_utils::assert_line_has_no_color(&output, 12, "─────────────────────────┘");
        let output = strip_ansi_codes(&output);
        assert!(output.contains(
            "
─────────────────────────┐
impl<'a> Alignment<'a> { │
─────────────────────────┘
"
        ));
    }

    #[test]
    fn test_removed_empty_line_highlight() {
        let minus_empty_line_marker_style = "bold yellow magenta ul";
        _do_test_removed_empty_line_highlight(minus_empty_line_marker_style, "red reverse", true);
        _do_test_removed_empty_line_highlight(minus_empty_line_marker_style, "normal red", true);
        _do_test_removed_empty_line_highlight(minus_empty_line_marker_style, "red", false);
        _do_test_removed_empty_line_highlight(
            minus_empty_line_marker_style,
            "normal red reverse",
            false,
        );
    }

    fn _do_test_removed_empty_line_highlight(
        empty_line_marker_style: &str,
        base_style: &str,
        base_style_has_background_color: bool,
    ) {
        _do_test_empty_line_highlight(
            "--minus-empty-line-marker-style",
            empty_line_marker_style,
            "--minus-style",
            base_style,
            base_style_has_background_color,
            DIFF_WITH_REMOVED_EMPTY_LINE,
        );
    }

    #[test]
    fn test_added_empty_line_highlight() {
        let plus_empty_line_marker_style = "bold yellow magenta ul";
        _do_test_added_empty_line_highlight(plus_empty_line_marker_style, "green reverse", true);
        _do_test_added_empty_line_highlight(plus_empty_line_marker_style, "normal green", true);
        _do_test_added_empty_line_highlight(plus_empty_line_marker_style, "green", false);
        _do_test_added_empty_line_highlight(
            plus_empty_line_marker_style,
            "normal green reverse",
            false,
        );
    }

    fn _do_test_added_empty_line_highlight(
        empty_line_marker_style: &str,
        base_style: &str,
        base_style_has_background_color: bool,
    ) {
        _do_test_empty_line_highlight(
            "--plus-empty-line-marker-style",
            empty_line_marker_style,
            "--plus-style",
            base_style,
            base_style_has_background_color,
            DIFF_WITH_ADDED_EMPTY_LINE,
        );
    }

    fn _do_test_empty_line_highlight(
        empty_line_marker_style_name: &str,
        empty_line_marker_style: &str,
        base_style_name: &str,
        base_style: &str,
        base_style_has_background_color: bool,
        example_diff: &str,
    ) {
        let config = integration_test_utils::make_config_from_args(&[
            base_style_name,
            base_style,
            empty_line_marker_style_name,
            empty_line_marker_style,
        ]);
        let output = integration_test_utils::run_delta(example_diff, &config);
        let line = output.lines().nth(6).unwrap();
        if base_style_has_background_color {
            let style = style::Style::from_str(base_style, None, None, true, false);
            assert_eq!(
                line,
                &style
                    .ansi_term_style
                    .paint(ansi::ANSI_CSI_CLEAR_TO_EOL)
                    .to_string()
            );
        } else {
            let style = style::Style::from_str(empty_line_marker_style, None, None, true, false);
            assert_eq!(
                line,
                &style
                    .ansi_term_style
                    .paint(ansi::ANSI_CSI_CLEAR_TO_BOL)
                    .to_string()
            );
        }
    }

    #[test]
    fn test_whitespace_error() {
        let whitespace_error_style = "bold yellow magenta ul";
        let config = integration_test_utils::make_config_from_args(&[
            "--whitespace-error-style",
            whitespace_error_style,
        ]);
        let output = integration_test_utils::run_delta(DIFF_WITH_WHITESPACE_ERROR, &config);
        ansi_test_utils::assert_line_has_style(&output, 6, " ", whitespace_error_style, &config);
        let output = integration_test_utils::run_delta(DIFF_WITH_REMOVED_WHITESPACE_ERROR, &config);
        ansi_test_utils::assert_line_does_not_have_style(
            &output,
            6,
            " ",
            whitespace_error_style,
            &config,
        );
    }

    #[test]
    fn test_color_only() {
        let config = integration_test_utils::make_config_from_args(&["--color-only"]);
        let output = integration_test_utils::run_delta(GIT_DIFF_SINGLE_HUNK, &config);
        ansi_test_utils::assert_line_is_syntax_highlighted(
            &output,
            12,
            "        for (i, x_i) in self.x.iter().enumerate() {",
            "rs",
            State::HunkZero,
            &config,
        );
    }

    #[test]
    fn test_git_diff_is_unchanged_under_color_only() {
        let config = integration_test_utils::make_config_from_args(&["--color-only"]);
        let input = DIFF_WITH_TWO_ADDED_LINES;
        let output = integration_test_utils::run_delta(input, &config);
        let output = strip_ansi_codes(&output);
        assert_eq!(output, input);
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_git_diff_U0_is_unchanged_under_color_only() {
        let config = integration_test_utils::make_config_from_args(&["--color-only"]);
        let input = DIFF_WITH_TWO_ADDED_LINES_CREATED_BY_GIT_DIFF_U0;
        let output = integration_test_utils::run_delta(input, &config);
        let output = strip_ansi_codes(&output);
        assert_eq!(output, input);
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

    const GIT_DIFF_SINGLE_HUNK_WITH_ANSI_ESCAPE_SEQUENCES: &str = "\
[1;31mcommit 94907c0f136f46dc46ffae2dc92dca9af7eb7c2e[m
Author: Dan Davison <dandavison7@gmail.com>
Date:   Thu May 14 11:13:17 2020 -0400

    rustfmt

[31mdiff --git a/src/align.rs b/src/align.rs[m
[31mindex 8e37a9e..6ce4863 100644[m
[31m--- a/src/align.rs[m
[31m+++ b/src/align.rs[m
[1;31m@@ -71,11 +71,8 @@[m [mimpl<'a> Alignment<'a> {[m
 [m
         for (i, x_i) in self.x.iter().enumerate() {[m
             for (j, y_j) in self.y.iter().enumerate() {[m
[1;31m-                let (left, diag, up) = ([m
[1;31m-                    self.index(i, j + 1),[m
[1;31m-                    self.index(i, j),[m
[1;31m-                    self.index(i + 1, j),[m
[1;31m-                );[m
[1;32m+[m[1;32m                let (left, diag, up) =[m
[1;32m+[m[1;32m                    (self.index(i, j + 1), self.index(i, j), self.index(i + 1, j));[m
                 let candidates = [[m
                     Cell {[m
                         parent: left,[m
[31mdiff --git a/src/bat/mod.rs b/src/bat/mod.rs[m
[31mindex 362ba77..7812e7c 100644[m
[31m--- a/src/bat/mod.rs[m
[31m+++ b/src/bat/mod.rs[m
[1;31m@@ -1,5 +1,5 @@[m
 pub mod assets;[m
 pub mod dirs;[m
[1;32m+[m[1;32mmod less;[m
 pub mod output;[m
 pub mod terminal;[m
[1;31m-mod less;[m
[31mdiff --git a/src/bat/output.rs b/src/bat/output.rs[m
[31mindex d23f5e8..e4ed702 100644[m
[31m--- a/src/bat/output.rs[m
[31m+++ b/src/bat/output.rs[m
[1;31m@@ -8,8 +8,8 @@[m [muse std::process::{Child, Command, Stdio};[m
 [m
 use shell_words;[m
 [m
[1;31m-use crate::env;[m
 use super::less::retrieve_less_version;[m
[1;32m+[m[1;32muse crate::env;[m
 [m
 #[derive(Debug, Clone, Copy, PartialEq)][m
 #[allow(dead_code)][m
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

    const RENAMED_FILE_WITH_CHANGES_INPUT: &str = "\
commit 5a6dd572797813525199c32e26471e88732cae1f
Author: Waldir Pimenta <waldyrious@gmail.com>
Date:   Sat Jul 11 19:14:43 2020 +0100

    Rename font-dejavusansmono-nerd-font→font-dejavu-sans-mono-nerd-font
    
    This makes the filename more readable, and is consistent with `font-dejavu-sans-mono-for-powerline`.

diff --git a/Casks/font-dejavusansmono-nerd-font.rb b/Casks/font-dejavu-sans-mono-nerd-font.rb
similarity index 95%
rename from Casks/font-dejavusansmono-nerd-font.rb
rename to Casks/font-dejavu-sans-mono-nerd-font.rb
index 2c8b440f..d1c1b0f3 100644
--- a/Casks/font-dejavusansmono-nerd-font.rb
+++ b/Casks/font-dejavu-sans-mono-nerd-font.rb
@@ -1,4 +1,4 @@
-cask 'font-dejavusansmono-nerd-font' do
+cask 'font-dejavu-sans-mono-nerd-font' do
   version '2.1.0'
   sha256 '3fbcc4904c88f68d24c8b479784a1aba37f2d78b1162d21f6fc85a58ffcc0e0f'
 
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

    // A bug appeared with the change to the tokenization regex in
    // b5d87819a1f76de9ef8f16f1bfb413468af50b62. The bug was triggered by this diff.
    const DIFF_EXHIBITING_TRUNCATION_BUG: &str = r#"
diff --git a/a.rs b/b.rs
index cba6064..ba1a4de 100644
--- a/a.rs
+++ b/b.rs
@@ -1,1 +1,1 @@
- Co
+ let col = Co
"#;

    // A bug appeared with the change to the state machine parser in
    // 5adc445ec38142046fc4cc4518e7019fe54f2e35. The bug was triggered by this diff. The bug was
    // present prior to that commit.
    const DIFF_EXHIBITING_STATE_MACHINE_PARSER_BUG: &str = r"
diff --git a/src/delta.rs b/src/delta.rs
index 20aef29..20416c0 100644
--- a/src/delta.rs
+++ b/src/delta.rs
@@ -994,0 +1014,2 @@ index cba6064..ba1a4de 100644
+++ a
+++ b
";

    const DIFF_EXHIBITING_PARSE_FILE_NAME_BUG: &str = r"
diff --git c/a i/a
new file mode 100644
index 0000000..eea55b6
--- /dev/null
+++ i/a
@@ -0,0 +1 @@
+++ a
";

    const DIFF_WITH_REMOVED_EMPTY_LINE: &str = r"
diff --git i/a w/a
index 8b13789..e69de29 100644
--- i/a
+++ w/a
@@ -1 +0,0 @@
-
";

    const DIFF_WITH_ADDED_EMPTY_LINE: &str = r"
diff --git i/a w/a
index e69de29..8b13789 100644
--- i/a
+++ w/a
@@ -0,0 +1 @@
+
";

    const DIFF_WITH_WHITESPACE_ERROR: &str = r"
diff --git c/a i/a
new file mode 100644
index 0000000..8d1c8b6
--- /dev/null
+++ i/a
@@ -0,0 +1 @@
+ 
";

    const DIFF_WITH_REMOVED_WHITESPACE_ERROR: &str = r"
diff --git i/a w/a
index 8d1c8b6..8b13789 100644
--- i/a
+++ w/a
@@ -1 +1 @@
- 
+
";

    const DIFF_WITH_TWO_ADDED_LINES: &str = r#"
diff --git a/example.c b/example.c
index 386f291a..22666f79 100644
--- a/example.c
+++ b/example.c
@@ -1,6 +1,8 @@
 int other_routine() {
+    return 0;
 }
 
 int main() {
     puts("Hello, world!");
+    return 0;
 }
"#;

    const DIFF_WITH_TWO_ADDED_LINES_CREATED_BY_GIT_DIFF_U0: &str = r#"
diff --git a/example.c b/example.c
index 386f291a..22666f79 100644
--- a/example.c
+++ b/example.c
@@ -1,0 +2 @@ int other_routine() {
+    return 0;
@@ -5,0 +7 @@ int main() {
+    return 0;
"#;
}
