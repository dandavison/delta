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
    fn report(&self) -> String {
        match self.version {
            Some(version) => format!(
                "`less` version >= {} is required (your version: {})",
                MIN_LESS_VERSION, version
            ),
            None => "`less` version >= {} is required".to_string(),
        }
    }

    fn diagnose(&self) -> Health {
        match self.version {
            Some(n) if n < self.min_version => Unhealthy(
                "You may need a newer `less` version".to_string(),
                format!("Install `less` version >= {}", MIN_LESS_VERSION),
            ),
            None => Unhealthy(
                "Delta could not determine your `less` version".to_string(),
                format!("Install `less` version >= {}", MIN_LESS_VERSION),
            ),
            _ => Healthy,
        }
    }

    fn remedy(&self) -> Option<String> {
        match self.diagnose() {
            Unhealthy(_, remedy) => Some(remedy),
            _ => None,
        }
    }
}
