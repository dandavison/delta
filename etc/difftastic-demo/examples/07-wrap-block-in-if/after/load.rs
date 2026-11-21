fn load(path: &Path, verbose: bool) -> Config {
    let text = read(path);
    let config = parse(&text);
    validate(&config);
    if verbose {
        log_config(&config);
    }
    config
}
