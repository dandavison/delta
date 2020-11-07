use std::borrow::Cow;

use crate::config::Config;
use crate::features;

/// If output is going to a tty, emit hyperlinks if requested.
// Although raw output should basically be emitted unaltered, we do this.
pub fn format_raw_line<'a>(line: &'a str, config: &Config) -> Cow<'a, str> {
    if config.hyperlinks && atty::is(atty::Stream::Stdout) {
        features::hyperlinks::format_commit_line_with_osc8_commit_hyperlink(line, config)
    } else {
        Cow::from(line)
    }
}
