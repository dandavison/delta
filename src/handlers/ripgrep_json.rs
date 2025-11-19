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
