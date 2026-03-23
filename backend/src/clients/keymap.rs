use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use crate::clients::error::{ConfigError, ConfigResult};

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default)]
struct RawKeyMap {
    pub version: Option<String>,
    pub transports: HashMap<String, Vec<String>>, // norm -> aliases
}

#[derive(Debug, Default, Clone)]
pub struct KeyMapRegistry {
    transports: HashMap<String, HashSet<String>>, // norm -> alias set (lowercased)
}

impl KeyMapRegistry {
    fn from_raw(raw: RawKeyMap) -> Self {
        let mut transports: HashMap<String, HashSet<String>> = HashMap::new();
        for (norm, aliases) in raw.transports.into_iter() {
            let key = norm.to_ascii_lowercase();
            let set: HashSet<String> = aliases.into_iter().map(|a| a.to_ascii_lowercase()).collect();
            transports.insert(key, set);
        }
        Self { transports }
    }

    fn with_defaults() -> Self {
        let raw = RawKeyMap {
            version: Some("2025-10-11".into()),
            transports: HashMap::from([
                (
                    "streamable_http".into(),
                    vec![
                        "streamableHttp".into(),
                        "http".into(),
                        "HTTP".into(),
                        "sse".into(),
                        "SSE".into(),
                    ],
                ),
                ("stdio".into(), vec!["STDIO".into(), "Stdio".into()]),
            ]),
        };
        Self::from_raw(raw)
    }

    /// Determine if the map contains a rule for the normalized transport in the given format_rules map
    pub fn has_rule(
        &self,
        format_rules: &HashMap<String, crate::clients::models::FormatRule>,
        norm: &str,
    ) -> bool {
        let norm_lc = norm.to_ascii_lowercase();
        if format_rules.contains_key(&norm_lc) {
            return true;
        }
        if let Some(aliases) = self.transports.get(&norm_lc) {
            for alias in aliases {
                if format_rules.contains_key(alias) {
                    return true;
                }
            }
        }
        false
    }

    /// Resolve the concrete key existing in format_rules for the given normalized transport
    pub fn resolve_rule_key(
        &self,
        format_rules: &HashMap<String, crate::clients::models::FormatRule>,
        norm: &str,
    ) -> Option<String> {
        let norm_lc = norm.to_ascii_lowercase();
        if format_rules.contains_key(&norm_lc) {
            return Some(norm_lc);
        }
        if let Some(aliases) = self.transports.get(&norm_lc) {
            for alias in aliases {
                if format_rules.contains_key(alias) {
                    return Some(alias.clone());
                }
            }
        }
        None
    }

    /// Advertise normalized supported transports based on presence of normalized or alias keys
    pub fn advertise_supported(
        &self,
        format_rules: &HashMap<String, crate::clients::models::FormatRule>,
    ) -> Vec<String> {
        let mut out = Vec::new();
        for t in ["streamable_http", "stdio"] {
            if self.has_rule(format_rules, t) {
                out.push(t.to_string());
            }
        }
        out
    }
}

static REGISTRY: Lazy<RwLock<KeyMapRegistry>> = Lazy::new(|| RwLock::new(KeyMapRegistry::with_defaults()));

pub fn reload() -> ConfigResult<()> {
    use crate::system::paths::PathService;
    let path_service = PathService::new().map_err(|e| ConfigError::PathResolutionError(e.to_string()))?;
    let root = path_service
        .resolve_user_path("~/.mcpmate/client/common/keymap.json5")
        .map_err(|e| ConfigError::PathResolutionError(e.to_string()))?;
    let path = root;
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(ConfigError::IoError)?;
        }
        // Seed default file so users can discover and edit it.
        let content = r#"{
  "version": "2025-10-11",
  "transports": {
    "streamable_http": ["streamableHttp", "http", "HTTP", "sse", "SSE"],
    "stdio": ["STDIO", "Stdio"]
  }
}
"#;
        std::fs::write(&path, content).map_err(ConfigError::IoError)?;
    }
    let content = std::fs::read_to_string(&path).map_err(ConfigError::IoError)?;
    let raw: RawKeyMap = json5::from_str(&content)
        .or_else(|_| serde_json::from_str(&content))
        .map_err(|e| ConfigError::TemplateParseError(format!("Failed to parse keymap.json5: {}", e)))?;
    let registry = KeyMapRegistry::from_raw(raw);
    if let Ok(mut guard) = REGISTRY.write() {
        *guard = registry;
    }
    Ok(())
}

pub fn registry() -> KeyMapRegistry {
    REGISTRY.read().unwrap().clone()
}
