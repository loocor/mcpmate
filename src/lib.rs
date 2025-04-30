pub mod client;
pub mod config;
pub mod proxy;

#[cfg(test)]
mod tests {
    use super::config::*;
    use std::path::PathBuf;

    #[test]
    fn test_load_server_config() {
        let config_path = PathBuf::from("config/mcp.json");
        let config = load_server_config(config_path).unwrap();

        // verify the basic structure
        assert!(config.mcp_servers.contains_key("blender"));
        assert!(config.mcp_servers.contains_key("firecrawl"));

        // verify stdio type service
        let blender = config.mcp_servers.get("blender").unwrap();
        assert_eq!(blender.kind, "stdio");
        assert_eq!(blender.command.as_deref(), Some("uvx"));
        assert!(blender
            .args
            .as_ref()
            .map_or(false, |args| args.contains(&"blender-mcp".to_string())));

        // verify optional fields
        let firecrawl = config.mcp_servers.get("firecrawl").unwrap();
        assert!(firecrawl.env.is_some());
        assert!(firecrawl
            .env
            .as_ref()
            .unwrap()
            .contains_key("FIRECRAWL_API_KEY"));

        // verify sse type service
        let thinking = config.mcp_servers.get("thinking").unwrap();
        assert_eq!(thinking.kind, "sse");
        assert!(thinking.url.is_some());
    }

    #[test]
    fn test_load_rule_config() {
        let rule_path = PathBuf::from("config/rule.json5");
        let config = load_rule_config(rule_path).unwrap();

        // verify the basic structure
        assert!(config.rules.contains_key("blender"));
        assert!(config.rules.contains_key("firecrawl"));

        // verify the service enabled status
        let blender = config.rules.get("blender").unwrap();
        assert!(blender.enabled);
        assert!(blender.tools.is_empty());

        // verify the disabled service
        let proxy = config.rules.get("proxy").unwrap();
        assert!(!proxy.enabled);
    }

    #[test]
    fn test_config_error_handling() {
        // test the file not exists case
        let result = load_server_config("nonexistent.json");
        assert!(result.is_err());

        // test the file format error case
        let result = load_rule_config("Cargo.toml");
        assert!(result.is_err());
    }
}
