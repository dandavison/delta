use std::ops::{Index, IndexMut};

use itertools::Itertools;
use unicode_segmentation::UnicodeSegmentation;

use super::draw;
use crate::cli;
use crate::config::{self, delta_unreachable};
use crate::delta::{DiffType, InMergeConflict, MergeParents, State, StateMachine};
use crate::minusplus::MinusPlus;
use crate::paint::{self, prepare};
use crate::style::Style;

#[derive(Clone, Debug, PartialEq, Eq)]
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
        use State::*;

        let mut handled_line = false;
        if self.config.color_only || !self.config.handle_merge_conflicts {
            return Ok(handled_line);
        }

        match self.state.clone() {
            HunkHeader(Combined(merge_parents, InMergeConflict::No), _, _, _)
            | HunkMinus(Combined(merge_parents, InMergeConflict::No), _)
            | HunkZero(Combined(merge_parents, InMergeConflict::No), _)
            | HunkPlus(Combined(merge_parents, InMergeConflict::No), _) => {
                handled_line = self.enter_merge_conflict(&merge_parents)
            }
            MergeConflict(merge_parents, Ours) => {
                handled_line = self.enter_ancestral(&merge_parents)
                    || self.enter_theirs(&merge_parents)
                    || self.exit_merge_conflict(&merge_parents)?
                    || self.store_line(
                        Ours,
                        HunkPlus(Combined(merge_parents, InMergeConflict::Yes), None),
                    );
            }
            MergeConflict(merge_parents, Ancestral) => {
                handled_line = self.enter_theirs(&merge_parents)
                    || self.exit_merge_conflict(&merge_parents)?
                    || self.store_line(
                        Ancestral,
                        HunkMinus(Combined(merge_parents, InMergeConflict::Yes), None),
                    );
            }
            MergeConflict(merge_parents, Theirs) => {
                handled_line = self.exit_merge_conflict(&merge_parents)?
                    || self.store_line(
                        Theirs,
                        HunkPlus(Combined(merge_parents, InMergeConflict::Yes), None),
                    );
            }
            _ => {}
        }

        Ok(handled_line)
    }

    fn enter_merge_conflict(&mut self, merge_parents: &MergeParents) -> bool {
        use State::*;
        if let Some(commit) = parse_merge_marker(&self.line, "++<<<<<<<") {
            self.state = MergeConflict(merge_parents.clone(), Ours);
            self.painter.merge_conflict_commit_names[Ours] = Some(commit.to_string());
            true
        } else {
            false
        }
    }

    fn enter_ancestral(&mut self, merge_parents: &MergeParents) -> bool {
        use State::*;
        if let Some(commit) = parse_merge_marker(&self.line, "++|||||||") {
            self.state = MergeConflict(merge_parents.clone(), Ancestral);
            self.painter.merge_conflict_commit_names[Ancestral] = Some(commit.to_string());
            true
        } else {
            false
        }
    }

    fn enter_theirs(&mut self, merge_parents: &MergeParents) -> bool {
        use State::*;
        if self.line.starts_with("++=======") {
            self.state = MergeConflict(merge_parents.clone(), Theirs);
            true
        } else {
            false
        }
    }

    fn exit_merge_conflict(&mut self, merge_parents: &MergeParents) -> std::io::Result<bool> {
        if let Some(commit) = parse_merge_marker(&self.line, "++>>>>>>>") {
            self.painter.merge_conflict_commit_names[Theirs] = Some(commit.to_string());
            self.paint_buffered_merge_conflict_lines(merge_parents)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn store_line(&mut self, commit: MergeConflictCommit, state: State) -> bool {
        use State::*;
        if let HunkMinus(diff_type, _) | HunkZero(diff_type, _) | HunkPlus(diff_type, _) = &state {
            let line = prepare(&self.line, diff_type.n_parents(), self.config);
            self.painter.merge_conflict_lines[commit].push((line, state));
            true
        } else {
            delta_unreachable(&format!("Invalid state: {:?}", state))
        }
    }

    fn paint_buffered_merge_conflict_lines(
        &mut self,
        merge_parents: &MergeParents,
    ) -> std::io::Result<()> {
        use DiffType::*;
        use State::*;
        self.painter.emit()?;

        write_merge_conflict_bar(
            &self.config.merge_conflict_begin_symbol,
            &mut self.painter,
            self.config,
        )?;
        for (derived_commit_type, header_style) in &[
            (Ours, self.config.merge_conflict_ours_diff_header_style),
            (Theirs, self.config.merge_conflict_theirs_diff_header_style),
        ] {
            write_diff_header(
                derived_commit_type,
                *header_style,
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
        write_merge_conflict_bar(
            &self.config.merge_conflict_end_symbol,
            &mut self.painter,
            self.config,
        )?;
        self.painter.merge_conflict_lines.clear();
        self.state = HunkZero(Combined(merge_parents.clone(), InMergeConflict::No), None);
        Ok(())
    }
}

fn write_diff_header(
    derived_commit_type: &MergeConflictCommit,
    style: Style,
    painter: &mut paint::Painter,
    config: &config::Config,
) -> std::io::Result<()> {
    let (mut draw_fn, pad, decoration_ansi_term_style) =
        draw::get_draw_function(style.decoration_style);
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
        "",
        &config.decorations_width,
        style,
        decoration_ansi_term_style,
    )?;
    Ok(())
}

fn write_merge_conflict_bar(
    s: &str,
    painter: &mut paint::Painter,
    config: &config::Config,
) -> std::io::Result<()> {
    let width = match config.decorations_width {
        cli::Width::Fixed(width) => width,
        cli::Width::Variable => config.available_terminal_width,
    };
    writeln!(
        painter.writer,
        "{}",
        &s.graphemes(true).cycle().take(width).join("")
    )?;
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

#[cfg(test)]
mod tests {
    use crate::ansi::strip_ansi_codes;
    use crate::tests::integration_test_utils;

    #[test]
    fn test_toy_merge_conflict_no_context() {
        let config = integration_test_utils::make_config_from_args(&[]);
        let output = integration_test_utils::run_delta(GIT_TOY_MERGE_CONFLICT_NO_CONTEXT, &config);
        let output = strip_ansi_codes(&output);
        assert!(output.contains("\n▼▼▼▼▼▼▼▼▼▼▼▼▼▼▼▼▼▼"));
        assert!(output.contains(
            "\
──────────────────┐
ancestor ⟶   HEAD │
──────────────────┘
"
        ));
        assert!(output.contains("\n▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲"));
    }

    #[test]
    fn test_real_merge_conflict() {
        let config = integration_test_utils::make_config_from_args(&[]);
        let output = integration_test_utils::run_delta(GIT_MERGE_CONFLICT, &config);
        let output = strip_ansi_codes(&output);
        assert!(output.contains("\n▼▼▼▼▼▼▼▼▼▼▼▼▼▼▼▼▼▼"));
        assert!(output.contains(
            "\
──────────────────┐
ancestor ⟶   HEAD │
──────────────────┘
"
        ));
        assert!(output.contains("\n▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲"));
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_real_merge_conflict_U0() {
        let config = integration_test_utils::make_config_from_args(&[]);
        let output = integration_test_utils::run_delta(GIT_MERGE_CONFLICT_U0, &config);
        let output = strip_ansi_codes(&output);
        assert!(output.contains("\n▼▼▼▼▼▼▼▼▼▼▼▼▼▼▼▼▼▼"));
        assert!(output.contains(
            "\
──────────────────┐
ancestor ⟶   HEAD │
──────────────────┘
"
        ));
        assert!(output.contains("\n▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲"));
    }

    const GIT_TOY_MERGE_CONFLICT_NO_CONTEXT: &str = "\
diff --cc file
index 6178079,7898192..0000000
--- a/file
+++ b/file
@@@ -1,1 -1,1 +1,6 @@@
++<<<<<<< HEAD
 +a
++||||||| parent of 0c20c9d... wip
++=======
+ b
++>>>>>>> 0c20c9d... wip
";

    const GIT_MERGE_CONFLICT: &str = r#"\
diff --cc src/handlers/merge_conflict.rs
index 27d47c0,3a7e7b9..0000000
--- a/src/handlers/merge_conflict.rs
+++ b/src/handlers/merge_conflict.rs
@@@ -1,14 -1,13 +1,24 @@@
 -use std::cmp::min;
  use std::ops::{Index, IndexMut};
  
++<<<<<<< HEAD
 +use itertools::Itertools;
 +use unicode_segmentation::UnicodeSegmentation;
 +
 +use super::draw;
 +use crate::cli;
 +use crate::config::{self, delta_unreachable};
 +use crate::delta::{DiffType, InMergeConflict, MergeParents, State, StateMachine};
++||||||| parent of b2b28c8... Display merge conflict branches
++use crate::delta::{DiffType, MergeParents, State, StateMachine};
++=======
+ use super::draw;
+ use crate::cli;
+ use crate::config::{self, delta_unreachable};
+ use crate::delta::{DiffType, MergeParents, State, StateMachine};
++>>>>>>> b2b28c8... Display merge conflict branches
  use crate::minusplus::MinusPlus;
  use crate::paint;
+ use crate::style::DecorationStyle;
  
  #[derive(Clone, Debug, PartialEq)]
  pub enum MergeConflictCommit {
@@@ -30,7 -29,8 +40,15 @@@ pub type MergeConflictCommitNames = Mer
  impl<'a> StateMachine<'a> {
      pub fn handle_merge_conflict_line(&mut self) -> std::io::Result<bool> {
          use DiffType::*;
++<<<<<<< HEAD
          use MergeConflictCommit::*;
++||||||| parent of b2b28c8... Display merge conflict branches
+         use MergeParents::*;
++        use Source::*;
++=======
++        use MergeConflictCommit::*;
++        use MergeParents::*;
++>>>>>>> b2b28c8... Display merge conflict branches
          use State::*;
  
          let mut handled_line = false;
@@@ -38,36 -38,28 +56,113 @@@
              return Ok(handled_line);
          }
  
++<<<<<<< HEAD
 +        match self.state.clone() {
 +            HunkHeader(Combined(merge_parents, InMergeConflict::No), _, _)
 +            | HunkMinus(Combined(merge_parents, InMergeConflict::No), _)
 +            | HunkZero(Combined(merge_parents, InMergeConflict::No))
 +            | HunkPlus(Combined(merge_parents, InMergeConflict::No), _) => {
 +                handled_line = self.enter_merge_conflict(&merge_parents)
 +            }
 +            MergeConflict(merge_parents, Ours) => {
 +                handled_line = self.enter_ancestral(&merge_parents)
 +                    || self.enter_theirs(&merge_parents)
 +                    || self.exit_merge_conflict(&merge_parents)?
 +                    || self.store_line(
 +                        Ours,
 +                        HunkPlus(Combined(merge_parents, InMergeConflict::Yes), None),
 +                    );
++||||||| parent of b2b28c8... Display merge conflict branches
++        // TODO: don't allocate on heap at this point
++        let prefix = self.line[..min(self.line.len(), 2)].to_string();
++        let diff_type = Combined(Prefix(prefix));
++
++        match self.state {
++            // The only transition into a merge conflict is HunkZero => MergeConflict(Ours)
++            // TODO: shouldn't this be HunkZero(Some(_))?
++            HunkZero(_) => {
++                if self.line.starts_with("++<<<<<<<") {
++                    self.state = MergeConflict(Ours);
++                    handled_line = true
++                }
 +            }
++            MergeConflict(Ours) => {
++                if self.line.starts_with("++|||||||") {
++                    self.state = MergeConflict(Ancestral);
++                } else if self.line.starts_with("++=======") {
++                    self.state = MergeConflict(Theirs);
++                } else if self.line.starts_with("++>>>>>>>") {
++                    self.paint_buffered_merge_conflict_lines(diff_type)?;
++                } else {
++                    let line = self.painter.prepare(&self.line, diff_type.n_parents());
++                    self.painter.merge_conflict_lines[Ours].push((line, HunkPlus(diff_type, None)));
++                }
++                handled_line = true
++=======
+         // TODO: don't allocate on heap at this point
+         let prefix = self.line[..min(self.line.len(), 2)].to_string();
+         let diff_type = Combined(Prefix(prefix));
+ 
+         match self.state {
+             // The only transition into a merge conflict is HunkZero => MergeConflict(Ours)
+             // TODO: shouldn't this be HunkZero(Some(_))?
+             HunkZero(_) => handled_line = self.enter_merge_conflict(),
+             MergeConflict(Ours) => {
+                 handled_line = self.enter_ancestral()
+                     || self.enter_theirs()
+                     || self.exit_merge_conflict(diff_type.clone())?
+                     || self.store_line(Ours, HunkPlus(diff_type, None));
++>>>>>>> b2b28c8... Display merge conflict branches
+             }
++<<<<<<< HEAD
 +            MergeConflict(merge_parents, Ancestral) => {
 +                handled_line = self.enter_theirs(&merge_parents)
 +                    || self.exit_merge_conflict(&merge_parents)?
 +                    || self.store_line(
 +                        Ancestral,
 +                        HunkMinus(Combined(merge_parents, InMergeConflict::Yes), None),
 +                    );
++||||||| parent of b2b28c8... Display merge conflict branches
++            MergeConflict(Ancestral) => {
++                if self.line.starts_with("++=======") {
++                    self.state = MergeConflict(Theirs);
++                } else if self.line.starts_with("++>>>>>>>") {
++                    self.paint_buffered_merge_conflict_lines(diff_type)?;
++                } else {
++                    let line = self.painter.prepare(&self.line, diff_type.n_parents());
++                    self.painter.merge_conflict_lines[Ancestral]
++                        .push((line, HunkMinus(diff_type, None)));
++                }
++                handled_line = true
++=======
+             MergeConflict(Ancestral) => {
+                 handled_line = self.enter_theirs()
+                     || self.exit_merge_conflict(diff_type.clone())?
+                     || self.store_line(Ancestral, HunkMinus(diff_type, None));
++>>>>>>> b2b28c8... Display merge conflict branches
              }
++<<<<<<< HEAD
 +            MergeConflict(merge_parents, Theirs) => {
 +                handled_line = self.exit_merge_conflict(&merge_parents)?
 +                    || self.store_line(
 +                        Theirs,
 +                        HunkPlus(Combined(merge_parents, InMergeConflict::Yes), None),
 +                    );
++||||||| parent of b2b28c8... Display merge conflict branches
++            MergeConflict(Theirs) => {
++                if self.line.starts_with("++>>>>>>>") {
++                    self.paint_buffered_merge_conflict_lines(diff_type)?;
++                } else {
++                    let line = self.painter.prepare(&self.line, diff_type.n_parents());
++                    self.painter.merge_conflict_lines[Theirs]
++                        .push((line, HunkPlus(diff_type, None)));
++                }
++                handled_line = true
++=======
+             MergeConflict(Theirs) => {
+                 handled_line = self.exit_merge_conflict(diff_type.clone())?
+                     || self.store_line(Theirs, HunkPlus(diff_type, None));
++>>>>>>> b2b28c8... Display merge conflict branches
              }
              _ => {}
          }
@@@ -75,75 -67,71 +170,150 @@@
          Ok(handled_line)
      }
  
++<<<<<<< HEAD
 +    fn enter_merge_conflict(&mut self, merge_parents: &MergeParents) -> bool {
 +        use State::*;
 +        if let Some(commit) = parse_merge_marker(&self.line, "++<<<<<<<") {
 +            self.state = MergeConflict(merge_parents.clone(), Ours);
 +            self.painter.merge_conflict_commit_names[Ours] = Some(commit.to_string());
 +            true
 +        } else {
 +            false
 +        }
 +    }
 +
 +    fn enter_ancestral(&mut self, merge_parents: &MergeParents) -> bool {
 +        use State::*;
 +        if let Some(commit) = parse_merge_marker(&self.line, "++|||||||") {
 +            self.state = MergeConflict(merge_parents.clone(), Ancestral);
 +            self.painter.merge_conflict_commit_names[Ancestral] = Some(commit.to_string());
 +            true
 +        } else {
 +            false
 +        }
 +    }
 +
 +    fn enter_theirs(&mut self, merge_parents: &MergeParents) -> bool {
 +        use State::*;
 +        if self.line.starts_with("++=======") {
 +            self.state = MergeConflict(merge_parents.clone(), Theirs);
 +            true
 +        } else {
 +            false
 +        }
 +    }
 +
 +    fn exit_merge_conflict(&mut self, merge_parents: &MergeParents) -> std::io::Result<bool> {
 +        if let Some(commit) = parse_merge_marker(&self.line, "++>>>>>>>") {
 +            self.painter.merge_conflict_commit_names[Theirs] = Some(commit.to_string());
 +            self.paint_buffered_merge_conflict_lines(merge_parents)?;
 +            Ok(true)
 +        } else {
 +            Ok(false)
 +        }
 +    }
 +
 +    fn store_line(&mut self, commit: MergeConflictCommit, state: State) -> bool {
 +        use State::*;
 +        if let HunkMinus(diff_type, _) | HunkZero(diff_type) | HunkPlus(diff_type, _) = &state {
 +            let line = self.painter.prepare(&self.line, diff_type.n_parents());
 +            self.painter.merge_conflict_lines[commit].push((line, state));
 +            true
 +        } else {
 +            delta_unreachable(&format!("Invalid state: {:?}", state))
 +        }
 +    }
 +
 +    fn paint_buffered_merge_conflict_lines(
 +        &mut self,
 +        merge_parents: &MergeParents,
 +    ) -> std::io::Result<()> {
 +        use DiffType::*;
 +        use State::*;
++||||||| parent of b2b28c8... Display merge conflict branches
++    fn paint_buffered_merge_conflict_lines(&mut self, diff_type: DiffType) -> std::io::Result<()> {
++=======
+     fn enter_merge_conflict(&mut self) -> bool {
+         use State::*;
+         if let Some(commit) = parse_merge_marker(&self.line, "++<<<<<<<") {
+             self.state = MergeConflict(Ours);
+             self.painter.merge_conflict_commit_names[Ours] = Some(commit.to_string());
+             true
+         } else {
+             false
+         }
+     }
+ 
+     fn enter_ancestral(&mut self) -> bool {
+         use State::*;
+         if let Some(commit) = parse_merge_marker(&self.line, "++|||||||") {
+             self.state = MergeConflict(Ancestral);
+             self.painter.merge_conflict_commit_names[Ancestral] = Some(commit.to_string());
+             true
+         } else {
+             false
+         }
+     }
+ 
+     fn enter_theirs(&mut self) -> bool {
+         use State::*;
+         if self.line.starts_with("++=======") {
+             self.state = MergeConflict(Theirs);
+             true
+         } else {
+             false
+         }
+     }
+ 
+     fn exit_merge_conflict(&mut self, diff_type: DiffType) -> std::io::Result<bool> {
+         if let Some(commit) = parse_merge_marker(&self.line, "++>>>>>>>") {
+             self.painter.merge_conflict_commit_names[Theirs] = Some(commit.to_string());
+             self.paint_buffered_merge_conflict_lines(diff_type)?;
+             Ok(true)
+         } else {
+             Ok(false)
+         }
+     }
+ 
+     fn store_line(&mut self, commit: MergeConflictCommit, state: State) -> bool {
+         use State::*;
+         if let HunkMinus(diff_type, _) | HunkZero(diff_type) | HunkPlus(diff_type, _) = &state {
+             let line = self.painter.prepare(&self.line, diff_type.n_parents());
+             self.painter.merge_conflict_lines[commit].push((line, state));
+             true
+         } else {
+             delta_unreachable(&format!("Invalid state: {:?}", state))
+         }
+     }
+ 
+     fn paint_buffered_merge_conflict_lines(&mut self, diff_type: DiffType) -> std::io::Result<()> {
++>>>>>>> b2b28c8... Display merge conflict branches
          self.painter.emit()?;
++<<<<<<< HEAD
 +
 +        write_merge_conflict_bar(
 +            &self.config.merge_conflict_begin_symbol,
 +            &mut self.painter,
 +            self.config,
 +        )?;
 +        for derived_commit_type in &[Ours, Theirs] {
 +            write_diff_header(derived_commit_type, &mut self.painter, self.config)?;
 +            self.painter.emit()?;
++||||||| parent of b2b28c8... Display merge conflict branches
++        let lines = &self.painter.merge_conflict_lines;
++        for derived_lines in &[&lines[Ours], &lines[Theirs]] {
++=======
+ 
+         write_merge_conflict_bar("▼", &mut self.painter, self.config)?;
+         for (derived_commit_type, decoration_style) in &[(Ours, "box"), (Theirs, "box")] {
+             write_subhunk_header(
+                 derived_commit_type,
+                 decoration_style,
+                 &mut self.painter,
+                 self.config,
+             )?;
+             self.painter.emit()?;
++>>>>>>> b2b28c8... Display merge conflict branches
              paint::paint_minus_and_plus_lines(
                  MinusPlus::new(
                      &self.painter.merge_conflict_lines[Ancestral],
@@@ -156,78 -144,94 +326,190 @@@
              );
              self.painter.emit()?;
          }
++<<<<<<< HEAD
 +        // write_merge_conflict_decoration("bold ol", &mut self.painter, self.config)?;
 +        write_merge_conflict_bar(
 +            &self.config.merge_conflict_end_symbol,
 +            &mut self.painter,
 +            self.config,
 +        )?;
++||||||| parent of b2b28c8... Display merge conflict branches
++=======
+         // write_merge_conflict_decoration("bold ol", &mut self.painter, self.config)?;
+         write_merge_conflict_bar("▲", &mut self.painter, self.config)?;
++>>>>>>> b2b28c8... Display merge conflict branches
          self.painter.merge_conflict_lines.clear();
 -        self.state = State::HunkZero(diff_type);
 +        self.state = HunkZero(Combined(merge_parents.clone(), InMergeConflict::No));
          Ok(())
      }
  }
  
++<<<<<<< HEAD
 +fn write_diff_header(
 +    derived_commit_type: &MergeConflictCommit,
 +    painter: &mut paint::Painter,
 +    config: &config::Config,
 +) -> std::io::Result<()> {
 +    let (mut draw_fn, pad, decoration_ansi_term_style) =
 +        draw::get_draw_function(config.merge_conflict_diff_header_style.decoration_style);
 +    let derived_commit_name = &painter.merge_conflict_commit_names[derived_commit_type];
 +    let text = if let Some(_ancestral_commit) = &painter.merge_conflict_commit_names[Ancestral] {
 +        format!(
 +            "ancestor {} {}{}",
 +            config.right_arrow,
 +            derived_commit_name.as_deref().unwrap_or("?"),
 +            if pad { " " } else { "" }
 +        )
 +    } else {
 +        derived_commit_name.as_deref().unwrap_or("?").to_string()
 +    };
 +    draw_fn(
 +        painter.writer,
 +        &text,
 +        &text,
 +        &config.decorations_width,
 +        config.merge_conflict_diff_header_style,
 +        decoration_ansi_term_style,
 +    )?;
 +    Ok(())
 +}
 +
 +fn write_merge_conflict_bar(
 +    s: &str,
 +    painter: &mut paint::Painter,
 +    config: &config::Config,
 +) -> std::io::Result<()> {
 +    if let cli::Width::Fixed(width) = config.decorations_width {
 +        writeln!(
 +            painter.writer,
 +            "{}",
 +            &s.graphemes(true).cycle().take(width).join("")
 +        )?;
 +    }
 +    Ok(())
 +}
 +
 +fn parse_merge_marker<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
 +    match line.strip_prefix(marker) {
 +        Some(suffix) => {
 +            let suffix = suffix.trim();
 +            if !suffix.is_empty() {
 +                Some(suffix)
 +            } else {
 +                None
 +            }
 +        }
 +        None => None,
 +    }
 +}
 +
 +pub use MergeConflictCommit::*;
 +
++impl<T> Index<MergeConflictCommit> for MergeConflictCommits<T> {
++    type Output = T;
++    fn index(&self, commit: MergeConflictCommit) -> &Self::Output {
++        match commit {
++            Ours => &self.ours,
++            Ancestral => &self.ancestral,
++            Theirs => &self.theirs,
++        }
++    }
++}
++||||||| parent of b2b28c8... Display merge conflict branches
++pub use Source::*;
++=======
+ fn write_subhunk_header(
+     derived_commit_type: &MergeConflictCommit,
+     decoration_style: &str,
+     painter: &mut paint::Painter,
+     config: &config::Config,
+ ) -> std::io::Result<()> {
+     let (mut draw_fn, pad, decoration_ansi_term_style) =
+         draw::get_draw_function(DecorationStyle::from_str(
+             decoration_style,
+             config.true_color,
+             config.git_config.as_ref(),
+         ));
+     let derived_commit_name = &painter.merge_conflict_commit_names[derived_commit_type];
+     let text = if let Some(_ancestral_commit) = &painter.merge_conflict_commit_names[Ancestral] {
+         format!(
+             "ancestor {} {}{}",
+             config.right_arrow,
+             derived_commit_name.as_deref().unwrap_or("?"),
+             if pad { " " } else { "" }
+         )
+     } else {
+         derived_commit_name.as_deref().unwrap_or("?").to_string()
+     };
+     draw_fn(
+         painter.writer,
+         &text,
+         &text,
+         &config.decorations_width,
+         config.hunk_header_style,
+         decoration_ansi_term_style,
+     )?;
+     Ok(())
+ }
++>>>>>>> b2b28c8... Display merge conflict branches
+ 
++<<<<<<< HEAD
++impl<T> Index<&MergeConflictCommit> for MergeConflictCommits<T> {
++    type Output = T;
++    fn index(&self, commit: &MergeConflictCommit) -> &Self::Output {
++        match commit {
++||||||| parent of b2b28c8... Display merge conflict branches
++impl Index<Source> for MergeConflictLines {
++    type Output = Vec<(String, State)>;
++    fn index(&self, source: Source) -> &Self::Output {
++        match source {
++=======
+ #[allow(unused)]
+ fn write_merge_conflict_line(
+     painter: &mut paint::Painter,
+     config: &config::Config,
+ ) -> std::io::Result<()> {
+     let (mut draw_fn, _pad, decoration_ansi_term_style) = draw::get_draw_function(
+         DecorationStyle::from_str("bold ol", config.true_color, config.git_config.as_ref()),
+     );
+     draw_fn(
+         painter.writer,
+         "",
+         "",
+         &config.decorations_width,
+         config.hunk_header_style,
+         decoration_ansi_term_style,
+     )?;
+     Ok(())
+ }
+ 
+ fn write_merge_conflict_bar(
+     s: &str,
+     painter: &mut paint::Painter,
+     config: &config::Config,
+ ) -> std::io::Result<()> {
+     if let cli::Width::Fixed(width) = config.decorations_width {
+         writeln!(painter.writer, "{}", s.repeat(width))?;
+     }
+     Ok(())
+ }
+ 
+ fn parse_merge_marker<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
+     match line.strip_prefix(marker) {
+         Some(suffix) => {
+             let suffix = suffix.trim();
+             if !suffix.is_empty() {
+                 Some(suffix)
+             } else {
+                 None
+             }
+         }
+         None => None,
+     }
+ }
+ 
+ pub use MergeConflictCommit::*;
+ 
  impl<T> Index<MergeConflictCommit> for MergeConflictCommits<T> {
      type Output = T;
      fn index(&self, commit: MergeConflictCommit) -> &Self::Output {
@@@ -243,6 -247,6 +525,7 @@@ impl<T> Index<&MergeConflictCommit> fo
      type Output = T;
      fn index(&self, commit: &MergeConflictCommit) -> &Self::Output {
          match commit {
++>>>>>>> b2b28c8... Display merge conflict branches
              Ours => &self.ours,
              Ancestral => &self.ancestral,
              Theirs => &self.theirs,
"#;

    const GIT_MERGE_CONFLICT_U0: &str = r#"\
diff --cc src/handlers/merge_conflict.rs
index 27d47c0,3a7e7b9..0000000
--- a/src/handlers/merge_conflict.rs
+++ b/src/handlers/merge_conflict.rs
@@@ -3,7 -4,4 +3,16 @@@ use std::ops::{Index, IndexMut}
++<<<<<<< HEAD
 +use itertools::Itertools;
 +use unicode_segmentation::UnicodeSegmentation;
 +
 +use super::draw;
 +use crate::cli;
 +use crate::config::{self, delta_unreachable};
 +use crate::delta::{DiffType, InMergeConflict, MergeParents, State, StateMachine};
++||||||| parent of b2b28c8... Display merge conflict branches
++use crate::delta::{DiffType, MergeParents, State, StateMachine};
++=======
+ use super::draw;
+ use crate::cli;
+ use crate::config::{self, delta_unreachable};
+ use crate::delta::{DiffType, MergeParents, State, StateMachine};
++>>>>>>> b2b28c8... Display merge conflict branches
@@@ -33,0 -32,0 +43,1 @@@ impl<'a> StateMachine<'a> 
++<<<<<<< HEAD
@@@ -34,0 -33,1 +45,7 @@@
++||||||| parent of b2b28c8... Display merge conflict branches
+         use MergeParents::*;
++        use Source::*;
++=======
++        use MergeConflictCommit::*;
++        use MergeParents::*;
++>>>>>>> b2b28c8... Display merge conflict branches
@@@ -41,23 -41,18 +59,84 @@@
++<<<<<<< HEAD
 +        match self.state.clone() {
 +            HunkHeader(Combined(merge_parents, InMergeConflict::No), _, _)
 +            | HunkMinus(Combined(merge_parents, InMergeConflict::No), _)
 +            | HunkZero(Combined(merge_parents, InMergeConflict::No))
 +            | HunkPlus(Combined(merge_parents, InMergeConflict::No), _) => {
 +                handled_line = self.enter_merge_conflict(&merge_parents)
 +            }
 +            MergeConflict(merge_parents, Ours) => {
 +                handled_line = self.enter_ancestral(&merge_parents)
 +                    || self.enter_theirs(&merge_parents)
 +                    || self.exit_merge_conflict(&merge_parents)?
 +                    || self.store_line(
 +                        Ours,
 +                        HunkPlus(Combined(merge_parents, InMergeConflict::Yes), None),
 +                    );
++||||||| parent of b2b28c8... Display merge conflict branches
++        // TODO: don't allocate on heap at this point
++        let prefix = self.line[..min(self.line.len(), 2)].to_string();
++        let diff_type = Combined(Prefix(prefix));
++
++        match self.state {
++            // The only transition into a merge conflict is HunkZero => MergeConflict(Ours)
++            // TODO: shouldn't this be HunkZero(Some(_))?
++            HunkZero(_) => {
++                if self.line.starts_with("++<<<<<<<") {
++                    self.state = MergeConflict(Ours);
++                    handled_line = true
++                }
 +            }
++            MergeConflict(Ours) => {
++                if self.line.starts_with("++|||||||") {
++                    self.state = MergeConflict(Ancestral);
++                } else if self.line.starts_with("++=======") {
++                    self.state = MergeConflict(Theirs);
++                } else if self.line.starts_with("++>>>>>>>") {
++                    self.paint_buffered_merge_conflict_lines(diff_type)?;
++                } else {
++                    let line = self.painter.prepare(&self.line, diff_type.n_parents());
++                    self.painter.merge_conflict_lines[Ours].push((line, HunkPlus(diff_type, None)));
++                }
++                handled_line = true
++=======
+         // TODO: don't allocate on heap at this point
+         let prefix = self.line[..min(self.line.len(), 2)].to_string();
+         let diff_type = Combined(Prefix(prefix));
+ 
+         match self.state {
+             // The only transition into a merge conflict is HunkZero => MergeConflict(Ours)
+             // TODO: shouldn't this be HunkZero(Some(_))?
+             HunkZero(_) => handled_line = self.enter_merge_conflict(),
+             MergeConflict(Ours) => {
+                 handled_line = self.enter_ancestral()
+                     || self.enter_theirs()
+                     || self.exit_merge_conflict(diff_type.clone())?
+                     || self.store_line(Ours, HunkPlus(diff_type, None));
++>>>>>>> b2b28c8... Display merge conflict branches
+             }
++<<<<<<< HEAD
 +            MergeConflict(merge_parents, Ancestral) => {
 +                handled_line = self.enter_theirs(&merge_parents)
 +                    || self.exit_merge_conflict(&merge_parents)?
 +                    || self.store_line(
 +                        Ancestral,
 +                        HunkMinus(Combined(merge_parents, InMergeConflict::Yes), None),
 +                    );
++||||||| parent of b2b28c8... Display merge conflict branches
++            MergeConflict(Ancestral) => {
++                if self.line.starts_with("++=======") {
++                    self.state = MergeConflict(Theirs);
++                } else if self.line.starts_with("++>>>>>>>") {
++                    self.paint_buffered_merge_conflict_lines(diff_type)?;
++                } else {
++                    let line = self.painter.prepare(&self.line, diff_type.n_parents());
++                    self.painter.merge_conflict_lines[Ancestral]
++                        .push((line, HunkMinus(diff_type, None)));
++                }
++                handled_line = true
++=======
+             MergeConflict(Ancestral) => {
+                 handled_line = self.enter_theirs()
+                     || self.exit_merge_conflict(diff_type.clone())?
+                     || self.store_line(Ancestral, HunkMinus(diff_type, None));
++>>>>>>> b2b28c8... Display merge conflict branches
@@@ -65,6 -60,3 +144,22 @@@
++<<<<<<< HEAD
 +            MergeConflict(merge_parents, Theirs) => {
 +                handled_line = self.exit_merge_conflict(&merge_parents)?
 +                    || self.store_line(
 +                        Theirs,
 +                        HunkPlus(Combined(merge_parents, InMergeConflict::Yes), None),
 +                    );
++||||||| parent of b2b28c8... Display merge conflict branches
++            MergeConflict(Theirs) => {
++                if self.line.starts_with("++>>>>>>>") {
++                    self.paint_buffered_merge_conflict_lines(diff_type)?;
++                } else {
++                    let line = self.painter.prepare(&self.line, diff_type.n_parents());
++                    self.painter.merge_conflict_lines[Theirs]
++                        .push((line, HunkPlus(diff_type, None)));
++                }
++                handled_line = true
++=======
+             MergeConflict(Theirs) => {
+                 handled_line = self.exit_merge_conflict(diff_type.clone())?
+                     || self.store_line(Theirs, HunkPlus(diff_type, None));
++>>>>>>> b2b28c8... Display merge conflict branches
@@@ -78,59 -70,54 +173,118 @@@
++<<<<<<< HEAD
 +    fn enter_merge_conflict(&mut self, merge_parents: &MergeParents) -> bool {
 +        use State::*;
 +        if let Some(commit) = parse_merge_marker(&self.line, "++<<<<<<<") {
 +            self.state = MergeConflict(merge_parents.clone(), Ours);
 +            self.painter.merge_conflict_commit_names[Ours] = Some(commit.to_string());
 +            true
 +        } else {
 +            false
 +        }
 +    }
 +
 +    fn enter_ancestral(&mut self, merge_parents: &MergeParents) -> bool {
 +        use State::*;
 +        if let Some(commit) = parse_merge_marker(&self.line, "++|||||||") {
 +            self.state = MergeConflict(merge_parents.clone(), Ancestral);
 +            self.painter.merge_conflict_commit_names[Ancestral] = Some(commit.to_string());
 +            true
 +        } else {
 +            false
 +        }
 +    }
 +
 +    fn enter_theirs(&mut self, merge_parents: &MergeParents) -> bool {
 +        use State::*;
 +        if self.line.starts_with("++=======") {
 +            self.state = MergeConflict(merge_parents.clone(), Theirs);
 +            true
 +        } else {
 +            false
 +        }
 +    }
 +
 +    fn exit_merge_conflict(&mut self, merge_parents: &MergeParents) -> std::io::Result<bool> {
 +        if let Some(commit) = parse_merge_marker(&self.line, "++>>>>>>>") {
 +            self.painter.merge_conflict_commit_names[Theirs] = Some(commit.to_string());
 +            self.paint_buffered_merge_conflict_lines(merge_parents)?;
 +            Ok(true)
 +        } else {
 +            Ok(false)
 +        }
 +    }
 +
 +    fn store_line(&mut self, commit: MergeConflictCommit, state: State) -> bool {
 +        use State::*;
 +        if let HunkMinus(diff_type, _) | HunkZero(diff_type) | HunkPlus(diff_type, _) = &state {
 +            let line = self.painter.prepare(&self.line, diff_type.n_parents());
 +            self.painter.merge_conflict_lines[commit].push((line, state));
 +            true
 +        } else {
 +            delta_unreachable(&format!("Invalid state: {:?}", state))
 +        }
 +    }
 +
 +    fn paint_buffered_merge_conflict_lines(
 +        &mut self,
 +        merge_parents: &MergeParents,
 +    ) -> std::io::Result<()> {
 +        use DiffType::*;
 +        use State::*;
++||||||| parent of b2b28c8... Display merge conflict branches
++    fn paint_buffered_merge_conflict_lines(&mut self, diff_type: DiffType) -> std::io::Result<()> {
++=======
+     fn enter_merge_conflict(&mut self) -> bool {
+         use State::*;
+         if let Some(commit) = parse_merge_marker(&self.line, "++<<<<<<<") {
+             self.state = MergeConflict(Ours);
+             self.painter.merge_conflict_commit_names[Ours] = Some(commit.to_string());
+             true
+         } else {
+             false
+         }
+     }
+ 
+     fn enter_ancestral(&mut self) -> bool {
+         use State::*;
+         if let Some(commit) = parse_merge_marker(&self.line, "++|||||||") {
+             self.state = MergeConflict(Ancestral);
+             self.painter.merge_conflict_commit_names[Ancestral] = Some(commit.to_string());
+             true
+         } else {
+             false
+         }
+     }
+ 
+     fn enter_theirs(&mut self) -> bool {
+         use State::*;
+         if self.line.starts_with("++=======") {
+             self.state = MergeConflict(Theirs);
+             true
+         } else {
+             false
+         }
+     }
+ 
+     fn exit_merge_conflict(&mut self, diff_type: DiffType) -> std::io::Result<bool> {
+         if let Some(commit) = parse_merge_marker(&self.line, "++>>>>>>>") {
+             self.painter.merge_conflict_commit_names[Theirs] = Some(commit.to_string());
+             self.paint_buffered_merge_conflict_lines(diff_type)?;
+             Ok(true)
+         } else {
+             Ok(false)
+         }
+     }
+ 
+     fn store_line(&mut self, commit: MergeConflictCommit, state: State) -> bool {
+         use State::*;
+         if let HunkMinus(diff_type, _) | HunkZero(diff_type) | HunkPlus(diff_type, _) = &state {
+             let line = self.painter.prepare(&self.line, diff_type.n_parents());
+             self.painter.merge_conflict_lines[commit].push((line, state));
+             true
+         } else {
+             delta_unreachable(&format!("Invalid state: {:?}", state))
+         }
+     }
+ 
+     fn paint_buffered_merge_conflict_lines(&mut self, diff_type: DiffType) -> std::io::Result<()> {
++>>>>>>> b2b28c8... Display merge conflict branches
@@@ -138,9 -125,10 +292,25 @@@
++<<<<<<< HEAD
 +
 +        write_merge_conflict_bar(
 +            &self.config.merge_conflict_begin_symbol,
 +            &mut self.painter,
 +            self.config,
 +        )?;
 +        for derived_commit_type in &[Ours, Theirs] {
 +            write_diff_header(derived_commit_type, &mut self.painter, self.config)?;
 +            self.painter.emit()?;
++||||||| parent of b2b28c8... Display merge conflict branches
++        let lines = &self.painter.merge_conflict_lines;
++        for derived_lines in &[&lines[Ours], &lines[Theirs]] {
++=======
+ 
+         write_merge_conflict_bar("▼", &mut self.painter, self.config)?;
+         for (derived_commit_type, decoration_style) in &[(Ours, "box"), (Theirs, "box")] {
+             write_subhunk_header(
+                 derived_commit_type,
+                 decoration_style,
+                 &mut self.painter,
+                 self.config,
+             )?;
+             self.painter.emit()?;
++>>>>>>> b2b28c8... Display merge conflict branches
@@@ -159,6 -147,2 +329,12 @@@
++<<<<<<< HEAD
 +        // write_merge_conflict_decoration("bold ol", &mut self.painter, self.config)?;
 +        write_merge_conflict_bar(
 +            &self.config.merge_conflict_end_symbol,
 +            &mut self.painter,
 +            self.config,
 +        )?;
++||||||| parent of b2b28c8... Display merge conflict branches
++=======
+         // write_merge_conflict_decoration("bold ol", &mut self.painter, self.config)?;
+         write_merge_conflict_bar("▲", &mut self.painter, self.config)?;
++>>>>>>> b2b28c8... Display merge conflict branches
@@@ -171,60 -155,80 +347,166 @@@
++<<<<<<< HEAD
 +fn write_diff_header(
 +    derived_commit_type: &MergeConflictCommit,
 +    painter: &mut paint::Painter,
 +    config: &config::Config,
 +) -> std::io::Result<()> {
 +    let (mut draw_fn, pad, decoration_ansi_term_style) =
 +        draw::get_draw_function(config.merge_conflict_diff_header_style.decoration_style);
 +    let derived_commit_name = &painter.merge_conflict_commit_names[derived_commit_type];
 +    let text = if let Some(_ancestral_commit) = &painter.merge_conflict_commit_names[Ancestral] {
 +        format!(
 +            "ancestor {} {}{}",
 +            config.right_arrow,
 +            derived_commit_name.as_deref().unwrap_or("?"),
 +            if pad { " " } else { "" }
 +        )
 +    } else {
 +        derived_commit_name.as_deref().unwrap_or("?").to_string()
 +    };
 +    draw_fn(
 +        painter.writer,
 +        &text,
 +        &text,
 +        &config.decorations_width,
 +        config.merge_conflict_diff_header_style,
 +        decoration_ansi_term_style,
 +    )?;
 +    Ok(())
 +}
 +
 +fn write_merge_conflict_bar(
 +    s: &str,
 +    painter: &mut paint::Painter,
 +    config: &config::Config,
 +) -> std::io::Result<()> {
 +    if let cli::Width::Fixed(width) = config.decorations_width {
 +        writeln!(
 +            painter.writer,
 +            "{}",
 +            &s.graphemes(true).cycle().take(width).join("")
 +        )?;
 +    }
 +    Ok(())
 +}
 +
 +fn parse_merge_marker<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
 +    match line.strip_prefix(marker) {
 +        Some(suffix) => {
 +            let suffix = suffix.trim();
 +            if !suffix.is_empty() {
 +                Some(suffix)
 +            } else {
 +                None
 +            }
 +        }
 +        None => None,
 +    }
 +}
 +
 +pub use MergeConflictCommit::*;
 +
++impl<T> Index<MergeConflictCommit> for MergeConflictCommits<T> {
++    type Output = T;
++    fn index(&self, commit: MergeConflictCommit) -> &Self::Output {
++        match commit {
++            Ours => &self.ours,
++            Ancestral => &self.ancestral,
++            Theirs => &self.theirs,
++        }
++    }
++}
++||||||| parent of b2b28c8... Display merge conflict branches
++pub use Source::*;
++=======
+ fn write_subhunk_header(
+     derived_commit_type: &MergeConflictCommit,
+     decoration_style: &str,
+     painter: &mut paint::Painter,
+     config: &config::Config,
+ ) -> std::io::Result<()> {
+     let (mut draw_fn, pad, decoration_ansi_term_style) =
+         draw::get_draw_function(DecorationStyle::from_str(
+             decoration_style,
+             config.true_color,
+             config.git_config.as_ref(),
+         ));
+     let derived_commit_name = &painter.merge_conflict_commit_names[derived_commit_type];
+     let text = if let Some(_ancestral_commit) = &painter.merge_conflict_commit_names[Ancestral] {
+         format!(
+             "ancestor {} {}{}",
+             config.right_arrow,
+             derived_commit_name.as_deref().unwrap_or("?"),
+             if pad { " " } else { "" }
+         )
+     } else {
+         derived_commit_name.as_deref().unwrap_or("?").to_string()
+     };
+     draw_fn(
+         painter.writer,
+         &text,
+         &text,
+         &config.decorations_width,
+         config.hunk_header_style,
+         decoration_ansi_term_style,
+     )?;
+     Ok(())
+ }
++>>>>>>> b2b28c8... Display merge conflict branches
+ 
++<<<<<<< HEAD
++impl<T> Index<&MergeConflictCommit> for MergeConflictCommits<T> {
++    type Output = T;
++    fn index(&self, commit: &MergeConflictCommit) -> &Self::Output {
++        match commit {
++||||||| parent of b2b28c8... Display merge conflict branches
++impl Index<Source> for MergeConflictLines {
++    type Output = Vec<(String, State)>;
++    fn index(&self, source: Source) -> &Self::Output {
++        match source {
++=======
+ #[allow(unused)]
+ fn write_merge_conflict_line(
+     painter: &mut paint::Painter,
+     config: &config::Config,
+ ) -> std::io::Result<()> {
+     let (mut draw_fn, _pad, decoration_ansi_term_style) = draw::get_draw_function(
+         DecorationStyle::from_str("bold ol", config.true_color, config.git_config.as_ref()),
+     );
+     draw_fn(
+         painter.writer,
+         "",
+         "",
+         &config.decorations_width,
+         config.hunk_header_style,
+         decoration_ansi_term_style,
+     )?;
+     Ok(())
+ }
+ 
+ fn write_merge_conflict_bar(
+     s: &str,
+     painter: &mut paint::Painter,
+     config: &config::Config,
+ ) -> std::io::Result<()> {
+     if let cli::Width::Fixed(width) = config.decorations_width {
+         writeln!(painter.writer, "{}", s.repeat(width))?;
+     }
+     Ok(())
+ }
+ 
+ fn parse_merge_marker<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
+     match line.strip_prefix(marker) {
+         Some(suffix) => {
+             let suffix = suffix.trim();
+             if !suffix.is_empty() {
+                 Some(suffix)
+             } else {
+                 None
+             }
+         }
+         None => None,
+     }
+ }
+ 
+ pub use MergeConflictCommit::*;
+ 
@@@ -246,0 -250,0 +528,1 @@@ impl<T> Index<&MergeConflictCommit> fo
++>>>>>>> b2b28c8... Display merge conflict branches
"#;
}
