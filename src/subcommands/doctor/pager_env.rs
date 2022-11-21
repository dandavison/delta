use crate::subcommands::doctor::shared::Diagnostic;
use crate::subcommands::doctor::shared::Health;
use std::collections::HashMap;
use std::env;

use Health::*;

pub struct PagerEnvVars {
    env_vars: HashMap<String, String>,
}

const PAGER_ENV_VARS: [&str; 4] = ["PAGER", "DELTA_PAGER", "BAT_PAGER", "GIT_PAGER"];

impl PagerEnvVars {
    pub fn probe() -> Self {
        PagerEnvVars {
            env_vars: env::vars()
                .filter_map(|(k, v)| get_env_kv(k, v))
                .into_iter()
                .collect(),
        }
    }

    fn selected_pager_is_less(&self) -> Option<bool> {
        match self.env_vars.get("DELTA_PAGER") {
            Some(_v) => Some(val_contains_less(_v)),
            None => match self.env_vars.get("BAT_PAGER") {
                Some(_v) => Some(val_contains_less(_v)),
                None => match self.env_vars.get("PAGER") {
                    Some(_v) => Some(val_contains_less(_v)),
                    None => None,
                },
            },
        }
    }
}

impl Diagnostic for PagerEnvVars {
    fn report(&self) -> String {
        let vars = &self.env_vars;
        match self.selected_pager_is_less() {
            Some(v) => match v {
                true => "Your selected pager is `less`".to_owned(),
                false => {
                    let vars_str = vars
                        .iter()
                        .map(|(k, v)| format!(" - {} = {}", k, v))
                        .collect::<Vec<String>>()
                        .join("\n");
                    return "The pager specified by your *PAGER environment variables is not `less`. You have:\n".to_owned() + &vars_str;
                }
            },
            None => "Your selected pager is the system `less`".to_owned(),
        }
    }

    fn diagnose(&self) -> Health {
        let diagnosis = self.report();
        let remedy = "Set `DELTA_PAGER` to your preferred version of `less`, or unset it to use the system `less`.".to_string();

        match self.selected_pager_is_less() {
            Some(v) => match v {
                true => Healthy,
                false => Unhealthy(diagnosis, remedy),
            },
            None => Healthy,
        }
    }

    fn remedy(&self) -> Option<String> {
        match self.diagnose() {
            Unhealthy(_, remedy) => Some(remedy),
            _ => None,
        }
    }
}

fn get_env_kv(k: String, v: String) -> Option<(String, String)> {
    match PAGER_ENV_VARS.contains(&k.as_str()) {
        true => Some((k, v)),
        false => None,
    }
}

fn val_contains_less(val: &str) -> bool {
    return val.to_ascii_lowercase().contains("less");
}
