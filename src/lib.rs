extern crate bitflags;

#[macro_use]
extern crate error_chain;

pub mod align;
pub mod ansi;
pub mod cli;
pub mod color;
pub mod colors;
pub mod config;
pub mod delta;
pub mod edits;
pub mod env;
pub mod features;
pub mod format;
pub mod git_config;
pub mod handlers;
pub mod minusplus;
pub mod options;
pub mod paint;
pub mod parse_style;
pub mod parse_styles;
pub mod style;
pub mod utils;
pub mod wrapping;

pub mod subcommands;

pub mod tests;

pub fn fatal<T>(errmsg: T) -> !
where
    T: AsRef<str> + std::fmt::Display,
{
    #[cfg(not(test))]
    {
        use std::process;
        eprintln!("{}", errmsg);
        // As in Config::error_exit_code: use 2 for error
        // because diff uses 0 and 1 for non-error.
        process::exit(2);
    }
    #[cfg(test)]
    panic!("{}\n", errmsg);
}

pub mod errors {
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
            SyntectError(::syntect::LoadingError);
            ParseIntError(::std::num::ParseIntError);
        }
    }
}
