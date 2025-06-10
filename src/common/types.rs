// Common type definitions for MCPMate
// This module contains shared enums and types used across the application

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Represents the category of a client application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ClientCategory {
    /// Standalone application that can run independently
    #[default]
    Application,
    /// Extension or plugin that depends on another application
    Extension,
}

impl fmt::Display for ClientCategory {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            ClientCategory::Application => write!(f, "application"),
            ClientCategory::Extension => write!(f, "extension"),
        }
    }
}

impl ClientCategory {
    /// Returns true if this is an application category
    pub fn is_application(&self) -> bool {
        matches!(self, ClientCategory::Application)
    }

    /// Returns true if this is an extension category
    pub fn is_extension(&self) -> bool {
        matches!(self, ClientCategory::Extension)
    }

    /// Parse from string representation (convenience method)
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "application" | "app" => Some(ClientCategory::Application),
            "extension" | "ext" => Some(ClientCategory::Extension),
            _ => None,
        }
    }

    /// Get all possible values
    pub fn all() -> &'static [ClientCategory] {
        &[ClientCategory::Application, ClientCategory::Extension]
    }
}

impl FromStr for ClientCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "application" | "app" => Ok(ClientCategory::Application),
            "extension" | "ext" => Ok(ClientCategory::Extension),
            _ => Err(format!("Invalid client category: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_category_display() {
        assert_eq!(ClientCategory::Application.to_string(), "application");
        assert_eq!(ClientCategory::Extension.to_string(), "extension");
    }

    #[test]
    fn test_client_category_parse() {
        assert_eq!(
            ClientCategory::parse("application"),
            Some(ClientCategory::Application)
        );
        assert_eq!(
            ClientCategory::parse("extension"),
            Some(ClientCategory::Extension)
        );
        assert_eq!(
            ClientCategory::parse("app"),
            Some(ClientCategory::Application)
        );
        assert_eq!(
            ClientCategory::parse("ext"),
            Some(ClientCategory::Extension)
        );
        assert_eq!(ClientCategory::parse("invalid"), None);
    }

    #[test]
    fn test_client_category_from_str() {
        use std::str::FromStr;
        assert_eq!(
            ClientCategory::from_str("application"),
            Ok(ClientCategory::Application)
        );
        assert_eq!(
            ClientCategory::from_str("extension"),
            Ok(ClientCategory::Extension)
        );
        assert_eq!(
            ClientCategory::from_str("app"),
            Ok(ClientCategory::Application)
        );
        assert_eq!(
            ClientCategory::from_str("ext"),
            Ok(ClientCategory::Extension)
        );
        assert!(ClientCategory::from_str("invalid").is_err());
    }

    #[test]
    fn test_client_category_predicates() {
        assert!(ClientCategory::Application.is_application());
        assert!(!ClientCategory::Application.is_extension());
        assert!(ClientCategory::Extension.is_extension());
        assert!(!ClientCategory::Extension.is_application());
    }

    #[test]
    fn test_client_category_serialization() {
        let app = ClientCategory::Application;
        let ext = ClientCategory::Extension;

        let app_json = serde_json::to_string(&app).unwrap();
        let ext_json = serde_json::to_string(&ext).unwrap();

        assert_eq!(app_json, "\"application\"");
        assert_eq!(ext_json, "\"extension\"");

        let app_deserialized: ClientCategory = serde_json::from_str(&app_json).unwrap();
        let ext_deserialized: ClientCategory = serde_json::from_str(&ext_json).unwrap();

        assert_eq!(app_deserialized, app);
        assert_eq!(ext_deserialized, ext);
    }
}
