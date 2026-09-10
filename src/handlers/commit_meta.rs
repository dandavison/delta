use std::borrow::Cow;

use super::draw;
use crate::delta::{State, StateMachine};
use crate::features;
use crate::features::diff_line_metadata::{commit_id, OscLinePrefixer};

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
            // With the default commit style the line is passed through
            // unchanged, and the record has to precede it just the same.
            self.painter.emit()?;
            write!(self.painter.writer, "{}", self.commit_osc())?;
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
        let mut writer = OscLinePrefixer::new(&mut *self.painter.writer, commit_osc);
        draw_fn(
            &mut writer,
            &format!("{}{}", formatted_line, if pad { " " } else { "" }),
            &format!("{}{}", formatted_raw_line, if pad { " " } else { "" }),
            "",
            &self.config.decorations_width,
            self.config.commit_style,
            decoration_ansi_term_style,
        )?;
        Ok(())
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
