use crate::subcommands::doctor::shared::Diagnostic;
use crate::subcommands::doctor::shared::Health;
use crate::utils::bat::less;

use Health::*;

pub struct Less {
    min_version: usize,
    version: Option<usize>,
}

#[cfg(target_os = "windows")]
const MIN_LESS_VERSION: usize = 558;

#[cfg(not(target_os = "windows"))]
const MIN_LESS_VERSION: usize = 530;

impl Less {
    pub fn probe() -> Self {
        Less {
            version: less::retrieve_less_version(),
            min_version: MIN_LESS_VERSION,
        }
    }
}

impl Diagnostic for Less {
    fn report(&self) -> (String, bool) {
        match self.version {
            Some(n) if n < self.min_version => (
                format!(
                    "`less` version >= {} is required (your version: {})",
                    MIN_LESS_VERSION, n
                ),
                false,
            ),
            Some(n) if n >= self.min_version => (
                format!(
                    "Your less version ({}) is >= {} as required",
                    n, self.min_version
                ),
                true,
            ),
            _ => ("`less` version >= {} is required".to_string(), false),
        }
    }

    fn diagnose(&self) -> Health {
        let (description, is_healthy) = self.report();
        let remedy = format!("Install `less` version >= {}", MIN_LESS_VERSION);

        match is_healthy {
            false => Unhealthy(description, remedy),
            true => Healthy,
        }
    }

    fn remedy(&self) -> Option<String> {
        match self.diagnose() {
            Unhealthy(_, remedy) => Some(remedy),
            _ => None,
        }
    }
}
