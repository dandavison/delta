use std::borrow::Cow;

use super::draw;
use crate::delta::{State, StateMachine};
use crate::features;
use crate::features::diff_line_metadata::{commit_id, write_with_header_osc};

impl StateMachine<'_> {
    #[inline]
    fn test_commit_meta_header_line(&self) -> bool {
        self.config.commit_regex.is_match(&self.line)
    }

    pub fn handle_commit_meta_header_line(&mut self) -> std::io::Result<bool> {
        if !self.test_commit_meta_header_line() {
            return Ok(false);
        }
        let mut handled_line = false;
        self.painter.paint_buffered_minus_and_plus_lines();
        self.handle_pending_line_with_diff_name()?;
        self.state = State::CommitMeta;
        if self.should_handle() {
            self.painter.emit()?;
            self._handle_commit_meta_header_line()?;
            handled_line = true
        } else {
            // A raw commit style passes the line through undrawn, so the record
            // is written here instead. Nothing happens when there is none, so
            // the output of a delta no host is asking anything of is unchanged.
            let osc = self.commit_osc();
            if !osc.is_empty() {
                self.painter.emit()?;
                write!(self.painter.writer, "{osc}")?;
            }
        }
        Ok(handled_line)
    }

    fn _handle_commit_meta_header_line(&mut self) -> std::io::Result<()> {
        if self.config.commit_style.is_omitted {
            return Ok(());
        }
        let (mut draw_fn, pad, decoration_ansi_term_style) =
            draw::get_draw_function(self.config.commit_style.decoration_style);
        let (formatted_line, formatted_raw_line) = if self.config.hyperlinks {
            (
                features::hyperlinks::format_commit_line_with_osc8_commit_hyperlink(
                    &self.line,
                    self.config,
                ),
                features::hyperlinks::format_commit_line_with_osc8_commit_hyperlink(
                    &self.raw_line,
                    self.config,
                ),
            )
        } else {
            (Cow::from(&self.line), Cow::from(&self.raw_line))
        };

        // Prefix every drawn row with the `C` record (no-op when empty), as the
        // file and hunk headers do for theirs.
        let commit_osc = self.commit_osc();
        let config = self.config;
        write_with_header_osc(self.painter.writer, &commit_osc, |w| {
            draw_fn(
                w,
                &format!("{}{}", formatted_line, if pad { " " } else { "" }),
                &format!("{}{}", formatted_raw_line, if pad { " " } else { "" }),
                "",
                &config.decorations_width,
                config.commit_style,
                decoration_ansi_term_style,
            )
        })
    }

    /// The `C` (commit) record for the commit-header line being drawn, or empty
    /// when metadata isn't negotiated or the line names no commit.
    fn commit_osc(&self) -> String {
        let Some(metadata) = self.painter.diff_line_metadata.as_ref() else {
            return String::new();
        };
        let Some(marker) = self.config.commit_regex.find(&self.line) else {
            return String::new();
        };
        commit_id(&self.line, marker.end())
            .map_or(String::new(), |commit| metadata.osc_for_commit(commit))
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::integration_test_utils::{
        make_config_from_args, run_delta, run_delta_with_diff_line_metadata,
    };
    use insta::assert_snapshot;

    const LOG: &str = "\
commit 8a9c3f2b1d4e5f60718293a4b5c6d7e8f9012345
Author: Someone <someone@example.com>
Date:   Mon Jan 1 00:00:00 2024 +0000

    A commit
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
    fn test_commit_record_on_every_row_the_header_draws() {
        // The record is on the commit row, and on the decoration row the style
        // adds; the rest of the commit block carries nothing.
        assert_snapshot!(visible_metadata_records(LOG, &["--commit-decoration-style", "blue ol"]), @"
        ⟦1;C;;;8a9c3f2b1d4e5f60718293a4b5c6d7e8f9012345⟧───────────────────────────────────────────────
        ⟦1;C;;;8a9c3f2b1d4e5f60718293a4b5c6d7e8f9012345⟧commit 8a9c3f2b1d4e5f60718293a4b5c6d7e8f9012345
        Author: Someone <someone@example.com>
        Date:   Mon Jan 1 00:00:00 2024 +0000

            A commit
        ");
    }

    #[test]
    fn test_commit_record_on_a_raw_commit_style_passed_through_undrawn() {
        assert_snapshot!(
            visible_metadata_records(
                LOG,
                &["--commit-style", "raw", "--commit-decoration-style", "none"]
            ),
            @r"
        ⟦1;C;;;8a9c3f2b1d4e5f60718293a4b5c6d7e8f9012345⟧commit 8a9c3f2b1d4e5f60718293a4b5c6d7e8f9012345
        Author: Someone <someone@example.com>
        Date:   Mon Jan 1 00:00:00 2024 +0000

            A commit
        "
        );
    }

    #[test]
    fn test_no_commit_record_when_no_host_negotiated() {
        let config = make_config_from_args(&["--commit-style", "raw"]);
        assert!(!run_delta(LOG, &config).contains("1717"));
    }
}
