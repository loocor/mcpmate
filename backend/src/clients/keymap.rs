use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default, Clone)]
pub struct KeyMapRegistry {
    transports: HashMap<String, HashSet<String>>, // norm -> alias set (lowercased)
}

impl KeyMapRegistry {
    fn from_defaults() -> Self {
        let mut transports: HashMap<String, HashSet<String>> = HashMap::new();
        for (norm, aliases) in default_transport_aliases() {
            let key = norm.to_ascii_lowercase();
            let set: HashSet<String> = aliases.iter().map(|alias| alias.to_ascii_lowercase()).collect();
            transports.insert(key, set);
        }
        Self { transports }
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
        for t in ["streamable_http", "sse", "stdio"] {
            if self.has_rule(format_rules, t) {
                out.push(t.to_string());
            }
        }
        out
    }
}

static REGISTRY: Lazy<KeyMapRegistry> = Lazy::new(KeyMapRegistry::from_defaults);

fn default_transport_aliases() -> [(&'static str, &'static [&'static str]); 3] {
    [
        ("streamable_http", &["streamableHttp", "http", "HTTP"]),
        ("sse", &["SSE", "sse"]),
        ("stdio", &["STDIO", "Stdio", "stdio"]),
    ]
}

pub fn registry() -> KeyMapRegistry {
    REGISTRY.clone()
}
