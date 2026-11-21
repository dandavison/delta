//! Difftastic is a syntactic diff tool.
//!
//! For usage instructions and advice on contributing, see [the
//! manual](http://difftastic.wilfred.me.uk/).
//!

// This tends to trigger on larger tuples of simple types, and naming
// them would probably be worse for readability.
#![allow(clippy::type_complexity)]
// == "" is often clearer when dealing with strings.
#![allow(clippy::comparison_to_empty)]
// It's common to have pairs foo_lhs and foo_rhs, leading to double
// the number of arguments and triggering this lint.
#![allow(clippy::too_many_arguments)]

mod context;
mod dijkstra;
mod files;
mod graph;
mod guess_language;
mod hunks;
mod inline;
mod line_parser;
mod lines;
mod myers_diff;
mod options;
mod positions;
mod side_by_side;
mod sliders;
mod style;
mod summary;
mod syntax;
mod tree_sitter_parser;
mod unchanged;

#[macro_use]
extern crate log;

use crate::hunks::{matched_pos_to_hunks, merge_adjacent};
use context::opposite_positions;
use files::read_files_or_die;
use guess_language::guess;
use log::info;
use mimalloc::MiMalloc;

/// The global allocator used by difftastic.
///
/// Diffing allocates a large amount of memory, and `MiMalloc` performs
/// better.
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use options::{should_use_color, DisplayMode, Mode};
use sliders::fix_all_sliders;
use std::{env, path::Path};
use style::BackgroundColor;
use summary::{DiffResult, FileContent};
use syntax::init_next_prev;
use typed_arena::Arena;
use walkdir::WalkDir;

use crate::{
    dijkstra::mark_syntax,
    files::{is_probably_binary, read_or_die},
    lines::MaxLine,
    syntax::init_all_info,
    tree_sitter_parser as tsp,
};

extern crate pretty_env_logger;

/// Terminate the process if we get SIGPIPE.
#[cfg(unix)]
fn reset_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn reset_sigpipe() {
    // Do nothing.
}

/// The entrypoint.
fn main() {
    pretty_env_logger::init();
    reset_sigpipe();

    match options::parse_args() {
        Mode::DumpTreeSitter { path } => {
            let path = Path::new(&path);
            let bytes = read_or_die(path);
            let src = String::from_utf8_lossy(&bytes).to_string();
            match guess(path, &src) {
                Some(lang) => {
                    let ts_lang = tsp::from_language(lang);
                    let tree = tsp::parse_to_tree(&src, &ts_lang);
                    tsp::print_tree(&src, &tree);
                }
                None => {
                    eprintln!("No tree-sitter parser for file: {:?}", path);
                }
            }
        }
        Mode::DumpSyntax { path } => {
            let path = Path::new(&path);
            let bytes = read_or_die(path);
            let src = String::from_utf8_lossy(&bytes).to_string();

            match guess(path, &src) {
                Some(lang) => {
                    let ts_lang = tsp::from_language(lang);
                    let arena = Arena::new();
                    let ast = tsp::parse(&arena, &src, &ts_lang);
                    init_all_info(&ast, &[]);
                    println!("{:#?}", ast);
                }
                None => {
                    eprintln!("No tree-sitter parser for file: {:?}", path);
                }
            }
        }
        Mode::Diff {
            node_limit,
            byte_limit,
            print_unchanged,
            missing_as_empty,
            display_mode,
            background_color,
            color_output,
            display_width,
            display_path,
            lhs_path,
            rhs_path,
            ..
        } => {
            let use_color = should_use_color(color_output);

            let lhs_path = Path::new(&lhs_path);
            let rhs_path = Path::new(&rhs_path);

            if lhs_path == rhs_path {
                eprintln!(
                    "warning: You've specified the same {} twice.\n",
                    if lhs_path.is_dir() {
                        "directory"
                    } else {
                        "file"
                    }
                );
            }

            if lhs_path.is_dir() && rhs_path.is_dir() {
                for diff_result in
                    diff_directories(lhs_path, rhs_path, missing_as_empty, node_limit, byte_limit)
                {
                    print_diff_result(
                        display_width,
                        use_color,
                        display_mode,
                        background_color,
                        print_unchanged,
                        &diff_result,
                    );
                }
            } else {
                let diff_result = diff_file(
                    &display_path,
                    lhs_path,
                    rhs_path,
                    missing_as_empty,
                    node_limit,
                    byte_limit,
                );
                print_diff_result(
                    display_width,
                    use_color,
                    display_mode,
                    background_color,
                    print_unchanged,
                    &diff_result,
                );
            }
        }
    };
}

/// Print a diff between two files.
fn diff_file(
    display_path: &str,
    lhs_path: &Path,
    rhs_path: &Path,
    missing_as_empty: bool,
    node_limit: u32,
    byte_limit: usize,
) -> DiffResult {
    let (lhs_bytes, rhs_bytes) = read_files_or_die(lhs_path, rhs_path, missing_as_empty);
    diff_file_content(display_path, &lhs_bytes, &rhs_bytes, node_limit, byte_limit)
}

fn diff_file_content(
    display_path: &str,
    lhs_bytes: &[u8],
    rhs_bytes: &[u8],
    node_limit: u32,
    byte_limit: usize,
) -> DiffResult {
    if is_probably_binary(lhs_bytes) || is_probably_binary(rhs_bytes) {
        return DiffResult {
            path: display_path.into(),
            language: None,
            lhs_src: FileContent::Binary(lhs_bytes.to_vec()),
            rhs_src: FileContent::Binary(rhs_bytes.to_vec()),
            lhs_positions: vec![],
            rhs_positions: vec![],
        };
    }

    // TODO: don't replace tab characters inside string literals.
    let mut lhs_src = String::from_utf8_lossy(lhs_bytes)
        .to_string()
        .replace('\t', "    ");
    let mut rhs_src = String::from_utf8_lossy(rhs_bytes)
        .to_string()
        .replace('\t', "    ");

    // Ignore the trailing newline, if present.
    // TODO: highlight if this has changes (#144).
    // TODO: factor out a string cleaning function.
    if lhs_src.ends_with('\n') {
        lhs_src.pop();
    }
    if rhs_src.ends_with('\n') {
        rhs_src.pop();
    }

    // TODO: take a Path directly instead.
    let path = Path::new(&display_path);

    // Take the larger of the two files when guessing the
    // language. This is useful when we've added or removed a whole
    // file.
    let guess_src = if lhs_src.len() > rhs_src.len() {
        &lhs_src
    } else {
        &rhs_src
    };
    let ts_lang = guess(path, guess_src).map(tsp::from_language);

    if lhs_bytes == rhs_bytes {
        // If the two files are completely identical, return early
        // rather than doing any more work.
        return DiffResult {
            path: display_path.into(),
            language: ts_lang.map(|l| l.name.into()),
            lhs_src: FileContent::Text("".into()),
            rhs_src: FileContent::Text("".into()),
            lhs_positions: vec![],
            rhs_positions: vec![],
        };
    }

    let (lang_name, lhs_positions, rhs_positions) = match ts_lang {
        _ if lhs_bytes.len() > byte_limit || rhs_bytes.len() > byte_limit => {
            let lhs_positions = line_parser::change_positions(&lhs_src, &rhs_src);
            let rhs_positions = line_parser::change_positions(&rhs_src, &lhs_src);
            (
                Some("Text (exceeded DFT_BYTE_LIMIT)".into()),
                lhs_positions,
                rhs_positions,
            )
        }
        Some(ts_lang) => {
            let arena = Arena::new();
            let lhs = tsp::parse(&arena, &lhs_src, &ts_lang);
            let rhs = tsp::parse(&arena, &rhs_src, &ts_lang);

            init_all_info(&lhs, &rhs);

            let possibly_changed = if env::var("DFT_DBG_KEEP_UNCHANGED").is_ok() {
                vec![(lhs.clone(), rhs.clone())]
            } else {
                unchanged::mark_unchanged(&lhs, &rhs)
            };

            let possibly_changed_max = max_num_nodes(&possibly_changed);
            if possibly_changed_max > node_limit {
                info!(
                    "Found {} nodes, exceeding the limit {}",
                    possibly_changed_max, node_limit
                );

                let lhs_positions = line_parser::change_positions(&lhs_src, &rhs_src);
                let rhs_positions = line_parser::change_positions(&rhs_src, &lhs_src);
                (
                    Some("Text (exceeded DFT_NODE_LIMIT)".into()),
                    lhs_positions,
                    rhs_positions,
                )
            } else {
                for (lhs_section_nodes, rhs_section_nodes) in possibly_changed {
                    init_next_prev(&lhs_section_nodes);
                    init_next_prev(&rhs_section_nodes);

                    mark_syntax(
                        lhs_section_nodes.get(0).copied(),
                        rhs_section_nodes.get(0).copied(),
                    );

                    fix_all_sliders(&lhs_section_nodes);
                    fix_all_sliders(&rhs_section_nodes);
                }

                let lhs_positions = syntax::change_positions(&lhs);
                let rhs_positions = syntax::change_positions(&rhs);
                (Some(ts_lang.name.into()), lhs_positions, rhs_positions)
            }
        }
        None => {
            let lhs_positions = line_parser::change_positions(&lhs_src, &rhs_src);
            let rhs_positions = line_parser::change_positions(&rhs_src, &lhs_src);
            (None, lhs_positions, rhs_positions)
        }
    };

    DiffResult {
        path: display_path.into(),
        language: lang_name,
        lhs_src: FileContent::Text(lhs_src),
        rhs_src: FileContent::Text(rhs_src),
        lhs_positions,
        rhs_positions,
    }
}

/// Given two directories that contain the files, compare them
/// pairwise. Returns an iterator, so we can print results
/// incrementally.
///
/// When more than one file is modified, the hg extdiff extension passes directory
/// paths with all the modified files.
fn diff_directories<'a>(
    lhs_dir: &'a Path,
    rhs_dir: &'a Path,
    missing_as_empty: bool,
    node_limit: u32,
    byte_limit: usize,
) -> impl Iterator<Item = DiffResult> + 'a {
    WalkDir::new(lhs_dir)
        .into_iter()
        .filter_map(Result::ok)
        .map(|entry| entry.into_path())
        .filter(|lhs_path| !lhs_path.is_dir())
        .map(move |lhs_path| {
            info!("LHS path is {:?} inside {:?}", lhs_path, lhs_dir);

            let rel_path = lhs_path.strip_prefix(lhs_dir).unwrap();
            let rhs_path = Path::new(rhs_dir).join(rel_path);

            diff_file(
                &rel_path.to_string_lossy(),
                &lhs_path,
                &rhs_path,
                missing_as_empty,
                node_limit,
                byte_limit,
            )
        })
}

// TODO: factor out a DiffOptions struct.
fn print_diff_result(
    display_width: usize,
    use_color: bool,
    display_mode: DisplayMode,
    background: BackgroundColor,
    print_unchanged: bool,
    summary: &DiffResult,
) {
    match (&summary.lhs_src, &summary.rhs_src) {
        (FileContent::Text(lhs_src), FileContent::Text(rhs_src)) => {
            let opposite_to_lhs = opposite_positions(&summary.lhs_positions);
            let opposite_to_rhs = opposite_positions(&summary.rhs_positions);

            let hunks = matched_pos_to_hunks(&summary.lhs_positions, &summary.rhs_positions);
            let hunks = merge_adjacent(
                &hunks,
                &opposite_to_lhs,
                &opposite_to_rhs,
                lhs_src.max_line(),
                rhs_src.max_line(),
            );

            let lang_name = summary.language.clone().unwrap_or_else(|| "Text".into());
            if hunks.is_empty() {
                if print_unchanged {
                    println!(
                        "{}",
                        style::header(&summary.path, 1, 1, &lang_name, use_color, background)
                    );
                    if lang_name == "Text" || summary.lhs_src == summary.rhs_src {
                        // TODO: there are other Text names now, so
                        // they will hit the second case incorrectly.
                        println!("No changes.\n");
                    } else {
                        println!("No syntactic changes.\n");
                    }
                }
                return;
            }

            match display_mode {
                DisplayMode::Inline => {
                    inline::print(
                        lhs_src,
                        rhs_src,
                        &summary.lhs_positions,
                        &summary.rhs_positions,
                        &hunks,
                        &summary.path,
                        &lang_name,
                        use_color,
                        background,
                    );
                }
                DisplayMode::SideBySide | DisplayMode::SideBySideShowBoth => {
                    side_by_side::print(
                        &hunks,
                        display_width,
                        use_color,
                        display_mode,
                        background,
                        &summary.path,
                        &lang_name,
                        lhs_src,
                        rhs_src,
                        &summary.lhs_positions,
                        &summary.rhs_positions,
                    );
                }
            }
        }
        (FileContent::Binary(lhs_bytes), FileContent::Binary(rhs_bytes)) => {
            let changed = lhs_bytes != rhs_bytes;
            if print_unchanged || changed {
                println!(
                    "{}",
                    style::header(&summary.path, 1, 1, "binary", use_color, background)
                );
                if changed {
                    println!("Binary contents changed.");
                } else {
                    println!("No changes.");
                }
            }
        }
        (_, FileContent::Binary(_)) | (FileContent::Binary(_), _) => {
            // We're diffing a binary file against a text file.
            println!(
                "{}",
                style::header(&summary.path, 1, 1, "binary", use_color, background)
            );
            println!("Binary contents changed.");
        }
    }
}

/// What is the total number of nodes in `roots`?
fn num_nodes(roots: &[&syntax::Syntax]) -> u32 {
    roots
        .iter()
        .map(|n| {
            1 + match n {
                syntax::Syntax::List {
                    num_descendants, ..
                } => *num_descendants,
                syntax::Syntax::Atom { .. } => 0,
            }
        })
        .sum()
}

fn max_num_nodes(roots_vec: &[(Vec<&syntax::Syntax>, Vec<&syntax::Syntax>)]) -> u32 {
    roots_vec
        .iter()
        .map(|(lhs, rhs)| num_nodes(lhs) + num_nodes(rhs))
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::{DEFAULT_BYTE_LIMIT, DEFAULT_NODE_LIMIT};

    #[test]
    fn test_diff_identical_content() {
        let s = "foo";
        let res = diff_file_content(
            "foo.el",
            s.as_bytes(),
            s.as_bytes(),
            DEFAULT_NODE_LIMIT,
            DEFAULT_BYTE_LIMIT,
        );

        assert_eq!(res.lhs_positions, vec![]);
        assert_eq!(res.rhs_positions, vec![]);
    }
}
