use crate::subcommands::doctor::git_config_env::GitConfigEnvVars;
use crate::subcommands::doctor::less::Less;
use crate::subcommands::doctor::pager_env::PagerEnvVars;
use crate::subcommands::doctor::shared::Diagnostic;
use tabled::{settings::Style, Table, Tabled};

#[cfg(not(tarpaulin_include))]
pub fn run_diagnostics() -> std::io::Result<()> {
    let diagnostics: Vec<Box<dyn Diagnostic>> = vec![
        Box::new(Less::probe()),
        Box::new(GitConfigEnvVars::probe()),
        Box::new(PagerEnvVars::probe()),
    ];

    let mut reports = Vec::new();
    for d in diagnostics {
        reports.push(build_report_row(d));
    }

    print_table(reports);
    Ok(())
}

fn build_report_row(diagnostic: Box<dyn Diagnostic>) -> Report {
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

#[derive(Tabled)]
struct Report {
    result: String,
    description: String,
    remedy: String,
}

fn print_table(report: Vec<Report>) {
    let mut table = Table::new(&report);
    table.with(Style::modern());
    println!("{}", table);
}
