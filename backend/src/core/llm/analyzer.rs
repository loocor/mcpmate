use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum ToolCategory {
    Query,
    Create,
    Update,
    Delete,
    Execute,
    FileIO,
    ApiProxy,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAnalysis {
    pub category: ToolCategory,
    pub required_fields: Vec<String>,
    pub optional_fields: Vec<String>,
    pub enum_fields: Vec<EnumField>,
    pub has_nested_objects: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumField {
    pub name: String,
    pub values: Vec<String>,
}

pub fn classify_tool_type(
    tool_name: &str,
    description: &str,
    _input_schema: &serde_json::Value,
) -> ToolCategory {
    let name_lower = tool_name.to_lowercase();
    let desc_lower = description.to_lowercase();

    let has_write_hint = desc_lower.contains("create")
        || desc_lower.contains("write")
        || desc_lower.contains("add")
        || desc_lower.contains("insert")
        || desc_lower.contains("save")
        || desc_lower.contains("store");

    let has_delete_hint = desc_lower.contains("delete")
        || desc_lower.contains("remove")
        || desc_lower.contains("destroy")
        || desc_lower.contains("drop");

    let has_update_hint = desc_lower.contains("update")
        || desc_lower.contains("modify")
        || desc_lower.contains("edit")
        || desc_lower.contains("patch")
        || desc_lower.contains("change");

    if name_lower.starts_with("get_")
        || name_lower.starts_with("list_")
        || name_lower.starts_with("search_")
        || name_lower.starts_with("find_")
        || name_lower.starts_with("query_")
        || name_lower.starts_with("fetch_")
        || name_lower.starts_with("read_") && !name_lower.starts_with("read_file")
    {
        return ToolCategory::Query;
    }

    if name_lower.starts_with("create_")
        || name_lower.starts_with("add_")
        || name_lower.starts_with("insert_")
        || name_lower.starts_with("new_")
        || name_lower.starts_with("write_") && !name_lower.starts_with("write_file")
    {
        return ToolCategory::Create;
    }

    if name_lower.starts_with("update_")
        || name_lower.starts_with("modify_")
        || name_lower.starts_with("edit_")
        || name_lower.starts_with("patch_")
    {
        return ToolCategory::Update;
    }

    if name_lower.starts_with("delete_")
        || name_lower.starts_with("remove_")
        || name_lower.starts_with("destroy_")
    {
        return ToolCategory::Delete;
    }

    if name_lower.starts_with("read_file")
        || name_lower.starts_with("write_file")
        || name_lower.starts_with("list_files")
        || name_lower.starts_with("create_directory")
        || name_lower.contains("file")
        || name_lower.contains("path")
        || name_lower.contains("directory")
    {
        return ToolCategory::FileIO;
    }

    if name_lower.starts_with("run_")
        || name_lower.starts_with("execute_")
        || name_lower.starts_with("call_")
        || name_lower.starts_with("invoke_")
        || name_lower.starts_with("start_")
        || name_lower.starts_with("stop_")
    {
        return ToolCategory::Execute;
    }

    if has_write_hint && !has_delete_hint && !has_update_hint {
        return ToolCategory::Create;
    }
    if has_delete_hint {
        return ToolCategory::Delete;
    }
    if has_update_hint {
        return ToolCategory::Update;
    }

    let is_query_like = desc_lower.contains("retrieve")
        || desc_lower.contains("return")
        || desc_lower.contains("list")
        || desc_lower.contains("get")
        || desc_lower.contains("find")
        || desc_lower.contains("search")
        || desc_lower.contains("query");

    if is_query_like {
        return ToolCategory::Query;
    }

    ToolCategory::Unknown
}

pub fn extract_field_info(schema: &serde_json::Value) -> (Vec<String>, Vec<String>, Vec<EnumField>) {
    let mut required = Vec::new();
    let mut optional = Vec::new();
    let mut enums = Vec::new();

    let properties = match schema.get("properties").and_then(|p| p.as_object()) {
        Some(props) => props,
        None => return (required, optional, enums),
    };

    let required_set: std::collections::HashSet<String> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    for (name, prop_schema) in properties {
        if required_set.contains(name) {
            required.push(name.clone());
        } else {
            optional.push(name.clone());
        }

        if let Some(enum_values) = prop_schema.get("enum").and_then(|e| e.as_array()) {
            let values: Vec<String> = enum_values
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            if !values.is_empty() {
                enums.push(EnumField {
                    name: name.clone(),
                    values,
                });
            }
        }
    }

    (required, optional, enums)
}

pub fn analyze_tool(
    tool_name: &str,
    description: &str,
    input_schema: &serde_json::Value,
) -> ToolAnalysis {
    let category = classify_tool_type(tool_name, description, input_schema);
    let (required_fields, optional_fields, enum_fields) = extract_field_info(input_schema);

    let has_nested_objects = input_schema
        .get("properties")
        .and_then(|p| p.as_object())
        .map(|props| {
            props.values().any(|v| {
                v.get("type").and_then(|t| t.as_str()) == Some("object")
                    || v.get("type").and_then(|t| t.as_str()) == Some("array")
            })
        })
        .unwrap_or(false);

    ToolAnalysis {
        category,
        required_fields,
        optional_fields,
        enum_fields,
        has_nested_objects,
    }
}
