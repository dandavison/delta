//! Apply conflict markers to obtain the original file contents.
//!
//! https://git-scm.com/docs/git-merge#Documentation/git-merge.txt-mergeconflictStyle

use ConflictState::*;

#[derive(Debug, Clone, Copy)]
enum ConflictState {
    NoConflict,
    Left,
    Base,
    Right,
}

pub const START_LHS_MARKER: &str = "<<<<<<<";
const START_BASE_MARKER: &str = "|||||||";
const START_RHS_MARKER: &str = "=======";
const END_RHS_MARKER: &str = ">>>>>>>";

pub struct ConflictFiles {
    pub lhs_name: Option<String>,
    pub lhs_content: String,
    pub rhs_name: Option<String>,
    pub rhs_content: String,
    pub num_conflicts: usize,
}

/// Convert a string with conflict markers into the two conflicting
/// file contents.
pub fn apply_conflict_markers(s: &str) -> Result<ConflictFiles, String> {
    let mut lhs_name = String::new();
    let mut rhs_name = String::new();

    let mut lhs_content = String::with_capacity(s.len());
    let mut rhs_content = String::with_capacity(s.len());
    let mut num_conflicts = 0;

    let mut state = NoConflict;
    let mut conflict_start_line = None;
    for (i, line) in s.split_inclusive('\n').enumerate() {
        if let Some(hunk_lhs_name) = line.strip_prefix(START_LHS_MARKER) {
            state = Left;
            num_conflicts += 1;
            conflict_start_line = Some(i);

            let hunk_lhs_name = hunk_lhs_name.trim();
            if hunk_lhs_name.len() > lhs_name.len() {
                lhs_name = hunk_lhs_name.to_owned();
            }

            continue;
        }
        if line.starts_with(START_BASE_MARKER) {
            state = Base;
            continue;
        }
        if line.starts_with(START_RHS_MARKER) {
            state = Right;
            continue;
        }
        if let Some(hunk_rhs_name) = line.strip_prefix(END_RHS_MARKER) {
            state = NoConflict;

            let hunk_rhs_name = hunk_rhs_name.trim();
            if hunk_rhs_name.len() > rhs_name.len() {
                rhs_name = hunk_rhs_name.to_owned();
            }
            continue;
        }

        match state {
            NoConflict => {
                lhs_content.push_str(line);
                rhs_content.push_str(line);
            }
            Left => {
                lhs_content.push_str(line);
            }
            Right => {
                rhs_content.push_str(line);
            }
            Base => {}
        }
    }

    if matches!(state, NoConflict) {
        Ok(ConflictFiles {
            lhs_name,
            lhs_content,
            rhs_name,
            rhs_content,
            num_conflicts,
        })
    } else {
        let message = match conflict_start_line {
            Some(line_i) => format!(
                "Could not parse conflict markers, line {} has no matching {}.",
                line_i, END_RHS_MARKER
            ),
            None => "Could not parse conflict markers.".to_owned(),
        };
        Err(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_base() {
        // Deliberately avoid a multiline string literal to avoid
        // confusing text editors when we open this file.
        let s = "before\n<<<<<<< Temporary merge branch 1\nnew in left\n||||||| merged common ancestors\noriginal\n=======\nnew in right\n>>>>>>> Temporary merge branch 2\nafter";

        let conflict_files = apply_conflict_markers(s).unwrap();
        assert_eq!(conflict_files.lhs_content, "before\nnew in left\nafter");
        assert_eq!(conflict_files.rhs_content, "before\nnew in right\nafter");

        assert_eq!(conflict_files.lhs_name, "Temporary merge branch 1");
        assert_eq!(conflict_files.rhs_name, "Temporary merge branch 2");
    }

    #[test]
    fn test_without_base() {
        // Deliberately avoid a multiline string literal to avoid
        // confusing text editors when we open this file.
        let s = "before\n<<<<<<< Temporary merge branch 1\nnew in left\n=======\nnew in right\n>>>>>>> Temporary merge branch 2\nafter";

        let conflict_files = apply_conflict_markers(s).unwrap();
        assert_eq!(conflict_files.lhs_content, "before\nnew in left\nafter");
        assert_eq!(conflict_files.rhs_content, "before\nnew in right\nafter");

        assert_eq!(conflict_files.lhs_name, "Temporary merge branch 1");
        assert_eq!(conflict_files.rhs_name, "Temporary merge branch 2");
    }
}
