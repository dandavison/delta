// See https://github.com/BurntSushi/ripgrep
// This module implements handling of `rg --json` output. It is called by the
// handler in handlers/grep.rs. Normal rg output (i.e. without --json) is
// handled by the same code paths as `git grep` etc output, in handlers/grep.rs.
use std::borrow::Cow;

use crate::handlers::grep;
use serde::Deserialize;
use serde_json::Value;

pub fn parse_line(line: &str) -> Option<grep::GrepLine<'_>> {
    let ripgrep_line: Option<RipGrepLine> = serde_json::from_str(line).ok();
    match ripgrep_line {
        Some(ripgrep_line) => {
            // A real line of rg --json output, i.e. either of type "match" or
            // "context".
            // Ripgrep orders submatches by their zero-based byte offsets into `lines`.
            // Save the first match's one-based source column before tab expansion
            // shifts the submatch offsets used for highlighting.
            let column_number = if ripgrep_line._type == grep::LineType::Match {
                ripgrep_line.data.submatches.first().map(|m| m.start + 1)
            } else {
                // Inverted searches can put submatches in context records, but
                // those are not search hits. Context links still target column 1.
                None
            };
            let mut code = ripgrep_line.data.lines.text;
            // Keep newlines so the syntax highlighter handles C-style line comments
            // correctly. Also remove \r, see [EndCRLF] in src/delta.rs, but this time
            // it is syntect which adds an ANSI escape sequence in between \r\n later.
            if code.ends_with("\r\n") {
                code.truncate(code.len() - 2);
                code.push('\n');
            }
            Some(grep::GrepLine {
                grep_type: crate::config::GrepType::Ripgrep,
                line_type: ripgrep_line._type,
                line_number: ripgrep_line.data.line_number,
                column_number,
                path: Cow::from(ripgrep_line.data.path.text),
                code: Cow::from(code),
                submatches: Some(
                    ripgrep_line
                        .data
                        .submatches
                        .iter()
                        .map(|m| (m.start, m.end))
                        .collect(),
                ),
            })
        }
        None => {
            let value: Value = serde_json::from_str(line).ok()?;
            match &value["type"] {
                Value::String(s) if s == "begin" || s == "end" || s == "summary" => {
                    Some(grep::GrepLine {
                        // ripgrep --json also emits these metadata lines at
                        // file boundaries. We emit nothing but signal that the
                        // line has been handled.
                        grep_type: crate::config::GrepType::Ripgrep,
                        line_type: grep::LineType::Ignore,
                        line_number: None,
                        column_number: None,
                        path: "".into(),
                        code: "".into(),
                        submatches: None,
                    })
                }
                _ => {
                    // Failed to interpret the line as ripgrep output; allow
                    // another delta handler to try.
                    None
                }
            }
        }
    }
}

//   {
//     "type": "match",
//     "data": {
//       "path": {
//         "text": "src/cli.rs"
//       },
//       "lines": {
//         "text": "    fn from_clap_and_git_config(\n"
//       },
//       "line_number": null,
//       "absolute_offset": 35837,
//       "submatches": [
//         {
//           "match": {
//             "text": "fn"
//           },
//           "start": 4,
//           "end": 6
//         }
//       ]
//     }
//   }

#[derive(Deserialize, PartialEq, Debug)]
struct RipGrepLine {
    #[serde(rename(deserialize = "type"))]
    _type: grep::LineType,
    data: RipGrepLineData,
}

#[derive(Deserialize, PartialEq, Debug)]
struct RipGrepLineData {
    path: RipGrepLineText,
    lines: RipGrepLineText,
    line_number: Option<usize>,
    absolute_offset: usize,
    submatches: Vec<RipGrepLineSubmatch>,
}

#[derive(Deserialize, PartialEq, Debug)]
struct RipGrepLineText {
    text: String,
}

#[derive(Deserialize, PartialEq, Debug)]
struct RipGrepLineSubmatch {
    #[serde(rename(deserialize = "match"))]
    _match: RipGrepLineText,
    start: usize,
    end: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::integration_test_utils::DeltaTest;
    use insta::assert_snapshot;
    use rstest::rstest;

    fn hyperlink_test(output_type: &str, link_format: &str) -> DeltaTest<'static> {
        DeltaTest::with_args(&[
            "--hyperlinks",
            "--hyperlinks-file-link-format",
            link_format,
            "--grep-output-type",
            output_type,
        ])
    }

    fn match_record(
        line_type: &str,
        code: &str,
        line_number: Option<usize>,
        submatches: &[(usize, usize)],
    ) -> String {
        serde_json::json!({
            "type": line_type,
            "data": {
                "path": {"text": "test_file.rs"},
                "lines": {"text": code},
                "line_number": line_number,
                "absolute_offset": 144,
                "submatches": submatches.iter().map(|&(start, end)| serde_json::json!({
                    "match": {"text": &code[start..end]}, "start": start, "end": end,
                })).collect::<Vec<_>>(),
            },
        })
        .to_string()
    }

    fn file_link_target(suffix: &str) -> String {
        let path = crate::utils::path::fake_delta_cwd_for_tests().join("test_file.rs");
        format!("\x1b]8;;file://{}{suffix}\x1b\\", path.display())
    }

    #[rstest]
    fn test_ripgrep_hyperlink_column(#[values("ripgrep", "classic")] output_type: &str) {
        let data = match_record("match", "xx world world\n", Some(6), &[(3, 8), (9, 14)]);
        let result = hyperlink_test(output_type, "file://{path}:{line}:{column}")
            .with_input(&data)
            .expect_raw_contains(&file_link_target(":6:4"));
        let line_only = hyperlink_test(output_type, "file://{path}:{line}").with_input(&data);
        assert_eq!(result.output, line_only.output);
        if output_type == "ripgrep" {
            result.expect_raw_contains(&file_link_target(":0:1"));
        }
    }

    #[rstest]
    #[case::first_byte("match", "world\n", &[(0, 5)], 1)]
    #[case::zero_width("match", "xx world\n", &[(3, 3)], 4)]
    #[case::empty_submatches("match", "other\n", &[], 1)]
    #[case::inverted_context("context", "xx world\n", &[(3, 8)], 1)]
    #[case::utf8("match", "é world\n", &[(3, 8)], 4)]
    fn test_ripgrep_hyperlink_column_positions(
        #[values("ripgrep", "classic")] output_type: &str,
        #[case] line_type: &str,
        #[case] code: &str,
        #[case] submatches: &[(usize, usize)],
        #[case] column: usize,
    ) {
        let data = match_record(line_type, code, Some(6), submatches);
        hyperlink_test(output_type, "file://{path}:{line}:{column}")
            .with_input(&data)
            .expect_raw_contains(&file_link_target(&format!(":6:{column}")));
    }

    #[rstest]
    #[case::before_match("\tworld\n", &[(1, 6)], 2)]
    #[case::after_match("world\t\n", &[(0, 5)], 1)]
    fn test_ripgrep_hyperlink_column_ignores_tab_expansion(
        #[values("ripgrep", "classic")] output_type: &str,
        #[case] code: &str,
        #[case] submatches: &[(usize, usize)],
        #[case] column: usize,
    ) {
        hyperlink_test(output_type, "file://{path}:{line}:{column}")
            .set_config(|config| config.tab_cfg = crate::utils::tabs::TabCfg::new(4))
            .with_input(&match_record("match", code, Some(6), submatches))
            .expect_raw_contains(&file_link_target(&format!(":6:{column}")));
    }

    #[rstest]
    #[case("file://{path}", "")]
    #[case("file://{path}:{line}", ":6")]
    fn test_ripgrep_hyperlink_column_is_opt_in(#[case] link_format: &str, #[case] suffix: &str) {
        hyperlink_test("classic", link_format)
            .with_input(&match_record("match", "xx world\n", Some(6), &[(3, 8)]))
            .expect_raw_contains(&file_link_target(suffix));
    }

    #[rstest]
    fn test_ripgrep_hyperlink_column_without_line_number(
        #[values("ripgrep", "classic")] output_type: &str,
    ) {
        let result = hyperlink_test(output_type, "file://{path}:{line}:{column}")
            .with_input(&match_record("match", "xx world\n", None, &[(3, 8)]));
        assert!(!result.output.contains("1:"));
        assert_eq!(result.raw_output.matches("file://").count(), 1);
        result.expect_raw_contains(&file_link_target(if output_type == "classic" {
            ":1:4"
        } else {
            ":0:1"
        }));
    }

    #[test]
    fn test_ripgrep_hyperlink_column_with_hyperlinks_disabled() {
        let data = match_record("match", "xx world\n", Some(6), &[(3, 8)]);
        let test = hyperlink_test("ripgrep", "file://{path}:{line}:{column}");
        let enabled = test.with_input(&data);
        let disabled = test
            .set_config(|config| config.hyperlinks = false)
            .with_input(&data);
        assert!(!disabled.raw_output.contains("\x1b]8;"));
        assert_eq!(disabled.output, enabled.output);
    }

    #[test]
    fn test_non_json_grep_hyperlink_column_defaults_to_one() {
        hyperlink_test("classic", "file://{path}:{line}:{column}")
            .with_calling_process("git grep -n world")
            .with_input("test_file.rs:6:xx world")
            .expect_raw_contains(&file_link_target(":6:1"));
    }

    /* FILE test.c:
    // i ABC
    int f() { return 4; }
    const char* i = "ABC";
    double n = 1.23;
     */
    #[test]
    fn test_syntax_in_rg_output_with_context() {
        // `rg int  -C2 --json test.c`
        let data = r#"{"type":"begin","data":{"path":{"text":"test.c"}}}
{"type":"context","data":{"path":{"text":"test.c"},"lines":{"text":"// i ABC\n"},"line_number":1,"absolute_offset":0,"submatches":[]}}
{"type":"match","data":{"path":{"text":"test.c"},"lines":{"text":"int f() { return 4; }\n"},"line_number":2,"absolute_offset":9,"submatches":[{"match":{"text":"int"},"start":0,"end":3}]}}
{"type":"context","data":{"path":{"text":"test.c"},"lines":{"text":"const char* i = \"ABC\";\n"},"line_number":3,"absolute_offset":31,"submatches":[]}}
{"type":"context","data":{"path":{"text":"test.c"},"lines":{"text":"double n = 1.23;\n"},"line_number":4,"absolute_offset":54,"submatches":[]}}
{"type":"end","data":{"path":{"text":"test.c"},"binary_offset":null,"stats":{"elapsed":{"secs":0,"nanos":26941,"human":"0.000027s"},"searches":1,"searches_with_match":1,"bytes_searched":71,"bytes_printed":670,"matched_lines":1,"matches":1}}}
{"data":{"elapsed_total":{"human":"0.000479s","nanos":478729,"secs":0},"stats":{"bytes_printed":670,"bytes_searched":71,"elapsed":{"human":"0.000027s","nanos":26941,"secs":0},"matched_lines":1,"matches":1,"searches":1,"searches_with_match":1}},"type":"summary"}"#;
        let result = DeltaTest::with_args(&[]).explain_ansi().with_input(data);
        // eprintln!("{}", result.raw_output);
        assert_snapshot!(result.output, @r#"
        (purple)test.c(normal) 
        (green)1(normal)-(242)// i ABC(normal)
        (green)2(normal):(81 28)int(231) (149)f(231)() { (203)return(231) (141)4(231); }(normal)
        (green)3(normal)-(203)const(231) (81)char(203)*(231) i (203)=(231) (186)"ABC"(231);(normal)
        (green)4(normal)-(81)double(231) n (203)=(231) (141)1.23(231);(normal)
        "#);
    }

    #[test]
    fn test_syntax_in_rg_output_no_context() {
        // `rg i  --json test.c`
        let data = r#"{"type":"begin","data":{"path":{"text":"test.c"}}}
{"type":"match","data":{"path":{"text":"test.c"},"lines":{"text":"// i ABC\n"},"line_number":1,"absolute_offset":0,"submatches":[{"match":{"text":"i"},"start":3,"end":4}]}}
{"type":"match","data":{"path":{"text":"test.c"},"lines":{"text":"int f() { return 4; }\n"},"line_number":2,"absolute_offset":9,"submatches":[{"match":{"text":"i"},"start":0,"end":1}]}}
{"type":"match","data":{"path":{"text":"test.c"},"lines":{"text":"const char* i = \"ABC\";\n"},"line_number":3,"absolute_offset":31,"submatches":[{"match":{"text":"i"},"start":12,"end":13}]}}
{"type":"end","data":{"path":{"text":"test.c"},"binary_offset":null,"stats":{"elapsed":{"secs":0,"nanos":23885,"human":"0.000024s"},"searches":1,"searches_with_match":1,"bytes_searched":71,"bytes_printed":602,"matched_lines":3,"matches":3}}}
{"data":{"elapsed_total":{"human":"0.000433s","nanos":432974,"secs":0},"stats":{"bytes_printed":602,"bytes_searched":71,"elapsed":{"human":"0.000024s","nanos":23885,"secs":0},"matched_lines":3,"matches":3,"searches":1,"searches_with_match":1}},"type":"summary"}
"#;
        let result = DeltaTest::with_args(&[]).explain_ansi().with_input(data);
        // eprintln!("{}", result.raw_output);
        assert_snapshot!(result.output, @r#"
        (purple)test.c(normal) 
        (green)1(normal):(242)// (normal 28)i(242) ABC(normal)
        (green)2(normal):(81 28)i(81)nt(231) (149)f(231)() { (203)return(231) (141)4(231); }(normal)
        (green)3(normal):(203)const(231) (81)char(203)*(231) (normal 28)i(231) (203)=(231) (186)"ABC"(231);(normal)
        "#);
    }

    #[test]
    fn test_deserialize() {
        let line = r#"{"type":"match","data":{"path":{"text":"src/cli.rs"},"lines":{"text":"    fn from_clap_and_git_config(\n"},"line_number":null,"absolute_offset":35837,"submatches":[{"match":{"text":"fn"},"start":4,"end":6}]}}"#;
        let ripgrep_line: RipGrepLine = serde_json::from_str(line).unwrap();
        assert_eq!(
            ripgrep_line,
            RipGrepLine {
                _type: grep::LineType::Match,
                data: RipGrepLineData {
                    path: RipGrepLineText {
                        text: "src/cli.rs".into()
                    },
                    lines: RipGrepLineText {
                        text: "    fn from_clap_and_git_config(\n".into(),
                    },
                    line_number: None,
                    absolute_offset: 35837,
                    submatches: vec![RipGrepLineSubmatch {
                        _match: RipGrepLineText { text: "fn".into() },
                        start: 4,
                        end: 6
                    }]
                }
            }
        )
    }

    #[test]
    fn test_deserialize_2() {
        let line = r#"{"type":"match","data":{"path":{"text":"src/handlers/submodule.rs"},"lines":{"text":"                        .paint(minus_commit.chars().take(7).collect::<String>()),\n"},"line_number":41,"absolute_offset":1430,"submatches":[{"match":{"text":"("},"start":30,"end":31},{"match":{"text":"("},"start":49,"end":50},{"match":{"text":")"},"start":50,"end":51},{"match":{"text":"("},"start":56,"end":57},{"match":{"text":")"},"start":58,"end":59},{"match":{"text":"("},"start":77,"end":78},{"match":{"text":")"},"start":78,"end":79},{"match":{"text":")"},"start":79,"end":80}]}}"#;
        let ripgrep_line: RipGrepLine = serde_json::from_str(line).unwrap();
        assert_eq!(
            ripgrep_line,
            RipGrepLine {
                _type: grep::LineType::Match,
                data: RipGrepLineData {
                    path: RipGrepLineText {
                        text: "src/handlers/submodule.rs".into()
                    },
                    lines: RipGrepLineText {
                        text: "                        .paint(minus_commit.chars().take(7).collect::<String>()),\n".into(),
                    },
                    line_number: Some(41),
                    absolute_offset: 1430,
                    submatches: vec![
                        RipGrepLineSubmatch {
                            _match: RipGrepLineText { text: "(".into() },
                            start: 30,
                            end: 31
                        },
                        RipGrepLineSubmatch {
                            _match: RipGrepLineText { text: "(".into() },
                            start: 49,
                            end: 50
                        },
                        RipGrepLineSubmatch {
                            _match: RipGrepLineText { text: ")".into() },
                            start: 50,
                            end: 51
                        },
                        RipGrepLineSubmatch {
                            _match: RipGrepLineText { text: "(".into() },
                            start: 56,
                            end: 57
                        },
                        RipGrepLineSubmatch {
                            _match: RipGrepLineText { text: ")".into() },
                            start: 58,
                            end: 59
                        },
                        RipGrepLineSubmatch {
                            _match: RipGrepLineText { text: "(".into() },
                            start: 77,
                            end: 78
                        },
                        RipGrepLineSubmatch {
                            _match: RipGrepLineText { text: ")".into() },
                            start: 78,
                            end: 79
                        },
                        RipGrepLineSubmatch {
                            _match: RipGrepLineText { text: ")".into() },
                            start: 79,
                            end: 80
                        },
                    ]
                }
            }
        )
    }
}
