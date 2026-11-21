// Parse the config file at path, returning its settings.
fn parse_config(path: &Path) -> Settings {
    Settings::from(read(path))
}
