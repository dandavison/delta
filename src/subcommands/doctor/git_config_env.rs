use crate::subcommands::doctor::shared::Diagnostic;
use crate::subcommands::doctor::shared::Health;
use std::collections::HashMap;
use std::env;

use Health::*;

pub struct GitConfigEnvVars {
    env_vars: HashMap<String, Option<String>>,
}

const UNSUPPORTED_GIT_CONFIG_ENV_VARS: [&str; 3] = [
    "GIT_CONFIG_GLOBAL",
    "GIT_CONFIG_SYSTEM",
    "GIT_CONFIG_NO_SYSTEM",
];

impl GitConfigEnvVars {
    pub fn probe() -> Self {
        GitConfigEnvVars {
            env_vars: UNSUPPORTED_GIT_CONFIG_ENV_VARS
                .map(|s| {
                    (
                        s.into(),
                        match env::var(s) {
                            Ok(v) => Some(v),
                            Err(_) => None,
                        },
                    )
                })
                .iter()
                .cloned()
                .collect(),
        }
    }
}

impl Diagnostic for GitConfigEnvVars {
    fn report(&self) -> String {
        let output_str = self
            .env_vars
            .iter()
            .map(|(k, v)| {
                format!(
                    "- {} = {}",
                    k,
                    match v {
                        Some(v) => v,
                        None => "",
                    }
                )
            })
            .collect::<Vec<String>>()
            .join("\n");
        "`GIT_CONFIG_*` environment variables: \n".to_string() + &output_str
    }

    fn diagnose(&self) -> Health {
        let set_vars = self
            .env_vars
            .iter()
            .filter(|&(_, v)| v.is_some())
            .map(|(k, v)| (k, v));

        let mut n_vals = 0;
        let diagnosis_prefix =
            "Unsupported `GIT_CONFIG_*` environment variables are set: \n".to_string();
        let remedy_prefix = "Unset `GIT_CONFIG_*` environment variables: \n".to_string();

        let output_str_items = &set_vars
            .map(|(k, v)| {
                format!(
                    "- {} = {}",
                    k,
                    match v {
                        Some(v) => {
                            n_vals += 1;
                            v
                        }
                        None => "",
                    }
                )
            })
            .collect::<Vec<String>>()
            .join("\n");

        match n_vals > 0 {
            true => Unhealthy(
                diagnosis_prefix + output_str_items,
                remedy_prefix + output_str_items,
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
