use std::cmp::min;
use std::ops::{Index, IndexMut};

use super::draw;
use crate::cli;
use crate::config::{self, delta_unreachable};
use crate::delta::{DiffType, MergeParents, State, StateMachine};
use crate::minusplus::MinusPlus;
use crate::paint;
use crate::style::DecorationStyle;

#[derive(Clone, Debug, PartialEq)]
pub enum MergeConflictCommit {
    Ours,
    Ancestral,
    Theirs,
}

pub struct MergeConflictCommits<T> {
    ours: T,
    ancestral: T,
    theirs: T,
}

pub type MergeConflictLines = MergeConflictCommits<Vec<(String, State)>>;

pub type MergeConflictCommitNames = MergeConflictCommits<Option<String>>;

impl<'a> StateMachine<'a> {
    pub fn handle_merge_conflict_line(&mut self) -> std::io::Result<bool> {
        use DiffType::*;
        use MergeConflictCommit::*;
        use MergeParents::*;
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
            HunkZero(_) => handled_line = self.enter_merge_conflict(),
            MergeConflict(Ours) => {
                handled_line = self.enter_ancestral()
                    || self.enter_theirs()
                    || self.exit_merge_conflict(diff_type.clone())?
                    || self.store_line(Ours, HunkPlus(diff_type, None));
            }
            MergeConflict(Ancestral) => {
                handled_line = self.enter_theirs()
                    || self.exit_merge_conflict(diff_type.clone())?
                    || self.store_line(Ancestral, HunkMinus(diff_type, None));
            }
            MergeConflict(Theirs) => {
                handled_line = self.exit_merge_conflict(diff_type.clone())?
                    || self.store_line(Theirs, HunkPlus(diff_type, None));
            }
            _ => {}
        }

        Ok(handled_line)
    }

    fn enter_merge_conflict(&mut self) -> bool {
        use State::*;
        if let Some(commit) = parse_merge_marker(&self.line, "++<<<<<<<") {
            self.state = MergeConflict(Ours);
            self.painter.merge_conflict_commit_names[Ours] = Some(commit.to_string());
            true
        } else {
            false
        }
    }

    fn enter_ancestral(&mut self) -> bool {
        use State::*;
        if let Some(commit) = parse_merge_marker(&self.line, "++|||||||") {
            self.state = MergeConflict(Ancestral);
            self.painter.merge_conflict_commit_names[Ancestral] = Some(commit.to_string());
            true
        } else {
            false
        }
    }

    fn enter_theirs(&mut self) -> bool {
        use State::*;
        if self.line.starts_with("++=======") {
            self.state = MergeConflict(Theirs);
            true
        } else {
            false
        }
    }

    fn exit_merge_conflict(&mut self, diff_type: DiffType) -> std::io::Result<bool> {
        if let Some(commit) = parse_merge_marker(&self.line, "++>>>>>>>") {
            self.painter.merge_conflict_commit_names[Theirs] = Some(commit.to_string());
            self.paint_buffered_merge_conflict_lines(diff_type)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn store_line(&mut self, commit: MergeConflictCommit, state: State) -> bool {
        use State::*;
        if let HunkMinus(diff_type, _) | HunkZero(diff_type) | HunkPlus(diff_type, _) = &state {
            let line = self.painter.prepare(&self.line, diff_type.n_parents());
            self.painter.merge_conflict_lines[commit].push((line, state));
            true
        } else {
            delta_unreachable(&format!("Invalid state: {:?}", state))
        }
    }

    fn paint_buffered_merge_conflict_lines(&mut self, diff_type: DiffType) -> std::io::Result<()> {
        self.painter.emit()?;

        write_merge_conflict_bar("▼", &mut self.painter, self.config)?;
        for (derived_commit_type, decoration_style) in &[(Ours, "box"), (Theirs, "box")] {
            write_subhunk_header(
                derived_commit_type,
                decoration_style,
                &mut self.painter,
                self.config,
            )?;
            self.painter.emit()?;
            paint::paint_minus_and_plus_lines(
                MinusPlus::new(
                    &self.painter.merge_conflict_lines[Ancestral],
                    &self.painter.merge_conflict_lines[derived_commit_type],
                ),
                &mut self.painter.line_numbers_data,
                &mut self.painter.highlighter,
                &mut self.painter.output_buffer,
                self.config,
            );
            self.painter.emit()?;
        }
        // write_merge_conflict_decoration("bold ol", &mut self.painter, self.config)?;
        write_merge_conflict_bar("▲", &mut self.painter, self.config)?;
        self.painter.merge_conflict_lines.clear();
        self.state = State::HunkZero(diff_type);
        Ok(())
    }
}

fn write_subhunk_header(
    derived_commit_type: &MergeConflictCommit,
    decoration_style: &str,
    painter: &mut paint::Painter,
    config: &config::Config,
) -> std::io::Result<()> {
    let (mut draw_fn, pad, decoration_ansi_term_style) =
        draw::get_draw_function(DecorationStyle::from_str(
            decoration_style,
            config.true_color,
            config.git_config.as_ref(),
        ));
    let derived_commit_name = &painter.merge_conflict_commit_names[derived_commit_type];
    let text = if let Some(_ancestral_commit) = &painter.merge_conflict_commit_names[Ancestral] {
        format!(
            "ancestor {} {}{}",
            config.right_arrow,
            derived_commit_name.as_deref().unwrap_or("?"),
            if pad { " " } else { "" }
        )
    } else {
        derived_commit_name.as_deref().unwrap_or("?").to_string()
    };
    draw_fn(
        painter.writer,
        &text,
        &text,
        &config.decorations_width,
        config.hunk_header_style,
        decoration_ansi_term_style,
    )?;
    Ok(())
}

#[allow(unused)]
fn write_merge_conflict_line(
    painter: &mut paint::Painter,
    config: &config::Config,
) -> std::io::Result<()> {
    let (mut draw_fn, _pad, decoration_ansi_term_style) = draw::get_draw_function(
        DecorationStyle::from_str("bold ol", config.true_color, config.git_config.as_ref()),
    );
    draw_fn(
        painter.writer,
        "",
        "",
        &config.decorations_width,
        config.hunk_header_style,
        decoration_ansi_term_style,
    )?;
    Ok(())
}

fn write_merge_conflict_bar(
    s: &str,
    painter: &mut paint::Painter,
    config: &config::Config,
) -> std::io::Result<()> {
    if let cli::Width::Fixed(width) = config.decorations_width {
        writeln!(painter.writer, "{}", s.repeat(width))?;
    }
    Ok(())
}

fn parse_merge_marker<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    match line.strip_prefix(marker) {
        Some(suffix) => {
            let suffix = suffix.trim();
            if !suffix.is_empty() {
                Some(suffix)
            } else {
                None
            }
        }
        None => None,
    }
}

pub use MergeConflictCommit::*;

impl<T> Index<MergeConflictCommit> for MergeConflictCommits<T> {
    type Output = T;
    fn index(&self, commit: MergeConflictCommit) -> &Self::Output {
        match commit {
            Ours => &self.ours,
            Ancestral => &self.ancestral,
            Theirs => &self.theirs,
        }
    }
}

impl<T> Index<&MergeConflictCommit> for MergeConflictCommits<T> {
    type Output = T;
    fn index(&self, commit: &MergeConflictCommit) -> &Self::Output {
        match commit {
            Ours => &self.ours,
            Ancestral => &self.ancestral,
            Theirs => &self.theirs,
        }
    }
}

impl<T> IndexMut<MergeConflictCommit> for MergeConflictCommits<T> {
    fn index_mut(&mut self, commit: MergeConflictCommit) -> &mut Self::Output {
        match commit {
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

impl MergeConflictCommitNames {
    pub fn new() -> Self {
        Self {
            ours: None,
            ancestral: None,
            theirs: None,
        }
    }
}
