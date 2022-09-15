use crate::subcommands::doctor::shared::Diagnostic;
use crate::subcommands::doctor::shared::Health;
use std::collections::HashMap;
use std::env;

use Health::*;

pub struct PagerEnvVars {
    env_vars: HashMap<String, String>,
}

const UNSUPPORTED_PAGER_SUFFIX: &str = "PAGER";

impl PagerEnvVars {
    pub fn probe() -> Self {
        PagerEnvVars {
            env_vars: env::vars()
                .filter(|(k, _)| k.ends_with(UNSUPPORTED_PAGER_SUFFIX))
                .collect(),
        }
    }
}

impl Diagnostic for PagerEnvVars {
    fn report(&self) -> String {
        let output_str = self
            .env_vars
            .iter()
            .map(|(k, v)| format!("- {} = {}", k, v))
            .collect::<Vec<String>>()
            .join("\n");
        "`*PAGER` environment variables: \n".to_string() + &output_str
    }

    fn diagnose(&self) -> Health {
        let diagnosis_prefix = "Unsupported `*PAGER` environment variables are set: \n".to_string();
        let remedy_prefix = "Unset `*PAGER` environment variables: \n".to_string();

        let output_str_items = self
            .env_vars
            .iter()
            .map(|(k, v)| format!("- {} = {}", k, v))
            .collect::<Vec<String>>()
            .join("\n");

        match self.env_vars.keys().len() > 0 {
            true => Unhealthy(
                diagnosis_prefix + &*output_str_items,
                remedy_prefix + &*output_str_items,
            ),
            false => Healthy,
        }
    }

    fn remedy(&self) -> Option<String> {
        match self.diagnose() {
            Unhealthy(_, remedy) => Some(remedy),
            _ => None,
        }
    }
}
