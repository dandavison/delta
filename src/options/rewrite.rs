/// This module applies rewrite rules to the command line options, in order to
/// 1. Express deprecated usages in the new non-deprecated form
/// 2. Implement options such as --raw which are defined to be equivalent to some set of
///    other options.
use crate::cli;

pub fn apply_rewrite_rules(opt: &mut cli::Opt) {
    rewrite_options_to_implement_deprecated_commit_and_file_style_box_option(opt);
}

/// For backwards-compatibility, --{commit,file}-style box means --element-decoration-style 'box ul'.
fn rewrite_options_to_implement_deprecated_commit_and_file_style_box_option(opt: &mut cli::Opt) {
    if &opt.commit_style == "box" {
        opt.commit_decoration_style = format!("box ul {}", opt.commit_decoration_style);
        opt.commit_style.clear();
    }
    if &opt.file_style == "box" {
        opt.file_decoration_style = format!("box ul {}", opt.file_decoration_style);
        opt.file_style.clear();
    }
}
