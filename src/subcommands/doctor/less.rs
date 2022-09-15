use crate::subcommands::doctor::shared::Diagnostic;
use crate::subcommands::doctor::shared::Health;
use crate::utils::bat::less;

use Health::*;

pub struct Less {
    min_version: usize,
    version: Option<usize>,
}

const MIN_LESS_VERSION: usize = 777777;

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
            Some(version) => format!("less version: {}", version),
            None => "Could not determine less version".to_string(),
        }
    }

    fn diagnose(&self) -> Health {
        match self.version {
            Some(n) if n < self.min_version => Unhealthy(
                "Your less version is too old".to_string(),
                "Install a newer version of less".to_string(),
            ),
            None => Unhealthy(
                "Could not determine less version".to_string(),
                "Is it installed?".into(),
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
