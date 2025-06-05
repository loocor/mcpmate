//! Minimal pagination utilities for MCPMate proxy server
//!
//! This module provides basic pagination support for aggregated MCP resources
//! following the MCP specification 2025-03-26.

use rmcp::Error as McpError;
use rmcp::model::{Cursor, PaginatedRequestParam};
use serde::{Deserialize, Serialize};

/// Configuration for pagination behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationConfig {
    /// Default page size for tools
    pub tools_page_size: usize,
    /// Default page size for prompts
    pub prompts_page_size: usize,
    /// Default page size for resources
    pub resources_page_size: usize,
    /// Default page size for resource templates
    pub resource_templates_page_size: usize,
    /// Whether to enable pagination (can be disabled for small deployments)
    pub enabled: bool,
}

impl Default for PaginationConfig {
    fn default() -> Self {
        Self {
            tools_page_size: 0,               // 0 means no pagination for tools
            prompts_page_size: 0,             // 0 means no pagination for prompts
            resources_page_size: 10,          // Enable pagination for resources only
            resource_templates_page_size: 10, // Enable pagination for resource templates
            enabled: true,
        }
    }
}

/// Internal cursor data structure
#[derive(Debug, Serialize, Deserialize)]
struct CursorData {
    /// Current offset in the result set
    offset: usize,
    /// Resource type for validation
    resource_type: String,
    /// Optional total count for validation
    #[serde(skip_serializing_if = "Option::is_none")]
    total: Option<usize>,
}

/// Pagination result containing items and optional next cursor
#[derive(Debug)]
pub struct PaginationResult<T> {
    /// Items for current page
    pub items: Vec<T>,
    /// Next cursor if there are more items
    pub next_cursor: Option<Cursor>,
}

/// Simple paginator for proxy aggregated results
#[derive(Debug, Clone)]
pub struct ProxyPaginator {
    config: PaginationConfig,
}

impl ProxyPaginator {
    /// Create a new paginator with default configuration
    pub fn new() -> Self {
        Self {
            config: PaginationConfig::default(),
        }
    }

    /// Create a new paginator with custom configuration
    pub fn with_config(config: PaginationConfig) -> Self {
        Self { config }
    }

    /// Parse cursor from request parameters
    fn parse_cursor(
        &self,
        request: &Option<PaginatedRequestParam>,
    ) -> Result<usize, McpError> {
        let Some(request) = request else {
            return Ok(0);
        };

        let Some(cursor_str) = &request.cursor else {
            return Ok(0);
        };

        if cursor_str.is_empty() {
            return Ok(0);
        }

        // Simple base64 decode
        let decoded = base64_decode(cursor_str).map_err(|_| {
            McpError::invalid_params(
                "Invalid cursor format",
                Some(serde_json::json!({
                    "cursor": cursor_str
                })),
            )
        })?;

        // Parse JSON
        let cursor_data: CursorData = serde_json::from_slice(&decoded).map_err(|_| {
            McpError::invalid_params(
                "Invalid cursor data",
                Some(serde_json::json!({
                    "cursor": cursor_str
                })),
            )
        })?;

        Ok(cursor_data.offset)
    }

    /// Create cursor for next page
    fn create_cursor(
        &self,
        offset: usize,
        resource_type: &str,
        total: Option<usize>,
    ) -> Result<Cursor, McpError> {
        let cursor_data = CursorData {
            offset,
            resource_type: resource_type.to_string(),
            total,
        };

        let json = serde_json::to_vec(&cursor_data).map_err(|e| {
            McpError::internal_error(
                "Failed to create cursor",
                Some(serde_json::json!({
                    "error": e.to_string()
                })),
            )
        })?;

        Ok(base64_encode(&json))
    }

    /// Paginate tools
    pub fn paginate_tools(
        &self,
        request: &Option<PaginatedRequestParam>,
        mut all_tools: Vec<rmcp::model::Tool>,
    ) -> Result<PaginationResult<rmcp::model::Tool>, McpError> {
        tracing::debug!(
            "Paginate tools called: enabled={}, total_tools={}, page_size={}, request={:?}",
            self.config.enabled,
            all_tools.len(),
            self.config.tools_page_size,
            request
        );

        // Sort tools by name using natural sorting (handles numbers correctly)
        all_tools.sort_by(|a, b| natural_sort_key(&a.name).cmp(&natural_sort_key(&b.name)));
        tracing::debug!("Sorted {} tools by name (natural sort)", all_tools.len());

        // Skip pagination if disabled or page_size is 0
        if !self.config.enabled || self.config.tools_page_size == 0 {
            tracing::debug!(
                "Pagination disabled or page_size is 0, returning all {} tools",
                all_tools.len()
            );
            return Ok(PaginationResult {
                items: all_tools,
                next_cursor: None,
            });
        }

        let offset = self.parse_cursor(request)?;
        let page_size = self.config.tools_page_size;

        tracing::debug!(
            "Pagination enabled: offset={}, page_size={}, total={}",
            offset,
            page_size,
            all_tools.len()
        );

        let result = self.paginate_items(all_tools, offset, page_size, "tools")?;

        tracing::debug!(
            "Pagination result: returning {} items, next_cursor={}",
            result.items.len(),
            result.next_cursor.is_some()
        );

        Ok(result)
    }

    /// Paginate prompts
    pub fn paginate_prompts(
        &self,
        request: &Option<PaginatedRequestParam>,
        mut all_prompts: Vec<rmcp::model::Prompt>,
    ) -> Result<PaginationResult<rmcp::model::Prompt>, McpError> {
        // Sort prompts by name using natural sorting (handles numbers correctly)
        all_prompts.sort_by(|a, b| natural_sort_key(&a.name).cmp(&natural_sort_key(&b.name)));

        // Skip pagination if disabled or page_size is 0
        if !self.config.enabled || self.config.prompts_page_size == 0 {
            return Ok(PaginationResult {
                items: all_prompts,
                next_cursor: None,
            });
        }

        let offset = self.parse_cursor(request)?;
        let page_size = self.config.prompts_page_size;

        self.paginate_items(all_prompts, offset, page_size, "prompts")
    }

    /// Paginate resources
    pub fn paginate_resources(
        &self,
        request: &Option<PaginatedRequestParam>,
        mut all_resources: Vec<rmcp::model::Resource>,
    ) -> Result<PaginationResult<rmcp::model::Resource>, McpError> {
        tracing::debug!(
            "Paginate resources called: enabled={}, total_resources={}, page_size={}, request={:?}",
            self.config.enabled,
            all_resources.len(),
            self.config.resources_page_size,
            request
        );

        // Sort resources by URI using natural sorting (handles numbers correctly)
        all_resources.sort_by(|a, b| natural_sort_key(&a.uri).cmp(&natural_sort_key(&b.uri)));
        tracing::debug!(
            "Sorted {} resources by URI (natural sort)",
            all_resources.len()
        );

        // Skip pagination if disabled or page_size is 0
        if !self.config.enabled || self.config.resources_page_size == 0 {
            tracing::debug!(
                "Pagination disabled or page_size is 0, returning all {} resources",
                all_resources.len()
            );
            return Ok(PaginationResult {
                items: all_resources,
                next_cursor: None,
            });
        }

        let offset = self.parse_cursor(request)?;
        let page_size = self.config.resources_page_size;

        tracing::debug!(
            "Pagination enabled: offset={}, page_size={}, total={}",
            offset,
            page_size,
            all_resources.len()
        );

        let result = self.paginate_items(all_resources, offset, page_size, "resources")?;

        tracing::debug!(
            "Pagination result: returning {} items, next_cursor={}",
            result.items.len(),
            result.next_cursor.is_some()
        );

        Ok(result)
    }

    /// Paginate resource templates
    pub fn paginate_resource_templates(
        &self,
        request: &Option<PaginatedRequestParam>,
        mut all_templates: Vec<rmcp::model::ResourceTemplate>,
    ) -> Result<PaginationResult<rmcp::model::ResourceTemplate>, McpError> {
        // Sort resource templates by uri_template for consistent ordering
        all_templates.sort_by(|a, b| a.uri_template.cmp(&b.uri_template));

        // Skip pagination if disabled or page_size is 0
        if !self.config.enabled || self.config.resource_templates_page_size == 0 {
            return Ok(PaginationResult {
                items: all_templates,
                next_cursor: None,
            });
        }

        let offset = self.parse_cursor(request)?;
        let page_size = self.config.resource_templates_page_size;

        self.paginate_items(all_templates, offset, page_size, "resource_templates")
    }

    /// Generic pagination logic
    fn paginate_items<T>(
        &self,
        items: Vec<T>,
        offset: usize,
        page_size: usize,
        resource_type: &str,
    ) -> Result<PaginationResult<T>, McpError> {
        let total_items = items.len();

        // Validate offset
        if offset > total_items {
            return Err(McpError::invalid_params(
                "Cursor offset exceeds available items",
                Some(serde_json::json!({
                    "offset": offset,
                    "total_items": total_items
                })),
            ));
        }

        // Extract current page
        let end = std::cmp::min(offset + page_size, total_items);
        let page_items: Vec<T> = items.into_iter().skip(offset).take(end - offset).collect();

        // Create next cursor if there are more items
        let next_cursor = if end < total_items {
            Some(self.create_cursor(end, resource_type, Some(total_items))?)
        } else {
            None
        };

        Ok(PaginationResult {
            items: page_items,
            next_cursor,
        })
    }

    /// Get pagination configuration
    pub fn config(&self) -> &PaginationConfig {
        &self.config
    }
}

impl Default for ProxyPaginator {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple base64 encoding (for cursor creation)
fn base64_encode(input: &[u8]) -> String {
    // Using a simple base64 implementation to avoid external dependencies
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();
    let mut i = 0;

    while i < input.len() {
        let b1 = input[i];
        let b2 = input.get(i + 1).copied().unwrap_or(0);
        let b3 = input.get(i + 2).copied().unwrap_or(0);

        let n = ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32);

        result.push(CHARS[((n >> 18) & 63) as usize] as char);
        result.push(CHARS[((n >> 12) & 63) as usize] as char);

        if i + 1 < input.len() {
            result.push(CHARS[((n >> 6) & 63) as usize] as char);
        } else {
            result.push('=');
        }

        if i + 2 < input.len() {
            result.push(CHARS[(n & 63) as usize] as char);
        } else {
            result.push('=');
        }

        i += 3;
    }

    result
}

/// Simple base64 decoding (for cursor parsing)
fn base64_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    let input = input.trim_end_matches('=');
    let mut result = Vec::new();
    let mut chars = input.chars().peekable();

    while chars.peek().is_some() {
        let mut n = 0u32;
        let mut padding = 0;

        for _ in 0..4 {
            if let Some(c) = chars.next() {
                let val = match c {
                    'A'..='Z' => (c as u8 - b'A') as u32,
                    'a'..='z' => (c as u8 - b'a' + 26) as u32,
                    '0'..='9' => (c as u8 - b'0' + 52) as u32,
                    '+' => 62,
                    '/' => 63,
                    '=' => {
                        padding += 1;
                        0
                    }
                    _ => return Err("Invalid base64 character"),
                };
                n = (n << 6) | val;
            } else {
                padding += 1;
                n <<= 6;
            }
        }

        result.push((n >> 16) as u8);
        if padding < 2 {
            result.push((n >> 8) as u8);
        }
        if padding < 1 {
            result.push(n as u8);
        }
    }

    Ok(result)
}

/// Natural sorting key generator that handles numeric parts correctly
///
/// For example: "resource1", "resource2", "resource10" will sort as 1, 2, 10
/// instead of the lexicographic order: 1, 10, 2
fn natural_sort_key(s: &str) -> Vec<SortKeyPart> {
    let mut result = Vec::new();
    let mut chars = s.chars().peekable();

    while chars.peek().is_some() {
        // Check if we're starting a number
        if chars.peek().unwrap().is_ascii_digit() {
            let mut number_str = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_ascii_digit() {
                    number_str.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            if let Ok(num) = number_str.parse::<u64>() {
                result.push(SortKeyPart::Number(num));
            } else {
                // Fallback to string if number is too large
                result.push(SortKeyPart::String(number_str));
            }
        } else {
            // Collect non-numeric characters
            let mut text = String::new();
            while let Some(&ch) = chars.peek() {
                if !ch.is_ascii_digit() {
                    text.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            result.push(SortKeyPart::String(text));
        }
    }

    result
}

/// Parts of a natural sort key
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum SortKeyPart {
    String(String),
    Number(u64),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_round_trip() {
        let data = b"Hello, World!";
        let encoded = base64_encode(data);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(data.to_vec(), decoded);
    }

    #[test]
    fn test_pagination_config_defaults() {
        let config = PaginationConfig::default();
        assert_eq!(config.tools_page_size, 0);
        assert_eq!(config.prompts_page_size, 0);
        assert_eq!(config.resources_page_size, 10);
        assert_eq!(config.resource_templates_page_size, 0);
        assert!(config.enabled);
    }

    #[test]
    fn test_cursor_data_serialization() {
        let cursor_data = CursorData {
            offset: 100,
            resource_type: "tools".to_string(),
            total: Some(200),
        };

        let json = serde_json::to_vec(&cursor_data).unwrap();
        let decoded: CursorData = serde_json::from_slice(&json).unwrap();

        assert_eq!(cursor_data.offset, decoded.offset);
        assert_eq!(cursor_data.resource_type, decoded.resource_type);
        assert_eq!(cursor_data.total, decoded.total);
    }

    #[test]
    fn test_natural_sorting() {
        let test_strings = vec![
            "resource1",
            "resource10",
            "resource100",
            "resource11",
            "resource2",
            "resource20",
            "resource3",
        ];

        let mut sorted_strings = test_strings.clone();
        sorted_strings.sort_by(|a, b| natural_sort_key(a).cmp(&natural_sort_key(b)));

        let expected = vec![
            "resource1",
            "resource2",
            "resource3",
            "resource10",
            "resource11",
            "resource20",
            "resource100",
        ];

        assert_eq!(sorted_strings, expected);
    }

    #[test]
    fn test_natural_sorting_uris() {
        let test_uris = vec![
            "test://static/resource/1",
            "test://static/resource/10",
            "test://static/resource/100",
            "test://static/resource/11",
            "test://static/resource/2",
            "test://static/resource/20",
        ];

        let mut sorted_uris = test_uris.clone();
        sorted_uris.sort_by(|a, b| natural_sort_key(a).cmp(&natural_sort_key(b)));

        let expected = vec![
            "test://static/resource/1",
            "test://static/resource/2",
            "test://static/resource/10",
            "test://static/resource/11",
            "test://static/resource/20",
            "test://static/resource/100",
        ];

        assert_eq!(sorted_uris, expected);
    }
}
