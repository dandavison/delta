fn load(path: &Path) -> Config {
    let text = read(path);
    let config = parse(&text);
    validate(&config);
    log_config(&config);
    config
}
