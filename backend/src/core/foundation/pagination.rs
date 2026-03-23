//! Core Pagination Utilities
//!
//! provides minimal pagination tools for MCPMate proxy servers
//!
//! this module provides basic pagination support for aggregated MCP resources,
//! following the MCP specification 2025-03-26.

use rmcp::ErrorData as McpError;
use rmcp::model::{Cursor, PaginatedRequestParams};
use serde::{Deserialize, Serialize};

/// pagination behavior configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationConfig {
    /// default page size for tools
    pub tools_page_size: usize,
    /// default page size for prompts
    pub prompts_page_size: usize,
    /// default page size for resources
    pub resources_page_size: usize,
    /// default page size for resource templates
    pub resource_templates_page_size: usize,
    /// whether to enable pagination (can be disabled for small deployments)
    pub enabled: bool,
}

impl Default for PaginationConfig {
    fn default() -> Self {
        Self {
            tools_page_size: 0,               // 0 means tools are not paginated
            prompts_page_size: 0,             // 0 means prompts are not paginated
            resources_page_size: 10,          // only enable pagination for resources
            resource_templates_page_size: 10, // enable pagination for resource templates
            enabled: true,
        }
    }
}

/// internal cursor data structure
#[derive(Debug, Serialize, Deserialize)]
struct CursorData {
    /// current offset in the result set
    offset: usize,
    /// resource type for validation
    resource_type: String,
    /// optional total for validation
    #[serde(skip_serializing_if = "Option::is_none")]
    total: Option<usize>,
}

/// pagination result containing items and optional next cursor
#[derive(Debug)]
pub struct PaginationResult<T> {
    /// items in the current page
    pub items: Vec<T>,
    /// next cursor if there are more items
    pub next_cursor: Option<Cursor>,
}

/// simple paginator for aggregated results
#[derive(Debug, Clone)]
pub struct ProxyPaginator {
    config: PaginationConfig,
}

impl ProxyPaginator {
    /// create a new paginator with default configuration
    pub fn new() -> Self {
        Self {
            config: PaginationConfig::default(),
        }
    }

    /// create a new paginator with custom configuration
    pub fn with_config(config: PaginationConfig) -> Self {
        Self { config }
    }

    /// parse cursor from request parameters
    fn parse_cursor(
        &self,
        request: &Option<PaginatedRequestParams>,
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

        // simple base64 decode
        let decoded = base64_decode(cursor_str).map_err(|_| {
            McpError::invalid_params(
                "Invalid cursor format",
                Some(serde_json::json!({
                    "cursor": cursor_str
                })),
            )
        })?;

        // parse JSON
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

    /// create cursor for next page
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

    /// paginate tools
    pub fn paginate_tools(
        &self,
        request: &Option<PaginatedRequestParams>,
        mut all_tools: Vec<rmcp::model::Tool>,
    ) -> Result<PaginationResult<rmcp::model::Tool>, McpError> {
        tracing::debug!(
            "Paginate tools called: enabled={}, total_tools={}, page_size={}, request={:?}",
            self.config.enabled,
            all_tools.len(),
            self.config.tools_page_size,
            request
        );

        // sort tools by name using natural sort (correctly handle numbers)
        all_tools.sort_by(|a, b| natural_sort_key(&a.name).cmp(&natural_sort_key(&b.name)));
        tracing::debug!("Sorted {} tools by name (natural sort)", all_tools.len());

        // if pagination is disabled or page_size is 0, skip pagination
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

    /// paginate prompts
    pub fn paginate_prompts(
        &self,
        request: &Option<PaginatedRequestParams>,
        mut all_prompts: Vec<rmcp::model::Prompt>,
    ) -> Result<PaginationResult<rmcp::model::Prompt>, McpError> {
        // sort prompts by name using natural sort (correctly handle numbers)
        all_prompts.sort_by(|a, b| natural_sort_key(&a.name).cmp(&natural_sort_key(&b.name)));

        // if pagination is disabled or page_size is 0, skip pagination
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

    /// paginate resources
    pub fn paginate_resources(
        &self,
        request: &Option<PaginatedRequestParams>,
        mut all_resources: Vec<rmcp::model::Resource>,
    ) -> Result<PaginationResult<rmcp::model::Resource>, McpError> {
        // sort resources by URI using natural sort (correctly handle numbers)
        all_resources.sort_by(|a, b| natural_sort_key(&a.uri).cmp(&natural_sort_key(&b.uri)));

        // if pagination is disabled or page_size is 0, skip pagination
        if !self.config.enabled || self.config.resources_page_size == 0 {
            return Ok(PaginationResult {
                items: all_resources,
                next_cursor: None,
            });
        }

        let offset = self.parse_cursor(request)?;
        let page_size = self.config.resources_page_size;

        self.paginate_items(all_resources, offset, page_size, "resources")
    }

    /// paginate resource templates
    pub fn paginate_resource_templates(
        &self,
        request: &Option<PaginatedRequestParams>,
        mut all_templates: Vec<rmcp::model::ResourceTemplate>,
    ) -> Result<PaginationResult<rmcp::model::ResourceTemplate>, McpError> {
        // sort resource templates by URI template using natural sort (correctly handle numbers)
        all_templates.sort_by(|a, b| natural_sort_key(&a.uri_template).cmp(&natural_sort_key(&b.uri_template)));

        // if pagination is disabled or page_size is 0, skip pagination
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

    /// paginate items
    fn paginate_items<T>(
        &self,
        items: Vec<T>,
        offset: usize,
        page_size: usize,
        resource_type: &str,
    ) -> Result<PaginationResult<T>, McpError> {
        let total = items.len();

        // check if offset is valid
        if offset >= total && total > 0 {
            return Err(McpError::invalid_params(
                "Offset exceeds total items",
                Some(serde_json::json!({
                    "offset": offset,
                    "total": total
                })),
            ));
        }

        // calculate page items
        let end = std::cmp::min(offset + page_size, total);
        let page_items = items.into_iter().skip(offset).take(end - offset).collect();

        // if there are more items, create next cursor
        let next_cursor = if end < total {
            Some(self.create_cursor(end, resource_type, Some(total))?)
        } else {
            None
        };

        Ok(PaginationResult {
            items: page_items,
            next_cursor,
        })
    }

    /// get configuration
    pub fn config(&self) -> &PaginationConfig {
        &self.config
    }
}

impl Default for ProxyPaginator {
    fn default() -> Self {
        Self::new()
    }
}

/// Base64 encode
fn base64_encode(input: &[u8]) -> String {
    use base64::{Engine as _, engine::general_purpose};
    general_purpose::STANDARD.encode(input)
}

/// Base64 decode
fn base64_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    use base64::{Engine as _, engine::general_purpose};
    general_purpose::STANDARD.decode(input).map_err(|_| "Invalid base64")
}

/// create sort key for natural sort
fn natural_sort_key(s: &str) -> Vec<SortKeyPart> {
    let mut result = Vec::new();
    let mut current_string = String::new();
    let mut current_number = String::new();
    let mut in_number = false;

    for ch in s.chars() {
        if ch.is_ascii_digit() {
            if !in_number {
                // switch from string to number
                if !current_string.is_empty() {
                    result.push(SortKeyPart::String(current_string.clone()));
                    current_string.clear();
                }
                in_number = true;
            }
            current_number.push(ch);
        } else {
            if in_number {
                // switch from number to string
                if !current_number.is_empty() {
                    if let Ok(num) = current_number.parse::<u64>() {
                        result.push(SortKeyPart::Number(num));
                    } else {
                        result.push(SortKeyPart::String(current_number.clone()));
                    }
                    current_number.clear();
                }
                in_number = false;
            }
            current_string.push(ch);
        }
    }

    // handle remaining part
    if in_number && !current_number.is_empty() {
        if let Ok(num) = current_number.parse::<u64>() {
            result.push(SortKeyPart::Number(num));
        } else {
            result.push(SortKeyPart::String(current_number));
        }
    } else if !current_string.is_empty() {
        result.push(SortKeyPart::String(current_string));
    }

    result
}

/// sort key part
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum SortKeyPart {
    String(String),
    Number(u64),
}
