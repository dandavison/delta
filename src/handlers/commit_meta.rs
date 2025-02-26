use std::borrow::Cow;

use super::draw;
use crate::delta::{State, StateMachine};
use crate::features;

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
        }
        Ok(handled_line)
    }

    fn _handle_commit_meta_header_line(&mut self) -> std::io::Result<()> {
        if self.config.commit_style.is_omitted {
            return Ok(());
        }
        let draw_fn = draw::get_draw_function(self.config.commit_style.decoration_style);
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

        draw_fn(
            self.painter.writer,
            &formatted_line,
            &formatted_raw_line,
            "",
            &self.config.decorations_width,
            self.config.commit_style,
            false,
        )?;

        Ok(())
    }
}
