use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

use crate::{
    config::{
        models::{ProfilePrompt, ProfileResource, ProfileServer, ProfileToolWithDetails},
        profile::{get_profile_servers, get_profile_tools, get_prompts_for_profile, get_resources_for_profile},
    },
    mcper::builtin::types::{PromptDetail, ResourceDetail, ServerDetail, ToolDetail},
};

pub struct ProfileCapabilityCounts {
    pub server_count: u32,
    pub tool_count: u32,
    pub prompt_count: u32,
    pub resource_count: u32,
}

pub struct ProfileDetailComponents {
    pub servers: Vec<ServerDetail>,
    pub tools: Vec<ToolDetail>,
    pub prompts: Vec<PromptDetail>,
    pub resources: Vec<ResourceDetail>,
}

pub async fn load_profile_capability_counts(
    pool: &Pool<Sqlite>,
    profile_id: &str,
) -> Result<ProfileCapabilityCounts> {
    let servers = get_profile_servers(pool, profile_id)
        .await
        .context("Failed to get profile servers")?;
    let tools = get_profile_tools(pool, profile_id)
        .await
        .context("Failed to get profile tools")?;
    let prompts = get_prompts_for_profile(pool, profile_id)
        .await
        .context("Failed to get profile prompts")?;
    let resources = get_resources_for_profile(pool, profile_id)
        .await
        .context("Failed to get profile resources")?;

    Ok(ProfileCapabilityCounts {
        server_count: servers.len() as u32,
        tool_count: tools.len() as u32,
        prompt_count: prompts.len() as u32,
        resource_count: resources.len() as u32,
    })
}

pub async fn load_profile_detail_components(
    pool: &Pool<Sqlite>,
    profile_id: &str,
) -> Result<ProfileDetailComponents> {
    let servers = get_profile_servers(pool, profile_id)
        .await
        .context("Failed to get profile servers")?;
    let tools = get_profile_tools(pool, profile_id)
        .await
        .context("Failed to get profile tools")?;
    let prompts = get_prompts_for_profile(pool, profile_id)
        .await
        .context("Failed to get profile prompts")?;
    let resources = get_resources_for_profile(pool, profile_id)
        .await
        .context("Failed to get profile resources")?;

    Ok(ProfileDetailComponents {
        servers: shape_server_details(pool, profile_id, servers).await?,
        tools: shape_tool_details(tools),
        prompts: shape_prompt_details(prompts),
        resources: shape_resource_details(resources),
    })
}

async fn shape_server_details(
    pool: &Pool<Sqlite>,
    profile_id: &str,
    servers: Vec<ProfileServer>,
) -> Result<Vec<ServerDetail>> {
    let mut details = Vec::with_capacity(servers.len());

    for server in servers {
        let server_id = server.server_id.clone();
        let name = match crate::config::server::crud::get_server_by_id(pool, &server_id).await {
            Ok(Some(server_model)) => server_model.name,
            Ok(None) => {
                tracing::warn!(server_id = %server_id, profile_id = %profile_id, "Server metadata missing, falling back to server ID");
                server_id.clone()
            }
            Err(error) => {
                tracing::warn!(error = %error, server_id = %server_id, profile_id = %profile_id, "Failed to load server metadata, falling back to server ID");
                server_id.clone()
            }
        };

        details.push(ServerDetail {
            association_id: server.id,
            server_id,
            name,
            enabled: server.enabled,
        });
    }

    details.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.server_id.cmp(&b.server_id)));
    Ok(details)
}

fn shape_tool_details(tools: Vec<ProfileToolWithDetails>) -> Vec<ToolDetail> {
    let mut details: Vec<ToolDetail> = tools
        .into_iter()
        .map(|tool| ToolDetail {
            association_id: tool.id,
            server_tool_id: tool.server_tool_id,
            server_id: tool.server_id,
            server_name: tool.server_name,
            tool_name: tool.tool_name,
            unique_name: tool.unique_name,
            description: tool.description,
            enabled: tool.enabled,
        })
        .collect();

    details.sort_by(|a, b| {
        a.server_name
            .cmp(&b.server_name)
            .then_with(|| a.tool_name.cmp(&b.tool_name))
            .then_with(|| a.unique_name.cmp(&b.unique_name))
    });
    details
}

fn shape_prompt_details(prompts: Vec<ProfilePrompt>) -> Vec<PromptDetail> {
    let mut details: Vec<PromptDetail> = prompts
        .into_iter()
        .map(|prompt| PromptDetail {
            association_id: prompt.id,
            server_id: prompt.server_id,
            server_name: prompt.server_name,
            prompt_name: prompt.prompt_name,
            enabled: prompt.enabled,
        })
        .collect();

    details.sort_by(|a, b| {
        a.server_name
            .cmp(&b.server_name)
            .then_with(|| a.prompt_name.cmp(&b.prompt_name))
    });
    details
}

fn shape_resource_details(resources: Vec<ProfileResource>) -> Vec<ResourceDetail> {
    let mut details: Vec<ResourceDetail> = resources
        .into_iter()
        .map(|resource| ResourceDetail {
            association_id: resource.id,
            server_id: resource.server_id,
            server_name: resource.server_name,
            resource_uri: resource.resource_uri,
            enabled: resource.enabled,
        })
        .collect();

    details.sort_by(|a, b| {
        a.server_name
            .cmp(&b.server_name)
            .then_with(|| a.resource_uri.cmp(&b.resource_uri))
    });
    details
}
