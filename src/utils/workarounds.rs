// env var which should disable workarounds
const NO_WORKAROUNDS: &str = "DELTA_NO_WORKAROUNDS";

// Work around a bug in the 'console' crate (inherited from 'terminal-size', #25): On Windows
// it can not determine the width of an MSYS2 / MINGW64 terminal (e.g. from Git-Bash) correctly.
// Instead use the usually included stty util from the MSYS2 distribution.
#[cfg(target_os = "windows")]
pub fn windows_msys2_width_fix(height_width: (u16, u16), term_stdout: &console::Term) -> usize {
    fn guess_real_width(current_width: u16, term_stdout: &console::Term) -> Option<u16> {
        use std::process::{Command, Stdio};

        let term_var = std::env::var("TERM").ok()?;
        // More checks before actually calling stty.
        if term_var.starts_with("xterm")
            && term_stdout.is_term()
            && term_stdout.features().is_msys_tty()
        {
            if std::env::var(NO_WORKAROUNDS).is_ok() {
                return Some(current_width);
            }

            // stderr/2 is passed to the Command below.
            let pseudo_term = "/dev/fd/2";

            // Read width via stty helper program (e.g. "C:\Program Files\Git\usr\bin\stty.exe")
            // which gets both the MSYS2 and cmd.exe width right.
            let result = Command::new("stty")
                .stderr(Stdio::inherit())
                .arg("-F")
                .arg(pseudo_term)
                .arg("size")
                .output()
                .ok()?;

            if result.status.success() {
                let size = std::str::from_utf8(&result.stdout).ok()?;
                let mut it = size.split_whitespace();
                let _height = it.next()?;
                return it.next().map(|width| width.parse().ok())?;
            }
        }
        None
    }

    // Calling an external binary is slow, so make sure this is actually necessary.
    // The fallback values of 25 lines by 80 columns (sometimes zero indexed) are a good
    // indicator.
    let (height, width) = height_width;
    match (height, width) {
        (24..=25, 79..=80) => guess_real_width(width, term_stdout).unwrap_or(width),
        _ => width,
    }
    .into()
}

#[cfg(not(target_os = "windows"))]
pub fn windows_msys2_width_fix(height_width: (u16, u16), _: &console::Term) -> usize {
    let _ = NO_WORKAROUNDS;
    height_width.1.into()
}
