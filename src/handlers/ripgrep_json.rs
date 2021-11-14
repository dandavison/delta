// See https://github.com/BurntSushi/ripgrep
// This module implements handling of `rg --json` output. It is called by the
// handler in handlers/grep.rs. Normal rg output (i.e. without --json) is
// handled by the same code paths as `git grep` etc output, in handlers/grep.rs.
use std::borrow::Cow;

use crate::handlers::grep;
use serde::Deserialize;
use serde_json::Value;

pub fn parse_line(line: &str) -> Option<grep::GrepLine> {
    let ripgrep_line: Option<RipGrepLine> = serde_json::from_str(line).ok();
    match ripgrep_line {
        Some(ripgrep_line) => {
            // A real line of rg --json output, i.e. either of type "match" or
            // "context".
            let mut code = ripgrep_line.data.lines.text;
            if code.ends_with('\n') {
                code.truncate(code.len() - 1);
                if code.ends_with('\r') {
                    code.truncate(code.len() - 1);
                }
            }
            Some(grep::GrepLine {
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
