use crate::subcommands::doctor::git_config_env::GitConfigEnvVars;
use crate::subcommands::doctor::less::Less;
use crate::subcommands::doctor::pager_env::PagerEnvVars;
use crate::subcommands::doctor::shared::Diagnostic;

#[cfg(not(tarpaulin_include))]
pub fn doctor() -> std::io::Result<()> {
    let diagnostics: Vec<Box<dyn Diagnostic>> = vec![
        Box::new(Less::probe()),
        Box::new(GitConfigEnvVars::probe()),
        Box::new(PagerEnvVars::probe()),
    ];

    for d in diagnostics {
        println!("{}", d.report());
    }

    Ok(())
}
