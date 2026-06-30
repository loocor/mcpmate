use std::sync::Arc;

use aide::axum::{
    ApiRouter,
    routing::{get_with, post_with},
};

use super::AppState;
use crate::api::handlers::llm;
use crate::api::models::llm::*;
use crate::{aide_wrapper, aide_wrapper_payload};

pub fn routes(state: Arc<AppState>) -> ApiRouter {
    aide_wrapper!(llm::list_providers, Vec<LlmProviderData>, "List all LLM providers");
    aide_wrapper_payload!(
        llm::create_provider,
        LlmProviderCreateReq,
        LlmProviderData,
        "Create an LLM provider"
    );
    aide_wrapper_payload!(
        llm::update_provider,
        LlmProviderUpdateReq,
        LlmProviderData,
        "Update an LLM provider"
    );
    aide_wrapper_payload!(llm::delete_provider, LlmProviderIdReq, (), "Delete an LLM provider");
    aide_wrapper_payload!(
        llm::test_provider,
        LlmProviderIdReq,
        LlmConnectivityResult,
        "Test LLM provider connectivity"
    );
    aide_wrapper_payload!(
        llm::list_models,
        LlmProviderIdReq,
        LlmModelsData,
        "List available models for a provider"
    );
    aide_wrapper_payload!(
        llm::list_models_for_config,
        LlmProviderModelPreviewReq,
        LlmModelsData,
        "List available models for an unsaved provider config"
    );
    aide_wrapper_payload!(
        llm::set_default_provider,
        LlmProviderIdReq,
        (),
        "Set default LLM provider"
    );
    aide_wrapper!(
        llm::get_default_provider,
        Option<LlmProviderData>,
        "Get default LLM provider"
    );

    ApiRouter::new()
        .api_route("/llm/providers", get_with(list_providers_aide, list_providers_docs))
        .api_route(
            "/llm/providers/create",
            post_with(create_provider_aide, create_provider_docs),
        )
        .api_route(
            "/llm/providers/update",
            post_with(update_provider_aide, update_provider_docs),
        )
        .api_route(
            "/llm/providers/delete",
            post_with(delete_provider_aide, delete_provider_docs),
        )
        .api_route("/llm/providers/test", post_with(test_provider_aide, test_provider_docs))
        .api_route("/llm/providers/models", post_with(list_models_aide, list_models_docs))
        .api_route(
            "/llm/providers/models/preview",
            post_with(list_models_for_config_aide, list_models_for_config_docs),
        )
        .api_route(
            "/llm/providers/default",
            get_with(get_default_provider_aide, get_default_provider_docs),
        )
        .api_route(
            "/llm/providers/set-default",
            post_with(set_default_provider_aide, set_default_provider_docs),
        )
        .with_state(state)
}
