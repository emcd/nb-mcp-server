use nb_mcp_server::Config;

#[test]
fn config_defaults_are_stable() {
    let config = Config::default();
    assert_eq!(config.notebook, None);
    assert!(!config.commit_signing_disabled);
    assert!(config.create_notebook);
    assert!(!config.show_paths);
}
