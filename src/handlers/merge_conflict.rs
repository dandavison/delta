use std::cmp::min;
use std::ops::{Index, IndexMut};

use crate::delta::{DiffType, MergeParents, State, StateMachine};
use crate::minusplus::MinusPlus;
use crate::paint;

#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    Ours,
    Ancestral,
    Theirs,
}

pub struct MergeConflictLines {
    ours: Vec<(String, State)>,
    ancestral: Vec<(String, State)>,
    theirs: Vec<(String, State)>,
}

impl<'a> StateMachine<'a> {
    pub fn handle_merge_conflict_line(&mut self) -> std::io::Result<bool> {
        use DiffType::*;
        use MergeParents::*;
        use Source::*;
        use State::*;

        let mut handled_line = false;
        if self.config.color_only || !self.config.handle_merge_conflicts {
            return Ok(handled_line);
        }

        // TODO: don't allocate on heap at this point
        let prefix = self.line[..min(self.line.len(), 2)].to_string();
        let diff_type = Combined(Prefix(prefix));

        match self.state {
            // The only transition into a merge conflict is HunkZero => MergeConflict(Ours)
            // TODO: shouldn't this be HunkZero(Some(_))?
            HunkZero(_) => {
                if self.line.starts_with("++<<<<<<<") {
                    self.state = MergeConflict(Ours);
                    handled_line = true
                }
            }
            MergeConflict(Ours) => {
                if self.line.starts_with("++|||||||") {
                    self.state = MergeConflict(Ancestral);
                } else if self.line.starts_with("++=======") {
                    self.state = MergeConflict(Theirs);
                } else if self.line.starts_with("++>>>>>>>") {
                    self.paint_buffered_merge_conflict_lines(diff_type)?;
                } else {
                    let line = self.painter.prepare(&self.line, diff_type.n_parents());
                    self.painter.merge_conflict_lines[Ours].push((line, HunkPlus(diff_type, None)));
                }
                handled_line = true
            }
            MergeConflict(Ancestral) => {
                if self.line.starts_with("++=======") {
                    self.state = MergeConflict(Theirs);
                } else if self.line.starts_with("++>>>>>>>") {
                    self.paint_buffered_merge_conflict_lines(diff_type)?;
                } else {
                    let line = self.painter.prepare(&self.line, diff_type.n_parents());
                    self.painter.merge_conflict_lines[Ancestral]
                        .push((line, HunkMinus(diff_type, None)));
                }
                handled_line = true
            }
            MergeConflict(Theirs) => {
                if self.line.starts_with("++>>>>>>>") {
                    self.paint_buffered_merge_conflict_lines(diff_type)?;
                } else {
                    let line = self.painter.prepare(&self.line, diff_type.n_parents());
                    self.painter.merge_conflict_lines[Theirs]
                        .push((line, HunkPlus(diff_type, None)));
                }
                handled_line = true
            }
            _ => {}
        }

        Ok(handled_line)
    }

    fn paint_buffered_merge_conflict_lines(&mut self, diff_type: DiffType) -> std::io::Result<()> {
        self.painter.emit()?;
        let lines = &self.painter.merge_conflict_lines;
        for derived_lines in &[&lines[Ours], &lines[Theirs]] {
            paint::paint_minus_and_plus_lines(
                MinusPlus::new(&lines[Ancestral], derived_lines),
                &mut self.painter.line_numbers_data,
                &mut self.painter.highlighter,
                &mut self.painter.output_buffer,
                self.config,
            );
            self.painter.output_buffer.push_str("\n\n");
        }
        self.painter.merge_conflict_lines.clear();
        self.state = State::HunkZero(diff_type);
        Ok(())
    }
}

pub use Source::*;

impl Index<Source> for MergeConflictLines {
    type Output = Vec<(String, State)>;
    fn index(&self, source: Source) -> &Self::Output {
        match source {
            Ours => &self.ours,
            Ancestral => &self.ancestral,
            Theirs => &self.theirs,
        }
    }
}

impl IndexMut<Source> for MergeConflictLines {
    fn index_mut(&mut self, source: Source) -> &mut Self::Output {
        match source {
            Ours => &mut self.ours,
            Ancestral => &mut self.ancestral,
            Theirs => &mut self.theirs,
        }
    }
}

impl MergeConflictLines {
    pub fn new() -> Self {
        Self {
            ours: Vec::new(),
            ancestral: Vec::new(),
            theirs: Vec::new(),
        }
    }

    fn clear(&mut self) {
        self[Ours].clear();
        self[Ancestral].clear();
        self[Theirs].clear();
    }
}
