#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use crate::bat::assets::HighlightingAssets;
    use crate::cli;
    use crate::config::ColorLayer::*;
    use crate::delta::State;
    use crate::paint;
    use crate::tests::ansi_test_utils::ansi_test_utils;
    use crate::tests::integration_test_utils::integration_test_utils;

    #[test]
    fn test_hunk_highlighting() {
        let mut options = integration_test_utils::get_command_line_options();
        options.max_line_distance = 1.0;
        for lines_to_be_syntax_highlighted in
            vec!["none", "all", "+", "0", "0+", "-", "-+", "-0", "-0+"]
        {
            options.lines_to_be_syntax_highlighted = lines_to_be_syntax_highlighted.to_string();
            _do_hunk_color_test(&options);
        }
    }

    fn _do_hunk_color_test(options: &cli::Opt) {
        let output = integration_test_utils::run_delta(
            DIFF_YIELDING_ALL_HUNK_LINE_COLOR_CATEGORIES,
            options,
        );

        let lines = output.trim().split("\n").skip(4);

        // Hack: The config has been built once already in run_delta
        let assets = HighlightingAssets::new();
        let config = cli::process_command_line_arguments(&assets, &options);

        let minus =
            paint::paint_text_background("", config.minus_style_modifier.background.unwrap(), true)
                .trim_end_matches(paint::ANSI_SGR_RESET)
                .to_string();
        let minus_emph = paint::paint_text_background(
            "",
            config.minus_emph_style_modifier.background.unwrap(),
            true,
        )
        .trim_end_matches(paint::ANSI_SGR_RESET)
        .to_string();
        let plus =
            paint::paint_text_background("", config.plus_style_modifier.background.unwrap(), true)
                .trim_end_matches(paint::ANSI_SGR_RESET)
                .to_string();
        let plus_emph = paint::paint_text_background(
            "",
            config.plus_emph_style_modifier.background.unwrap(),
            true,
        )
        .trim_end_matches(paint::ANSI_SGR_RESET)
        .to_string();

        let expectation = vec![
            // line 1: unchanged
            (
                State::HunkZero,
                vec![("", "(11111111, 11111111, 11111111)")],
            ),
            // line 2: removed, final token is minus-emph
            (
                State::HunkMinus,
                vec![
                    (minus.as_str(), "(22222222, 22222222"),
                    (minus_emph.as_str(), ", 22222222"),
                    (minus.as_str(), ")"),
                    (paint::ANSI_SGR_RESET, ""),
                ],
            ),
            // line 3: removed
            (
                State::HunkMinus,
                vec![
                    (minus.as_str(), "(33333333, 33333333, 33333333)"),
                    (paint::ANSI_SGR_RESET, ""),
                ],
            ),
            // line 4: removed
            (
                State::HunkMinus,
                vec![
                    (minus.as_str(), "(44444444, 44444444, 44444444)"),
                    (paint::ANSI_SGR_RESET, ""),
                ],
            ),
            // line 5: added, and syntax-higlighted.
            (
                State::HunkPlus,
                vec![
                    (plus.as_str(), "(22222222, 22222222)"),
                    (paint::ANSI_SGR_RESET, ""),
                ],
            ),
            // line 6: added, and syntax-highlighted. First is plus-emph.
            (
                State::HunkPlus,
                vec![
                    (plus.as_str(), "("),
                    (plus_emph.as_str(), "33333333, "),
                    (plus.as_str(), "33333333, 33333333, 33333333)"),
                    (paint::ANSI_SGR_RESET, ""),
                ],
            ),
            // line 7: unchanged
            (
                State::HunkZero,
                vec![("", "(55555555, 55555555, 55555555)")],
            ),
            // line 8: added, and syntax-highlighted.
            (
                State::HunkPlus,
                vec![
                    (plus.as_str(), "(66666666, 66666666, 66666666)"),
                    (paint::ANSI_SGR_RESET, ""),
                ],
            ),
        ];

        // TODO: check same length
        for ((state, assertion), line) in expectation.iter().zip_eq(lines) {
            if config.should_syntax_highlight(state) {
                assert!(ansi_test_utils::is_syntax_highlighted(line));
            } else {
                // An explicit assertion about the ANSI sequences should be available (when there's
                // syntax highlighting the pattern of ANSI sequences is too complex to make the
                // assertion).
                ansi_test_utils::assert_line_has_expected_ansi_sequences(line, &assertion)
            }
            // Background color should match the line's state.
            match config.get_color(state, Background) {
                Some(_color) => assert!(ansi_test_utils::line_has_background_color(
                    line, state, &config
                )),
                None => assert!(ansi_test_utils::line_has_no_background_color(line)),
            }
        }
    }

    const DIFF_YIELDING_ALL_HUNK_LINE_COLOR_CATEGORIES: &str = r"
diff --git a/file.py b/file.py
index 15c0fa2..dc2254c 100644
--- a/file.py
+++ b/file.py
@@ -1,6 +1,6 @@
 (11111111, 11111111, 11111111)
-(22222222, 22222222, 22222222)
-(33333333, 33333333, 33333333)
-(44444444, 44444444, 44444444)
+(22222222, 22222222)
+(33333333, 33333333, 33333333, 33333333)
 (55555555, 55555555, 55555555)
+(66666666, 66666666, 66666666)
";
}
