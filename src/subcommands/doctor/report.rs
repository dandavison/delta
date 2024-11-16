use crate::subcommands::doctor::git_config_env::GitConfigEnvVars;
use crate::subcommands::doctor::less::Less;
use crate::subcommands::doctor::pager_env::PagerEnvVars;
use crate::subcommands::doctor::shared::Diagnostic;
use tabled::{settings::Style, Table, Tabled};

#[cfg(not(tarpaulin_include))]
pub fn print_doctor_report() -> std::io::Result<()> {
    let diagnostics: Vec<Box<dyn Diagnostic>> = vec![
        Box::new(Less::probe()),
        Box::new(GitConfigEnvVars::probe()),
        Box::new(PagerEnvVars::probe()),
    ];

    let reports = diagnostics
        .into_iter()
        .map(|d| Report::from_diagnostic(d))
        .collect();

    print_table(reports);
    Ok(())
}

#[derive(Tabled)]
struct Report {
    result: String,
    description: String,
    remedy: String,
}

impl Report {
    pub fn from_diagnostic(diagnostic: Box<dyn Diagnostic>) -> Self {
        let remedy = diagnostic.remedy();
        match remedy {
            Some(r) => Report {
                result: "❌".to_string(),
                description: diagnostic.report().0,
                remedy: r,
            },
            None => Report {
                result: "✅".to_string(),
                description: diagnostic.report().0,
                remedy: "".to_string(),
            },
        }
    }
}

fn print_table(report: Vec<Report>) {
    let mut table = Table::new(&report);
    table.with(Style::modern());
    println!("{}", table);
}
