use crate::subcommands::doctor::shared::Diagnostic;
use crate::subcommands::doctor::shared::Health;
use std::env;

use Health::*;

pub struct GitConfigEnvVars {
    env_vars: Vec<std::string::String>,
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
                .iter()
                .filter_map(|s| get_env_kv(s))
                .map(|(k, v)| format!(" - {} = {}", k, v))
                .collect::<Vec<String>>(),
        }
    }

    fn has_unsupported_env_vars(&self) -> bool {
        !self.env_vars.is_empty()
    }
}

impl Diagnostic for GitConfigEnvVars {
    fn report(&self) -> (String, bool) {
        let vars = &self.env_vars;
        if self.has_unsupported_env_vars() {
            let vars_str = vars.join("\n");
            (
                "`GIT_CONFIG_*` environment variables are not supported, but were found in your environment:\n".to_owned() + &vars_str,
                false
            )
        } else {
            (
                "No `GIT_CONFIG_*` environment variables are set.".to_owned(),
                true,
            )
        }
    }

    fn diagnose(&self) -> Health {
        let diagnosis_is_healthy = self.report();
        let remedy = "Unset `GIT_CONFIG_*` environment variables.".to_string();

        match diagnosis_is_healthy.1 {
            true => Unhealthy(diagnosis_is_healthy.0, remedy),
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

fn get_env_kv(k: &str) -> Option<(String, String)> {
    match env::var(k) {
        Ok(v) => Some((k.to_owned(), v)),
        Err(_) => None,
    }
}
