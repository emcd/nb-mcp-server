use nb_mcp_server::Config;

#[test]
fn config_defaults_are_stable() {
    let config = Config::default();
    assert_eq!(config.notebook, None);
    assert!(!config.commit_signing_disabled);
    assert!(config.create_notebook);
    assert!(!config.allow_top_level_notes);
    assert!(!config.show_paths);
}

#[test]
fn config_converts_to_nb_api_config() {
    let config = Config {
        notebook: Some("test-notebook".to_string()),
        commit_signing_disabled: true,
        create_notebook: false,
        allow_top_level_notes: true,
        show_paths: true,
    };
    let nb_config = config.to_nb_api_config();
    assert_eq!(nb_config.notebook, Some("test-notebook".to_string()));
    assert!(nb_config.disable_git_signing);
    assert!(!nb_config.create_notebook);
    assert!(nb_config.allow_top_level_notes);
}

#[test]
fn nb_mcp_notebook_env_var_falls_back_to_config() {
    // When config.notebook is set, NB_MCP_NOTEBOOK is ignored.
    let config = Config {
        notebook: Some("from-cli".to_string()),
        ..Default::default()
    };
    let nb_config = config.to_nb_api_config();
    assert_eq!(nb_config.notebook, Some("from-cli".to_string()));
}
