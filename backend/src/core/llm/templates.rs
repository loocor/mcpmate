use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::SystemTime;

fn process_conditionals(content: &str, variables: &HashMap<String, String>) -> String {
    let mut result = String::new();
    let mut remaining = content;

    while let Some(start) = remaining.find("{{#if ") {
        result.push_str(&remaining[..start]);
        let after_tag = &remaining[start + 6..];

        let var_end = after_tag.find("}}").unwrap_or(after_tag.len());
        let var_name = after_tag[..var_end].trim();
        let after_open = &after_tag[var_end + 2..];

        // Find {{/if}}
        if let Some(end) = after_open.find("{{/if}}") {
            let block_content = &after_open[..end];
            let is_truthy = variables
                .get(var_name)
                .map(|v| !v.is_empty())
                .unwrap_or(false);

            if is_truthy {
                result.push_str(block_content);
            }
            remaining = &after_open[end + 7..];
        } else {
            // No matching {{/if}}, keep as-is
            result.push_str("{{#if ");
            result.push_str(var_name);
            result.push_str("}}");
            remaining = after_open;
        }
    }

    result.push_str(remaining);
    result
}

struct CachedTemplate {
    content: String,
    modified: SystemTime,
}

pub struct TemplateEngine {
    template_dir: PathBuf,
    cache: RwLock<HashMap<String, CachedTemplate>>,
}

impl TemplateEngine {
    pub fn new(template_dir: impl Into<PathBuf>) -> Self {
        Self {
            template_dir: template_dir.into(),
            cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn render(
        &self,
        template_name: &str,
        variables: &HashMap<String, String>,
    ) -> Result<String> {
        let content = self.load_template(template_name)?;

        // First pass: process {{#if var}}...{{/if}} blocks
        let content = process_conditionals(&content, variables);

        // Second pass: replace {{variable}} placeholders
        let mut result = content;
        for (key, value) in variables {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }

        Ok(result)
    }

    fn load_template(&self, name: &str) -> Result<String> {
        // Prevent path traversal
        if name.contains('/')
            || name.contains('\\')
            || name.contains('\0')
            || name.contains("..")
        {
            anyhow::bail!("Invalid template name: {}", name);
        }

        {
            let cache = self.cache.read().unwrap();
            if let Some(cached) = cache.get(name) {
                let path = self.template_path(name);
                if let Ok(metadata) = std::fs::metadata(&path) {
                    if let Ok(modified) = metadata.modified() {
                        if modified <= cached.modified {
                            return Ok(cached.content.clone());
                        }
                    }
                }
            }
        }

        let path = self.template_path(name);
        let content = std::fs::read_to_string(&path)
            .context(format!("Failed to read template: {}", path.display()))?;

        let modified = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::now());

        let mut cache = self.cache.write().unwrap();
        cache.insert(
            name.to_string(),
            CachedTemplate {
                content: content.clone(),
                modified,
            },
        );

        Ok(content)
    }

    fn template_path(&self, name: &str) -> PathBuf {
        self.template_dir.join(format!("{}.txt", name))
    }

    pub fn list_templates(&self) -> Result<Vec<String>> {
        let mut templates = Vec::new();
        if self.template_dir.exists() {
            for entry in std::fs::read_dir(&self.template_dir)? {
                let entry = entry?;
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".txt") {
                        templates.push(name.trim_end_matches(".txt").to_string());
                    }
                }
            }
        }
        templates.sort();
        Ok(templates)
    }
}
