use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::analyzer::{ToolCategory, analyze_tool};
use super::provider::LlmProvider;
use super::templates::TemplateEngine;
use super::types::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub id: String,
    pub params: serde_json::Value,
    pub description: String,
    pub test_type: TestType,
    pub expected_behavior: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum TestType {
    Normal,
    Boundary,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub case_id: String,
    pub params: serde_json::Value,
    pub actual_response: Option<serde_json::Value>,
    pub latency_ms: u64,
    pub status: TestStatus,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum TestStatus {
    Passed,
    Failed,
    Error,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: Option<serde_json::Value>,
    pub annotations: Option<serde_json::Value>,
}

pub async fn generate_test_cases(
    provider: &dyn LlmProvider,
    templates: &TemplateEngine,
    tool: &ToolInfo,
    template_name: Option<&str>,
    custom_scenario: Option<&str>,
    count: u32,
) -> Result<Vec<TestCase>> {
    let analysis = analyze_tool(&tool.name, &tool.description, &tool.input_schema);

    let effective_template = template_name.unwrap_or(match analysis.category {
        ToolCategory::Query => "test_gen_query",
        ToolCategory::Create => "test_gen_create",
        ToolCategory::Update => "test_gen_update",
        ToolCategory::Delete => "test_gen_delete",
        ToolCategory::Execute => "test_gen_execute",
        ToolCategory::FileIO => "test_gen_fileio",
        _ => "test_gen_custom",
    });

    let mut variables = HashMap::new();
    variables.insert("tool_name".to_string(), tool.name.clone());
    variables.insert("tool_description".to_string(), tool.description.clone());
    variables.insert(
        "input_schema_json".to_string(),
        serde_json::to_string_pretty(&tool.input_schema).unwrap_or_default(),
    );
    variables.insert(
        "output_schema_json".to_string(),
        tool.output_schema
            .as_ref()
            .map(|s| serde_json::to_string_pretty(s).unwrap_or_default())
            .unwrap_or_else(|| "{}".to_string()),
    );
    variables.insert(
        "annotations_json".to_string(),
        tool.annotations
            .as_ref()
            .map(|a| serde_json::to_string_pretty(a).unwrap_or_default())
            .unwrap_or_else(|| "{}".to_string()),
    );
    variables.insert("count".to_string(), count.to_string());
    variables.insert(
        "custom_scenario".to_string(),
        custom_scenario.unwrap_or("").to_string(),
    );

    let system_prompt = templates
        .render(effective_template, &variables)
        .context("Failed to render test generation template")?;

    let request = ChatRequest {
        messages: vec![
            ChatMessage {
                role: Role::System,
                content: system_prompt,
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: Role::User,
                content: format!(
                    "Generate {} test cases for the tool '{}'.",
                    count, tool.name
                ),
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        tools: None,
        temperature: Some(0.7),
        max_tokens: Some(4096),
    };

    let response = provider
        .chat_completion(request)
        .await
        .context("LLM test generation request failed")?;

    parse_test_cases(&response.message.content, count)
}

fn parse_test_cases(raw: &str, _expected_count: u32) -> Result<Vec<TestCase>> {
    let json_str = extract_json_array(raw);

    let cases: Vec<serde_json::Value> =
        serde_json::from_str(&json_str).context("Failed to parse test cases JSON from LLM response")?;

    let test_cases: Vec<TestCase> = cases
        .into_iter()
        .enumerate()
        .map(|(i, v)| {
            let test_type = v
                .get("test_type")
                .and_then(|t| t.as_str())
                .map(|s| match s {
                    "Normal" => TestType::Normal,
                    "Boundary" => TestType::Boundary,
                    "Error" => TestType::Error,
                    _ => TestType::Normal,
                })
                .unwrap_or(TestType::Normal);

            TestCase {
                id: format!("tc_{}", i + 1),
                params: v
                    .get("params")
                    .cloned()
                    .unwrap_or(serde_json::Value::Object(Default::default())),
                description: v
                    .get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("No description")
                    .to_string(),
                test_type,
                expected_behavior: v
                    .get("expected_behavior")
                    .and_then(|e| e.as_str())
                    .unwrap_or("")
                    .to_string(),
            }
        })
        .collect();

    if test_cases.is_empty() {
        anyhow::bail!("LLM returned no valid test cases");
    }

    Ok(test_cases)
}

fn extract_json_array(raw: &str) -> String {
    let trimmed = raw.trim();

    if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            if end > start {
                return trimmed[start..=end].to_string();
            }
        }
    }

    trimmed.to_string()
}
