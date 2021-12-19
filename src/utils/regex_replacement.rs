use std::borrow::Cow;

use regex::{Regex, RegexBuilder};

#[derive(Clone, Debug)]
pub struct RegexReplacement {
    regex: Regex,
    replacement: String,
    replace_all: bool,
}

impl RegexReplacement {
    pub fn from_sed_command(sed_command: &str) -> Option<Self> {
        let sep = sed_command.chars().nth(1)?;
        let mut parts = sed_command[2..].split(sep);
        let regex = parts.next()?;
        let replacement = parts.next()?.to_string();
        let flags = parts.next()?;
        let mut re_builder = RegexBuilder::new(regex);
        let mut replace_all = false;
        for flag in flags.chars() {
            match flag {
                'g' => {
                    replace_all = true;
                }
                'i' => {
                    re_builder.case_insensitive(true);
                }
                'm' => {
                    re_builder.multi_line(true);
                }
                's' => {
                    re_builder.dot_matches_new_line(true);
                }
                'U' => {
                    re_builder.swap_greed(true);
                }
                'x' => {
                    re_builder.ignore_whitespace(true);
                }
                _ => {}
            }
        }
        let regex = re_builder.build().ok()?;
        Some(RegexReplacement {
            regex,
            replacement,
            replace_all,
        })
    }

    pub fn execute<'t>(&self, s: &'t str) -> Cow<'t, str> {
        if self.replace_all {
            self.regex.replace_all(s, &self.replacement)
        } else {
            self.regex.replace(s, &self.replacement)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sed_command() {
        let command = "s,foo,bar,";
        let rr = RegexReplacement::from_sed_command(command).unwrap();
        assert_eq!(rr.regex.as_str(), "foo");
        assert_eq!(rr.replacement, "bar");
        assert_eq!(rr.replace_all, false);
        assert_eq!(rr.execute("foo"), "bar");
    }

    #[test]
    fn test_sed_command_i_flag() {
        let command = "s,FOO,bar,";
        let rr = RegexReplacement::from_sed_command(command).unwrap();
        assert_eq!(rr.execute("foo"), "foo");
        let command = "s,FOO,bar,i";
        let rr = RegexReplacement::from_sed_command(command).unwrap();
        assert_eq!(rr.execute("foo"), "bar");
    }

    #[test]
    fn test_sed_command_g_flag() {
        let command = "s,foo,bar,";
        let rr = RegexReplacement::from_sed_command(command).unwrap();
        assert_eq!(rr.execute("foofoo"), "barfoo");
        let command = "s,foo,bar,g";
        let rr = RegexReplacement::from_sed_command(command).unwrap();
        assert_eq!(rr.execute("foofoo"), "barbar");
    }

    #[test]
    fn test_sed_command_with_named_captures() {
        let command = r"s/(?P<last>[^,\s]+),\s+(?P<first>\S+)/$first $last/";
        let rr = RegexReplacement::from_sed_command(command).unwrap();
        assert_eq!(rr.execute("Springsteen, Bruce"), "Bruce Springsteen");
    }

    #[test]
    fn test_sed_command_invalid() {
        assert!(RegexReplacement::from_sed_command("").is_none());
        assert!(RegexReplacement::from_sed_command("s").is_none());
        assert!(RegexReplacement::from_sed_command("s,").is_none());
        assert!(RegexReplacement::from_sed_command("s,,").is_none());
        assert!(RegexReplacement::from_sed_command("s,,i").is_none());
        assert!(RegexReplacement::from_sed_command("s,,,").is_some());
        assert!(RegexReplacement::from_sed_command("s,,,i").is_some());
    }
}
