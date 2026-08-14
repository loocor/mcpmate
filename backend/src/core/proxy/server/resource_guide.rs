use rmcp::ErrorData;
use rmcp::model::{ReadResourceResult, Resource, ResourceContents, ResourceTemplate};

const README_URI: &str = "mcpmate://resources/mcpmate/readme";
const GUIDE_URI_PREFIX: &str = "mcpmate://resources/template/mcpmate/guide/";
const GUIDE_URI_TEMPLATE: &str = "mcpmate://resources/template/mcpmate/guide/{topic}";

const README: &str = include_str!("resource_guide/readme.md");
const OVERVIEW: &str = include_str!("resource_guide/overview.md");
const CATALOG: &str = include_str!("resource_guide/catalog.md");
const DETAILS: &str = include_str!("resource_guide/details.md");
const READ: &str = include_str!("resource_guide/read.md");
const TROUBLESHOOTING: &str = include_str!("resource_guide/troubleshooting.md");

pub(crate) fn listed_resource() -> Resource {
    Resource::new(README_URI, "Readme")
        .with_description("MCPMate resource workflow overview")
        .with_mime_type("text/markdown")
}

pub(crate) fn listed_template() -> ResourceTemplate {
    ResourceTemplate::new(GUIDE_URI_TEMPLATE, "MCPMate Resource Guide")
        .with_description("Read a stable MCPMate resource workflow guide by topic")
        .with_mime_type("text/markdown")
}

pub(crate) fn try_read(uri: &str) -> Option<Result<ReadResourceResult, ErrorData>> {
    if uri == README_URI {
        return Some(Ok(markdown_result(uri, README)));
    }

    let topic = uri.strip_prefix(GUIDE_URI_PREFIX)?;
    if topic.is_empty() || topic.contains('/') {
        return None;
    }

    let markdown = match topic {
        "overview" => OVERVIEW,
        "catalog" => CATALOG,
        "details" => DETAILS,
        "read" => READ,
        "troubleshooting" => TROUBLESHOOTING,
        _ => {
            return Some(Err(ErrorData::resource_not_found(
                format!("Built-in MCPMate resource guide topic '{topic}' was not found"),
                None,
            )));
        }
    };
    Some(Ok(markdown_result(uri, markdown)))
}

fn markdown_result(
    uri: &str,
    markdown: &str,
) -> ReadResourceResult {
    ReadResourceResult::new(vec![
        ResourceContents::text(markdown, uri).with_mime_type("text/markdown"),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn markdown_content(result: ReadResourceResult) -> (String, String) {
        assert_eq!(result.contents.len(), 1);
        match result.contents.into_iter().next().expect("guide content") {
            ResourceContents::TextResourceContents {
                uri, mime_type, text, ..
            } => {
                assert_eq!(mime_type.as_deref(), Some("text/markdown"));
                (uri, text)
            }
            ResourceContents::BlobResourceContents { .. } => panic!("expected Markdown text resource"),
            _ => panic!("expected known resource contents"),
        }
    }

    #[test]
    fn every_stable_topic_returns_its_concrete_uri_and_distinct_markdown() {
        for (topic, expected_heading) in [
            ("overview", "# Resource Guide Overview"),
            ("catalog", "# Catalog"),
            ("details", "# Details"),
            ("read", "# Standard Read"),
            ("troubleshooting", "# Troubleshooting"),
        ] {
            let uri = format!("mcpmate://resources/template/mcpmate/guide/{topic}");
            let result = try_read(&uri).expect("built-in guide URI").expect("stable guide topic");
            let (result_uri, markdown) = markdown_content(result);
            assert_eq!(result_uri, uri, "topic {topic}");
            assert!(markdown.contains(expected_heading), "topic {topic}");
        }
    }

    #[test]
    fn unknown_topic_is_not_found_but_non_builtin_uris_are_unhandled() {
        let error = try_read("mcpmate://resources/template/mcpmate/guide/unknown")
            .expect("built-in guide URI")
            .expect_err("unknown guide topic");
        assert_eq!(error.code, rmcp::model::ErrorCode::RESOURCE_NOT_FOUND);

        for uri in [
            "mcpmate://resources/template/mcpmate/guide/",
            "mcpmate://resources/template/mcpmate/guide/read/more",
            "mcpmate://resources/mcpmate/other",
        ] {
            assert!(try_read(uri).is_none(), "must defer non-exact URI {uri}");
        }
    }

    #[test]
    fn readme_explains_active_surface_and_broker_only_resource_sources() {
        let (_, readme) = markdown_content(
            try_read("mcpmate://resources/mcpmate/readme")
                .expect("built-in Readme URI")
                .expect("built-in Readme"),
        );
        assert!(readme.contains("Active Surface URIs come from standard lists"));
        assert!(readme.contains("BrokerOnly URIs come from Catalog/Details"));
        for topic in ["overview", "catalog", "details", "read", "troubleshooting"] {
            assert!(readme.contains(topic), "Readme must list topic {topic}");
        }
    }
}
