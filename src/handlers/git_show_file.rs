use crate::delta::{State, StateMachine};
use crate::paint::{BgShouldFill, StyleSectionSpecifier};
use crate::utils::process;

impl<'a> StateMachine<'a> {
    // If this is a line of `git show $revision:/path/to/file.ext` output then
    // syntax-highlight it as language `ext`.
    pub fn handle_git_show_file_line(&mut self) -> std::io::Result<bool> {
        self.painter.emit()?;
        let mut handled_line = false;
        if matches!(self.state, State::Unknown) {
            if let process::CallingProcess::GitShow(_, extension) = &*process::calling_process() {
                self.state = State::GitShowFile;
                self.painter.set_syntax(extension.as_deref());
                self.painter.set_highlighter();
            } else {
                return Ok(handled_line);
            }
        }
        if matches!(self.state, State::GitShowFile) {
            self.painter.syntax_highlight_and_paint_line(
                &self.line,
                StyleSectionSpecifier::Style(self.config.zero_style),
                self.state.clone(),
                BgShouldFill::default(),
            );
            handled_line = true;
        }
        Ok(handled_line)
    }
}
