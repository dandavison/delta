use regex::Regex;

#[derive(Debug, Default, PartialEq)]
pub struct FormatStringPlaceholderData<'a> {
    pub prefix: &'a str,
    pub placeholder: Option<&'a str>,
    pub alignment_spec: Option<&'a str>,
    pub width: Option<usize>,
    pub suffix: &'a str,
}

pub type FormatStringData<'a> = Vec<FormatStringPlaceholderData<'a>>;

pub fn make_placeholder_regex(labels: &[&str]) -> Regex {
    Regex::new(&format!(
        r"(?x)
    \{{
    ({})            # 1: Placeholder labels
    (?:             # Start optional format spec (non-capturing)
      :             #     Literal colon
      (?:           #     Start optional fill/alignment spec (non-capturing)
        ([^<^>])?   #         2: Optional fill character (ignored)
        ([<^>])     #         3: Alignment spec
      )?            #
      (\d+)         #     4: Width
    )?              #
    \}}
    ",
        labels.join("|")
    ))
    .unwrap()
}

pub fn parse_line_number_format<'a>(
    format_string: &'a str,
    placeholder_regex: &Regex,
) -> FormatStringData<'a> {
    let mut format_data = Vec::new();
    let mut offset = 0;

    for captures in placeholder_regex.captures_iter(format_string) {
        let _match = captures.get(0).unwrap();
        format_data.push(FormatStringPlaceholderData {
            prefix: &format_string[offset.._match.start()],
            placeholder: captures.get(1).map(|m| m.as_str()),
            alignment_spec: captures.get(3).map(|m| m.as_str()),
            width: captures.get(4).map(|m| {
                m.as_str()
                    .parse()
                    .unwrap_or_else(|_| panic!("Invalid width in format string: {}", format_string))
            }),
            suffix: &format_string[_match.end()..],
        });
        offset = _match.end();
    }
    if offset == 0 {
        // No placeholders
        format_data.push(FormatStringPlaceholderData {
            prefix: &format_string[..0],
            placeholder: None,
            alignment_spec: None,
            width: None,
            suffix: &format_string[0..],
        })
    }
    format_data
}

pub fn pad(s: &str, width: usize, alignment: &str) -> String {
    match alignment {
        "<" => format!("{0:<1$}", s, width),
        "^" => format!("{0:^1$}", s, width),
        ">" => format!("{0:>1$}", s, width),
        _ => unreachable!(),
    }
}
