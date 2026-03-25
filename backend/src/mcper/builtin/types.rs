use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ServerDetail {
    pub association_id: Option<String>,
    pub server_id: String,
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolDetail {
    pub association_id: String,
    pub server_tool_id: String,
    pub server_id: String,
    pub server_name: String,
    pub tool_name: String,
    pub unique_name: String,
    pub description: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PromptDetail {
    pub association_id: Option<String>,
    pub server_id: String,
    pub server_name: String,
    pub prompt_name: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourceDetail {
    pub association_id: Option<String>,
    pub server_id: String,
    pub server_name: String,
    pub resource_uri: String,
    pub enabled: bool,
}
