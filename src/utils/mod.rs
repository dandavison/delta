#[cfg(not(tarpaulin_include))]
pub mod bat;
pub mod git;
pub mod helpwrap;
pub mod path;
pub mod process;
pub mod regex_replacement;
pub mod round_char_boundary;
pub mod syntect;
pub mod tabs;
pub mod workarounds;

// Use the most (even overly) strict ordering. Atomics are not used in hot loops so
// a one-size-fits-all approach which is never incorrect is okay.
pub const DELTA_ATOMIC_ORDERING: std::sync::atomic::Ordering = std::sync::atomic::Ordering::SeqCst;
