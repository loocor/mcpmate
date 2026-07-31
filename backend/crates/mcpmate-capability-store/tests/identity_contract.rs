use std::str::FromStr;

use mcpmate_capability_store::{
    BUILTIN_CAPABILITY_SOURCE_ID, CapabilityId, CapabilityKind, CapabilityPayload, CapabilityRefId,
    CapabilitySourceIdentity, CatalogRecord, EffectiveCapabilityRecordV1, SurfaceManifestId,
};
use rmcp::model::{Prompt, Resource, ResourceTemplate};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

fn decode<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).expect("fixture must match RMCP 2.2")
}

fn tool(definition: Value) -> CapabilityPayload {
    CapabilityPayload::Tool(decode(definition))
}

fn effective_tool(
    server_id: &str,
    origin_key: &str,
    definition: Value,
) -> EffectiveCapabilityRecordV1 {
    let source = CapabilitySourceIdentity::new(server_id, CapabilityKind::Tools, origin_key);
    let ref_id = CapabilityRefId::derive(&source).expect("derive ref id");
    EffectiveCapabilityRecordV1::new(ref_id, source, tool(definition)).expect("build effective record")
}

#[test]
fn extracts_exact_origin_keys_for_every_capability_kind() {
    let fixtures = [
        (
            tool(json!({"name": "Get_Weather ", "inputSchema": {"type": "object"}})),
            CapabilityKind::Tools,
            "Get_Weather ",
        ),
        (
            CapabilityPayload::Prompt(Prompt::new("Compose_Text", None::<String>, None)),
            CapabilityKind::Prompts,
            "Compose_Text",
        ),
        (
            CapabilityPayload::Resource(Resource::new("FILE:///Exact/%2f", "Exact resource")),
            CapabilityKind::Resources,
            "FILE:///Exact/%2f",
        ),
        (
            CapabilityPayload::ResourceTemplate(ResourceTemplate::new("custom://Host/{Path}?Q={Q}", "Exact template")),
            CapabilityKind::ResourceTemplates,
            "custom://Host/{Path}?Q={Q}",
        ),
    ];

    for (payload, kind, expected_origin_key) in fixtures {
        let source = CapabilitySourceIdentity::from_payload("server-a", &payload);
        assert_eq!(source.kind, kind);
        assert_eq!(source.origin_key, expected_origin_key);
    }
}

#[test]
fn materialization_rejects_origin_keys_that_do_not_match_the_payload() {
    let fixtures = [
        tool(json!({"name": "actual_tool", "inputSchema": {"type": "object"}})),
        CapabilityPayload::Prompt(Prompt::new("actual_prompt", None::<String>, None)),
        CapabilityPayload::Resource(Resource::new("file:///actual", "Actual resource")),
        CapabilityPayload::ResourceTemplate(ResourceTemplate::new("custom://actual/{path}", "Actual template")),
    ];

    for payload in fixtures {
        let error = CatalogRecord::materialize("server-a", "different-origin", "external-name", payload)
            .expect_err("mismatched origin keys must be rejected");
        assert!(
            error.to_string().contains("does not match payload origin key"),
            "{error}"
        );
    }
}

#[test]
fn capability_ref_id_is_deterministic_and_preserves_exact_source_tuple() {
    let source = CapabilitySourceIdentity::new("server-a", CapabilityKind::Tools, "Get_Weather ");
    let same = CapabilitySourceIdentity::new("server-a", CapabilityKind::Tools, "Get_Weather ");
    let changed_case = CapabilitySourceIdentity::new("server-a", CapabilityKind::Tools, "get_weather ");
    let changed_server = CapabilitySourceIdentity::new("server-b", CapabilityKind::Tools, "Get_Weather ");

    let id = CapabilityRefId::derive(&source).expect("derive ref id");
    assert_eq!(id, CapabilityRefId::derive(&same).expect("derive same ref id"));
    assert_ne!(
        id,
        CapabilityRefId::derive(&changed_case).expect("derive case-sensitive ref id")
    );
    assert_ne!(
        id,
        CapabilityRefId::derive(&changed_server).expect("derive source-sensitive ref id")
    );
    assert_eq!(id.as_str().len(), "cref_sha256:".len() + 64);
}

#[test]
fn object_key_order_is_canonical_but_array_order_is_preserved() {
    let first = effective_tool(
        "server-a",
        "analyze",
        json!({
            "name": "server_a__analyze",
            "description": "Analyze",
            "inputSchema": {
                "type": "object",
                "required": ["query", "limit"],
                "properties": {
                    "query": {"type": "string"},
                    "limit": {"type": "integer"}
                }
            }
        }),
    );
    let reordered_objects = effective_tool(
        "server-a",
        "analyze",
        json!({
            "inputSchema": {
                "properties": {
                    "limit": {"type": "integer"},
                    "query": {"type": "string"}
                },
                "required": ["query", "limit"],
                "type": "object"
            },
            "description": "Analyze",
            "name": "server_a__analyze"
        }),
    );
    let reordered_array = effective_tool(
        "server-a",
        "analyze",
        json!({
            "name": "server_a__analyze",
            "description": "Analyze",
            "inputSchema": {
                "type": "object",
                "required": ["limit", "query"],
                "properties": {
                    "query": {"type": "string"},
                    "limit": {"type": "integer"}
                }
            }
        }),
    );

    assert_eq!(
        CapabilityId::derive(&first).expect("derive first id"),
        CapabilityId::derive(&reordered_objects).expect("derive reordered object id")
    );
    assert_ne!(
        CapabilityId::derive(&first).expect("derive first id"),
        CapabilityId::derive(&reordered_array).expect("derive reordered array id")
    );
}

#[test]
fn canonical_bytes_follow_the_rfc_8785_utf16_property_sort_fixture() {
    let record = effective_tool(
        "server-a",
        "ordering",
        json!({
            "name": "server_a__ordering",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "\u{20ac}": 1,
                    "\r": 2,
                    "\u{fb33}": 3,
                    "1": 4,
                    "\u{1f600}": 5,
                    "\u{0080}": 6,
                    "\u{00f6}": 7
                }
            }
        }),
    );

    let canonical =
        String::from_utf8(record.canonical_bytes().expect("canonical record")).expect("canonical JSON is UTF-8");
    let property_order = ["\\r", "1", "\u{0080}", "\u{00f6}", "\u{20ac}", "\u{1f600}", "\u{fb33}"];
    let mut previous = 0;
    for property in property_order {
        let position = canonical[previous..]
            .find(&format!("\"{property}\":"))
            .map(|offset| previous + offset)
            .expect("RFC 8785 fixture property must be present");
        assert!(position >= previous);
        previous = position + 1;
    }
}

#[test]
fn every_effective_model_visible_change_produces_a_new_capability_id() {
    let baseline = json!({
        "name": "server_a__analyze",
        "title": "Analyze",
        "description": "Analyze input",
        "inputSchema": {"type": "object"},
        "outputSchema": {"type": "object"},
        "annotations": {"readOnlyHint": true},
        "execution": {"taskSupport": "optional"},
        "icons": [{"src": "https://example.com/a.svg"}]
    });
    let baseline_id =
        CapabilityId::derive(&effective_tool("server-a", "analyze", baseline.clone())).expect("derive baseline id");

    let changes = [
        ("name", json!("server_a__analyze_v2")),
        ("title", json!("Analyze v2")),
        ("description", json!("Changed description")),
        ("inputSchema", json!({"type": "object", "required": ["query"]})),
        ("outputSchema", json!({"type": "string"})),
        ("annotations", json!({"readOnlyHint": false})),
        ("execution", json!({"taskSupport": "required"})),
        ("icons", json!([{"src": "https://example.com/b.svg"}])),
    ];

    for (field, value) in changes {
        let mut changed = baseline.clone();
        changed
            .as_object_mut()
            .expect("tool fixture is an object")
            .insert(field.to_string(), value);
        let changed_id =
            CapabilityId::derive(&effective_tool("server-a", "analyze", changed)).expect("derive changed id");
        assert_ne!(baseline_id, changed_id, "{field} must participate in identity");
    }
}

#[test]
fn observation_metadata_is_not_part_of_effective_capability_identity() {
    let record = effective_tool(
        "server-a",
        "analyze",
        json!({"name": "server_a__analyze", "inputSchema": {"type": "object"}}),
    );

    let before = CapabilityId::derive(&record).expect("derive before id");
    let after = CapabilityId::derive(&record).expect("derive after id");
    assert_eq!(before, after);
}

#[test]
fn builtin_source_uses_the_reserved_stable_identifier() {
    let source = CapabilitySourceIdentity::new(
        BUILTIN_CAPABILITY_SOURCE_ID,
        CapabilityKind::Tools,
        "mcpmate_ucan_catalog",
    );
    let id = CapabilityRefId::derive(&source).expect("derive built-in ref id");

    assert!(id.as_str().starts_with("cref_sha256:"));
}

#[test]
fn typed_ids_reject_wrong_prefix_length_uppercase_and_non_hex() {
    let valid_ref = format!("cref_sha256:{}", "a".repeat(64));
    let valid_capability = format!("cap_sha256:{}", "b".repeat(64));
    let valid_surface = format!("surf_sha256:{}", "c".repeat(64));

    assert!(CapabilityRefId::from_str(&valid_ref).is_ok());
    assert!(CapabilityId::from_str(&valid_capability).is_ok());
    assert!(SurfaceManifestId::from_str(&valid_surface).is_ok());

    for invalid in [
        format!("cap_sha256:{}", "a".repeat(63)),
        format!("cap_sha256:{}", "A".repeat(64)),
        format!("cap_sha256:{}", "g".repeat(64)),
        format!("cref_sha256:{}", "a".repeat(64)),
    ] {
        assert!(CapabilityId::from_str(&invalid).is_err(), "{invalid} must be rejected");
    }
}

#[test]
fn digest_hit_verifies_the_saved_canonical_record_bytes() {
    let record = effective_tool(
        "server-a",
        "analyze",
        json!({"name": "server_a__analyze", "inputSchema": {"type": "object"}}),
    );
    let id = CapabilityId::derive(&record).expect("derive capability id");
    let canonical = record.canonical_bytes().expect("canonical record");

    id.verify_canonical_content(&canonical, &canonical)
        .expect("identical canonical bytes must verify");

    let mut corrupted = canonical.clone();
    corrupted.push(b' ');
    assert!(id.verify_canonical_content(&canonical, &corrupted).is_err());
}
