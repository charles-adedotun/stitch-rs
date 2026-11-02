use stitch_rs::{sanitize_filename, AuthConfig, DeviceType, ModelId, StitchAgent};

#[test]
fn test_filename_sanitization() {
    assert_eq!(sanitize_filename("  My Cool Screen! "), "my_cool_screen");
    assert_eq!(
        sanitize_filename("Crypto Trading / Terminal 2026"),
        "crypto_trading_terminal_2026"
    );
    assert_eq!(sanitize_filename(""), "untitled");
    assert_eq!(sanitize_filename("___"), "untitled");
}

#[test]
fn test_enums_parsing_and_display() {
    let dev = "desktop".parse::<DeviceType>().unwrap();
    assert_eq!(dev, DeviceType::Desktop);
    assert_eq!(dev.as_str(), "DESKTOP");
    assert_eq!(format!("{dev}"), "DESKTOP");

    let model = "gemini-3.8-flash".parse::<ModelId>().unwrap();
    assert_eq!(model, ModelId::Gemini38Flash);
    assert_eq!(model.as_str(), "GEMINI_3_8_FLASH");
}

#[test]
fn test_agent_construction_with_config() {
    let auth = AuthConfig::from_api_key("dummy_key");
    let agent = StitchAgent::new(Some(auth)).expect("agent built");
    assert_eq!(agent.rpc.base_url, "https://stitch.googleapis.com/mcp");

    // LLM Adapters on agent instance
    let gemini = agent.to_gemini_tools(Some(&["create_project"]));
    assert!(gemini[0]["functionDeclarations"].as_array().unwrap().len() == 1);

    let openai = agent.to_openai_tools(Some(&["create_project"]));
    assert!(openai.as_array().unwrap().len() == 1);

    let mcp = agent.to_mcp_tools(Some(&["create_project"]));
    assert!(mcp.as_array().unwrap().len() == 1);
}
