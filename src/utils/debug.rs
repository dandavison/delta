use crate::config::Config;
use crate::env::{DeltaEnv, DELTA_DEBUG_LOGFILE, DELTA_DEBUG_LOGFILE_MAX_SIZE};
use crate::fatal;
use crate::utils::DELTA_ATOMIC_ORDERING;

use console::Term;

use std::ffi::OsString;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::sync::atomic::AtomicBool;

const RUST_BACKTRACE: &str = "RUST_BACKTRACE";

// Use a global because the config where this might be stored could
// itself panic while it is being built.
static USING_DELTA_DEBUG_LOGFILE: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
pub struct PrintNoticeOnPanic;

impl PrintNoticeOnPanic {
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(not(test))]
impl Drop for PrintNoticeOnPanic {
    fn drop(&mut self) {
        // Nothing elaborate with std::panic::set_hook or std::panic::catch_unwind to also get the backtrace,
        // only set RUST_BACKTRACE when recording
        if std::thread::panicking() {
            if USING_DELTA_DEBUG_LOGFILE.load(DELTA_ATOMIC_ORDERING) {
                if let Some(logfile) = std::env::var_os(DELTA_DEBUG_LOGFILE) {
                    eprintln!("  Wrote {logfile:?} (if you want to share it, ensure no sensitive information is contained). You may also want to include the stack backtrace.");
                }
            } else {
                // Setting GIT_PAGER however does not override interactive.diffFilter!
                eprintln!("\n\
            delta panicked and crashed, sorry :(\n\
            To quickly repeat the previous command without delta, run 'export GIT_PAGER=less' first. If you want\n\
            to report the crash and it is easy to repeat, run 'export DELTA_DEBUG_LOGFILE=crash.log', then delta again,\n\
            then submit the logfile to github at <https://github.com/dandavison/delta/issues>. Thank you!\n\n\
            !! Make sure there is NO sensitive information in the log file, it will contain the entire diff !!\n\
            ")
            }
        }
    }
}

#[derive(Debug)]
struct DeltaDetailInternal {
    file: File,
    bytes_written: u64,
    max_log_size: u64,
    truncate_back_to: u64,
}

pub struct RecordDeltaCall(Option<Box<DeltaDetailInternal>>);

impl RecordDeltaCall {
    pub fn new(config: &Config) -> Self {
        fn make(logfile: &OsString, config: &Config) -> Result<Box<DeltaDetailInternal>, String> {
            let mut file = File::create(logfile).map_err(|e| e.to_string())?;

            let mut write = |s: String| file.write_all(s.as_bytes()).map_err(|e| e.to_string());

            write(
                "<details>\n\
                <summary>Input which caused delta to crash</summary>\n\n```\n"
                    .into(),
            )?;

            if let Some(cfg) = config.git_config.as_ref() {
                write("git config values:\n".into())?;
                cfg.for_each(".*", |entry, value: Option<&str>| {
                    if !(entry.starts_with("user.")
                        || entry.starts_with("remote.")
                        || entry.starts_with("branch.")
                        || entry.starts_with("gui."))
                    {
                        let _ = write(format!("{} = {:?}\n", entry, value.unwrap_or("")));
                    }
                })
            } else {
                write("(NO git config)\n".into())?;
            };

            write(format!(
                "command line: {:?}\n",
                std::env::args_os().collect::<Vec<_>>()
            ))?;
            let mut delta_env = DeltaEnv::init();
            // redact, not interesting:
            delta_env.current_dir = None;
            delta_env.hostname = None;
            write(format!("DeltaEnv: {:?}\n", delta_env))?;

            let term = Term::stdout();

            write(format!(
                "TERM: {:?}, is_term: {}, size: {:?}\n",
                std::env::var_os("TERM"),
                term.is_term(),
                term.size()
            ))?;

            write(
                "raw crash input to delta (usually something git diff etc. generated):\n".into(),
            )?;
            write("================================\n".into())?;

            file.flush().map_err(|e| e.to_string())?;
            let truncate_back_to = file.stream_position().map_err(|e| e.to_string())?;

            let max_log_size = std::env::var_os(DELTA_DEBUG_LOGFILE_MAX_SIZE)
                .map(|v| {
                    v.to_string_lossy().parse::<u64>().unwrap_or_else(|_| {
                        fatal(format!(
                            "Invalid env var value in {} (got {:?}, expected integer)",
                            DELTA_DEBUG_LOGFILE_MAX_SIZE, v
                        ));
                    })
                })
                .unwrap_or(512 * 1024);

            if std::env::var_os(RUST_BACKTRACE).is_none() {
                // SAFETY:
                // a) we only get here when `DELTA_DEBUG_LOGFILE` is set, which means a user is expecting a crash anyhow
                // b) the requirement is "no other threads concurrently writing or reading(!) [env vars],
                // other than the ones in this [std::env] module.", the rust backtrace handler should use std::env.
                unsafe {
                    std::env::set_var(RUST_BACKTRACE, "1");
                }
            }

            USING_DELTA_DEBUG_LOGFILE.store(true, DELTA_ATOMIC_ORDERING);

            Ok(Box::new(DeltaDetailInternal {
                file,
                bytes_written: 0,
                max_log_size,
                truncate_back_to,
            }))
        }

        if let Some(logfile) = std::env::var_os(DELTA_DEBUG_LOGFILE) {
            Self(
                make(&logfile, config)
                    .map_err(|e| {
                        eprintln!(
                            "\nnotice: failed to write {logfile:?} given by {DELTA_DEBUG_LOGFILE}: {e}"
                        );
                    })
                    .ok(),
            )
        } else {
            Self(None)
        }
    }

    #[inline]
    pub fn write(&mut self, line: &[u8]) {
        if self.0.is_some() {
            self._write(line);
        }
    }

    fn _write(&mut self, line: &[u8]) {
        let internal = self.0.as_mut().unwrap();
        if internal.bytes_written > internal.max_log_size {
            let _ = internal.file.flush();
            let _ = internal
                .file
                .seek(SeekFrom::Start(internal.truncate_back_to));
            let _ = internal.file.set_len(internal.truncate_back_to);
            let _ = internal.file.write_all(
                format!(
                    "(truncated [max log size is {},\
                     set {DELTA_DEBUG_LOGFILE_MAX_SIZE} to override])\n",
                    internal.max_log_size
                )
                .as_bytes(),
            );
            internal.bytes_written = 0;
        }
        let _ = internal.file.write_all(line);
        let _ = internal.file.write_all(b"\n");
        internal.bytes_written += line.len() as u64 + 1;
        let _ = internal.file.flush();
    }
}

impl Drop for RecordDeltaCall {
    fn drop(&mut self) {
        if let Some(ref mut internal) = self.0 {
            let _ = internal.file.write_all(b"\n```\n</details>\n");
        }
    }
}
