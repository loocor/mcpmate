// MCP Proxy API routes for Profile management
// Contains route definitions for Profile endpoints

use std::sync::Arc;

use aide::axum::{
    ApiRouter,
    routing::{delete_with, get_with, post_with},
};
use axum::{extract::DefaultBodyLimit, routing::post};

use super::AppState;
use crate::api::models::profile::{
    ProfileAuthoringSaveReq, ProfileAuthoringSaveResp, ProfileAuthoringViewResp, ProfileComponentListReq,
    ProfileComponentManageReq, ProfileDeleteReq, ProfileDetailsReq, ProfileDetailsResp, ProfileIdReq, ProfileListReq,
    ProfileListResp, ProfileManageReq, ProfileManageResp, ProfilePromptsListResp, ProfileResourceTemplatesListResp,
    ProfileResourcesListResp, ProfileServerManageReq, ProfileServerManageResp, ProfileServersListResp,
    ProfileToolsListResp, WorkflowGuideExternalDocumentReq, WorkflowGuideExternalDocumentResp,
    WorkflowGuidePackageFileDeleteReq, WorkflowGuidePreviewReq, WorkflowGuidePreviewResp, WorkflowGuideRepairReq,
    WorkflowGuideSaveReq, WorkflowGuideSaveResp, WorkflowGuideViewResp, WorkflowMaterialDeleteReq,
    WorkflowMaterialDeleteResp, WorkflowMaterialPreviewResp, WorkflowMaterialSaveReq, WorkflowMaterialSaveResp,
    WorkflowMaterialsReorderReq, WorkflowMaterialsReorderResp, WorkflowMaterialsViewResp,
    WorkflowSpecificationPreviewResp, WorkflowSpecificationViewResp, WorkflowStepMaterialsSaveReq,
    WorkflowStepMaterialsSaveResp,
};
use crate::api::models::resp::ProfileApiErrorResp;
use crate::api::models::token_estimate::{CapabilityTokenLedgerResponse, TokenEstimateQuery, TokenEstimateResponse};
use crate::{aide_wrapper_payload, aide_wrapper_query};
use crate::{api::handlers::profile, core::profile::materials::MAX_UPLOAD_BYTES};

/// Create Profile management routes
pub fn routes(state: Arc<AppState>) -> ApiRouter {
    ApiRouter::new()
        .api_route("/mcp/profile/list", get_with(profile_list_aide, profile_list_docs))
        .api_route(
            "/mcp/profile/authoring/view",
            get_with(profile_authoring_view_aide, profile_authoring_view_contract_docs),
        )
        .api_route(
            "/mcp/profile/details",
            get_with(profile_details_aide, profile_details_docs),
        )
        .api_route(
            "/mcp/profile/authoring/save",
            post_with(profile_authoring_save_aide, profile_authoring_save_contract_docs),
        )
        .api_route(
            "/mcp/profile/workflow/specification/view",
            get_with(
                workflow_specification_view_aide,
                workflow_specification_view_contract_docs,
            ),
        )
        .api_route(
            "/mcp/profile/workflow/specification/preview",
            get_with(
                workflow_specification_preview_aide,
                workflow_specification_preview_contract_docs,
            ),
        )
        .api_route(
            "/mcp/profile/workflow/guide/view",
            get_with(workflow_guide_view_aide, workflow_guide_view_docs),
        )
        .api_route(
            "/mcp/profile/workflow/guide/save",
            post_with(workflow_guide_save_aide, workflow_guide_save_docs),
        )
        .api_route(
            "/mcp/profile/workflow/guide/preview",
            post_with(workflow_guide_preview_aide, workflow_guide_preview_docs),
        )
        .api_route(
            "/mcp/profile/workflow/guide/external-document",
            get_with(
                workflow_guide_external_document_view_aide,
                workflow_guide_external_document_view_docs,
            ),
        )
        .api_route(
            "/mcp/profile/workflow/guide/package-files/upload",
            post(profile::workflow_guide_package_file_upload).into(),
        )
        .api_route(
            "/mcp/profile/workflow/guide/package-files/delete",
            delete_with(
                workflow_guide_package_file_delete_aide,
                workflow_guide_package_file_delete_docs,
            ),
        )
        .api_route(
            "/mcp/profile/workflow/guide/repair",
            post_with(workflow_guide_repair_aide, workflow_guide_repair_docs),
        )
        .api_route(
            "/mcp/profile/workflow/materials/view",
            get_with(workflow_materials_view_aide, workflow_materials_view_docs),
        )
        .api_route(
            "/mcp/profile/workflow/materials/save",
            post_with(workflow_material_save_aide, workflow_material_save_docs),
        )
        .api_route(
            "/mcp/profile/workflow/materials/delete",
            delete_with(workflow_material_delete_aide, workflow_material_delete_docs),
        )
        .api_route(
            "/mcp/profile/workflow/materials/reorder",
            post_with(workflow_materials_reorder_aide, workflow_materials_reorder_docs),
        )
        .api_route(
            "/mcp/profile/workflow/step-materials/save",
            post_with(workflow_step_materials_save_aide, workflow_step_materials_save_docs),
        )
        .api_route(
            "/mcp/profile/workflow/materials/upload",
            post(profile::workflow_material_upload).into(),
        )
        .api_route(
            "/mcp/profile/workflow/materials/replace",
            post(profile::workflow_material_replace).into(),
        )
        .api_route(
            "/mcp/profile/workflow/materials/preview",
            get_with(workflow_material_preview_aide, workflow_material_preview_docs),
        )
        .api_route(
            "/mcp/profile/delete",
            delete_with(profile_delete_aide, profile_delete_contract_docs),
        )
        .api_route(
            "/mcp/profile/manage",
            post_with(profile_manage_aide, profile_manage_contract_docs),
        )
        .api_route(
            "/mcp/profile/servers/list",
            get_with(servers_list_aide, servers_list_contract_docs),
        )
        .api_route(
            "/mcp/profile/servers/manage",
            post_with(server_manage_aide, server_manage_contract_docs),
        )
        .api_route(
            "/mcp/profile/tools/list",
            get_with(tools_list_aide, tools_list_contract_docs),
        )
        .api_route(
            "/mcp/profile/capabilities/manage",
            post_with(component_manage_aide, component_manage_contract_docs),
        )
        .api_route(
            "/mcp/profile/tools/manage",
            post_with(component_manage_aide, component_manage_contract_docs),
        )
        .api_route(
            "/mcp/profile/resources/list",
            get_with(resources_list_aide, resources_list_contract_docs),
        )
        .api_route(
            "/mcp/profile/resources/manage",
            post_with(component_manage_aide, component_manage_contract_docs),
        )
        .api_route(
            "/mcp/profile/resource-templates/list",
            get_with(resource_templates_list_aide, resource_templates_list_contract_docs),
        )
        .api_route(
            "/mcp/profile/resource-templates/manage",
            post_with(component_manage_aide, component_manage_contract_docs),
        )
        .api_route(
            "/mcp/profile/prompts/list",
            get_with(prompts_list_aide, prompts_list_contract_docs),
        )
        .api_route(
            "/mcp/profile/prompts/manage",
            post_with(component_manage_aide, component_manage_contract_docs),
        )
        .api_route(
            "/mcp/profile/token-estimate",
            get_with(token_estimate_aide, token_estimate_docs),
        )
        .api_route(
            "/mcp/profile/capability-token-ledger",
            get_with(capability_token_ledger_aide, capability_token_ledger_docs),
        )
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES + 1024 * 1024))
        .with_state(state)
}

// Generate aide-compatible wrappers for basic operations
aide_wrapper_query!(
    profile::profile_list,
    ProfileListReq,
    ProfileListResp,
    "List all profile with optional filtering"
);

aide_wrapper_query!(
    profile::workflow_guide_view,
    ProfileIdReq,
    WorkflowGuideViewResp,
    "Load a document-first Workflow Guide and its readable reference palette"
);

aide_wrapper_payload!(
    profile::workflow_guide_repair,
    WorkflowGuideRepairReq,
    WorkflowGuideSaveResp,
    "Repair the projected Workflow Guide Skill package"
);

aide_wrapper_query!(
    profile::workflow_guide_external_document_view,
    WorkflowGuideExternalDocumentReq,
    WorkflowGuideExternalDocumentResp,
    "Load an editable external Markdown reference document"
);

aide_wrapper_payload!(
    profile::workflow_guide_package_file_delete,
    WorkflowGuidePackageFileDeleteReq,
    WorkflowGuideSaveResp,
    "Delete an unreferenced Workflow Guide package file"
);

aide_wrapper_payload!(
    profile::workflow_guide_save,
    WorkflowGuideSaveReq,
    WorkflowGuideSaveResp,
    "Save a Workflow Guide and atomically project its managed Skill package"
);

aide_wrapper_payload!(
    profile::workflow_guide_preview,
    WorkflowGuidePreviewReq,
    WorkflowGuidePreviewResp,
    "Render the current Workflow Guide draft without saving it"
);

aide_wrapper_query!(
    profile::profile_details,
    ProfileDetailsReq,
    ProfileDetailsResp,
    "Get details for a specific profile"
);

aide_wrapper_query!(
    profile::profile_authoring_view,
    ProfileIdReq,
    ProfileAuthoringViewResp,
    "Load a Profile authoring view"
);

aide_wrapper_payload!(
    profile::profile_authoring_save,
    ProfileAuthoringSaveReq,
    ProfileAuthoringSaveResp,
    "Atomically create or update Profile authoring state"
);

aide_wrapper_query!(
    profile::workflow_specification_view,
    ProfileIdReq,
    WorkflowSpecificationViewResp,
    "Load a Workflow Profile specification"
);

aide_wrapper_query!(
    profile::workflow_specification_preview,
    ProfileIdReq,
    WorkflowSpecificationPreviewResp,
    "Preview a Workflow Profile specification without publishing an MCP surface"
);

aide_wrapper_query!(
    profile::workflow_materials_view,
    ProfileIdReq,
    WorkflowMaterialsViewResp,
    "Load Workflow Profile Materials"
);

aide_wrapper_payload!(
    profile::workflow_material_save,
    WorkflowMaterialSaveReq,
    WorkflowMaterialSaveResp,
    "Create or update a Workflow Material"
);

aide_wrapper_payload!(
    profile::workflow_material_delete,
    WorkflowMaterialDeleteReq,
    WorkflowMaterialDeleteResp,
    "Delete a Workflow Material"
);

aide_wrapper_payload!(
    profile::workflow_materials_reorder,
    WorkflowMaterialsReorderReq,
    WorkflowMaterialsReorderResp,
    "Replace the ordered Workflow Materials library"
);

aide_wrapper_payload!(
    profile::workflow_step_materials_save,
    WorkflowStepMaterialsSaveReq,
    WorkflowStepMaterialsSaveResp,
    "Replace ordered Material references for a Workflow Step"
);

aide_wrapper_query!(
    profile::workflow_material_preview,
    profile::materials::MaterialPreviewReq,
    WorkflowMaterialPreviewResp,
    "Preview a text Workflow Material"
);

aide_wrapper_payload!(
    profile::profile_delete,
    ProfileDeleteReq,
    ProfileManageResp,
    "Delete a profile"
);

// Generate aide-compatible wrappers for management operations
aide_wrapper_payload!(
    profile::profile_manage,
    ProfileManageReq,
    ProfileManageResp,
    "Manage profile operations (activate/deactivate)"
);

// Generate aide-compatible wrappers for component list operations
aide_wrapper_query!(
    profile::servers_list,
    ProfileComponentListReq,
    ProfileServersListResp,
    "List servers in a profile"
);

aide_wrapper_query!(
    profile::tools_list,
    ProfileComponentListReq,
    ProfileToolsListResp,
    "List tools in a profile"
);

aide_wrapper_query!(
    profile::resources_list,
    ProfileComponentListReq,
    ProfileResourcesListResp,
    "List resources in a profile"
);

aide_wrapper_query!(
    profile::resource_templates_list,
    ProfileComponentListReq,
    ProfileResourceTemplatesListResp,
    "List resource templates in a profile"
);

aide_wrapper_query!(
    profile::prompts_list,
    ProfileComponentListReq,
    ProfilePromptsListResp,
    "List prompts in a profile"
);

// Generate aide-compatible wrappers for server management
aide_wrapper_payload!(
    profile::server_manage,
    ProfileServerManageReq,
    ProfileServerManageResp,
    "Manage server operations (enable/disable servers in profile)"
);

// Generate aide-compatible wrappers for component management
aide_wrapper_payload!(
    profile::component_manage,
    ProfileComponentManageReq,
    ProfileServerManageResp,
    "Manage component operations (enable/disable tools, resources, prompts)"
);

// Generate aide-compatible wrapper for token estimation
aide_wrapper_query!(
    profile::token_estimate,
    TokenEstimateQuery,
    TokenEstimateResponse,
    "Estimate token savings for a profile"
);

aide_wrapper_query!(
    profile::capability_token_ledger,
    TokenEstimateQuery,
    CapabilityTokenLedgerResponse,
    "Per-capability JSON payloads for client-side tokenizer (profile trimming)"
);

fn with_profile_not_found(op: aide::transform::TransformOperation) -> aide::transform::TransformOperation {
    op.response::<404, axum::Json<ProfileApiErrorResp>>()
}

fn with_profile_conflict(op: aide::transform::TransformOperation) -> aide::transform::TransformOperation {
    with_profile_not_found(op).response::<409, axum::Json<ProfileApiErrorResp>>()
}

fn profile_authoring_view_contract_docs(
    op: aide::transform::TransformOperation
) -> aide::transform::TransformOperation {
    with_profile_not_found(profile_authoring_view_docs(op))
}

fn profile_authoring_save_contract_docs(
    op: aide::transform::TransformOperation
) -> aide::transform::TransformOperation {
    with_profile_conflict(profile_authoring_save_docs(op))
}

fn workflow_specification_view_contract_docs(
    op: aide::transform::TransformOperation
) -> aide::transform::TransformOperation {
    with_profile_not_found(workflow_specification_view_docs(op))
}

fn workflow_specification_preview_contract_docs(
    op: aide::transform::TransformOperation
) -> aide::transform::TransformOperation {
    with_profile_not_found(workflow_specification_preview_docs(op))
}

fn profile_delete_contract_docs(op: aide::transform::TransformOperation) -> aide::transform::TransformOperation {
    with_profile_conflict(profile_delete_docs(op))
}

fn profile_manage_contract_docs(op: aide::transform::TransformOperation) -> aide::transform::TransformOperation {
    with_profile_conflict(profile_manage_docs(op))
}

fn servers_list_contract_docs(op: aide::transform::TransformOperation) -> aide::transform::TransformOperation {
    with_profile_not_found(servers_list_docs(op))
}

fn server_manage_contract_docs(op: aide::transform::TransformOperation) -> aide::transform::TransformOperation {
    with_profile_conflict(server_manage_docs(op))
}

fn tools_list_contract_docs(op: aide::transform::TransformOperation) -> aide::transform::TransformOperation {
    with_profile_not_found(tools_list_docs(op))
}

fn resources_list_contract_docs(op: aide::transform::TransformOperation) -> aide::transform::TransformOperation {
    with_profile_not_found(resources_list_docs(op))
}

fn resource_templates_list_contract_docs(
    op: aide::transform::TransformOperation
) -> aide::transform::TransformOperation {
    with_profile_not_found(resource_templates_list_docs(op))
}

fn prompts_list_contract_docs(op: aide::transform::TransformOperation) -> aide::transform::TransformOperation {
    with_profile_not_found(prompts_list_docs(op))
}

fn component_manage_contract_docs(op: aide::transform::TransformOperation) -> aide::transform::TransformOperation {
    with_profile_conflict(component_manage_docs(op))
}

#[cfg(test)]
mod tests {
    use aide::{axum::ApiRouter, openapi::OpenApi};

    use super::*;

    #[test]
    fn profile_openapi_exposes_typed_not_found_and_conflict_responses() {
        let mut api = OpenApi::default();
        let _router = ApiRouter::<Arc<AppState>>::new()
            .api_route(
                "/mcp/profile/authoring/save",
                post_with(profile_authoring_save_aide, profile_authoring_save_contract_docs),
            )
            .api_route(
                "/mcp/profile/delete",
                delete_with(profile_delete_aide, profile_delete_contract_docs),
            )
            .api_route(
                "/mcp/profile/capabilities/manage",
                post_with(component_manage_aide, component_manage_contract_docs),
            )
            .finish_api_with(&mut api, |api| api);
        let document = serde_json::to_value(api).unwrap();

        for path in [
            "/mcp/profile/authoring/save",
            "/mcp/profile/delete",
            "/mcp/profile/capabilities/manage",
        ] {
            let operation = if path == "/mcp/profile/delete" {
                "delete"
            } else {
                "post"
            };
            for status in ["404", "409"] {
                let schema =
                    &document["paths"][path][operation]["responses"][status]["content"]["application/json"]["schema"];
                assert!(schema.get("$ref").is_some(), "{path} {status} must use a typed schema");
            }
        }
        let schemas = document["components"]["schemas"].as_object().unwrap();
        let coded_error = schemas
            .values()
            .find(|schema| schema["properties"]["error"]["$ref"].is_string())
            .expect("coded Profile error response schema");
        let error_ref = coded_error["properties"]["error"]["$ref"].as_str().unwrap();
        let error_name = error_ref.rsplit('/').next().unwrap();
        let error = &schemas[error_name];
        assert!(error["properties"]["code"].is_object());
        assert!(error["properties"]["details"].is_object());
    }
}
