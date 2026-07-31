import type {
	CapabilityAuthenticationFailure,
	CapabilityBatchFailure,
	CapabilityBatchLists,
} from "../../lib/api";
import { getCapabilityBatchFailures } from "../../lib/api";
import type {
	ConfigSuitPrompt,
	ConfigSuitResource,
	ConfigSuitResourceTemplate,
	ConfigSuitTool,
	UnifyDirectCapabilityRefs,
} from "../../lib/types";

export type DirectCapabilityProjection = {
	tools: ConfigSuitTool[];
	prompts: ConfigSuitPrompt[];
	resources: ConfigSuitResource[];
	templates: ConfigSuitResourceTemplate[];
	failures: CapabilityBatchFailure[];
	failureState: "none" | "partial" | "complete";
	authentication: CapabilityAuthenticationFailure | null;
};

export type DirectAuthenticationNotice = {
	titleKey: string;
	titleDefault: string;
	descriptionKey: string;
	descriptionDefault: string;
};

export function getDirectAuthenticationNotice(
	failure: CapabilityAuthenticationFailure | null,
): DirectAuthenticationNotice | null {
	if (!failure) return null;

	if (failure.code === "insufficient_scope") {
		return {
			titleKey:
				"servers:detail.capabilityList.authentication.insufficientScope.title",
			titleDefault: "Additional authorization scope required",
			descriptionKey:
				"servers:detail.capabilityList.authentication.insufficientScope.description",
			descriptionDefault:
				"The current authorization does not grant the scope required to discover capabilities.",
		};
	}

	if (failure.code === "forbidden") {
		return {
			titleKey: "servers:detail.capabilityList.authentication.forbidden.title",
			titleDefault: "Authorization rejected",
			descriptionKey:
				"servers:detail.capabilityList.authentication.forbidden.description",
			descriptionDefault:
				"The upstream denied capability access for the current authorization.",
		};
	}

	return {
		titleKey: "servers:detail.capabilityList.authentication.required.title",
		titleDefault: "Authentication required",
		descriptionKey:
			"servers:detail.capabilityList.authentication.required.description",
		descriptionDefault:
			"Configure authentication before discovering this server's capabilities.",
	};
}

export function getCapabilityId(
	item: Record<string, unknown>,
	keys: string[],
): string | null {
	for (const key of keys) {
		const value = item[key];
		if (typeof value === "string" && value.trim()) {
			return value;
		}
	}
	return null;
}

export function projectDirectCapabilities(
	lists: CapabilityBatchLists,
	selectedCapabilityRefs: UnifyDirectCapabilityRefs,
	serverId: string,
	serverName: string,
): DirectCapabilityProjection {
	const selectedToolSet = new Set(selectedCapabilityRefs.tool_refs ?? []);
	const selectedPromptSet = new Set(selectedCapabilityRefs.prompt_refs ?? []);
	const selectedResourceSet = new Set(selectedCapabilityRefs.resource_refs ?? []);
	const selectedTemplateSet = new Set(selectedCapabilityRefs.template_refs ?? []);
	const rawTools = lists.tools.items as Array<Record<string, unknown>>;
	const rawPrompts = lists.prompts.items as Array<Record<string, unknown>>;
	const rawResources = lists.resources.items as Array<Record<string, unknown>>;
	const rawTemplates = lists.templates.items as Array<Record<string, unknown>>;
	const failures = getCapabilityBatchFailures(lists);
	const failureState =
		failures.length === 0
			? "none"
			: failures.length === 4
				? "complete"
				: "partial";

	return {
		tools: rawTools.flatMap((tool) => {
			const toolName = String(tool["tool_name"] ?? tool["name"] ?? "");
			const capabilityRefId = getCapabilityId(tool, ["ref_id"]);
			if (!capabilityRefId) return [];
			return {
				...tool,
				id: capabilityRefId,
				server_id: serverId,
				server_name: serverName,
				tool_name: toolName,
				unique_name: String(tool["unique_name"] ?? tool["name"] ?? toolName),
				enabled: selectedToolSet.has(capabilityRefId),
				allowed_operations: [],
			};
		}),
		prompts: rawPrompts.flatMap((prompt) => {
			const promptName = String(prompt["prompt_name"] ?? prompt["name"] ?? "");
			const capabilityRefId = getCapabilityId(prompt, ["ref_id"]);
			if (!capabilityRefId) return [];
			return {
				...prompt,
				id: capabilityRefId,
				server_id: serverId,
				server_name: serverName,
				prompt_name: promptName,
				unique_name: String(
					prompt["unique_name"] ?? prompt["name"] ?? promptName,
				),
				enabled: selectedPromptSet.has(capabilityRefId),
				allowed_operations: [],
			};
		}),
		resources: rawResources.flatMap((resource) => {
			const resourceUri = String(
				resource["resource_uri"] ?? resource["uri"] ?? "",
			);
			const capabilityRefId = getCapabilityId(resource, ["ref_id"]);
			if (!capabilityRefId) return [];
			return {
				...resource,
				id: capabilityRefId,
				server_id: serverId,
				server_name: serverName,
				resource_uri: resourceUri,
				unique_uri: String(
					resource["unique_uri"] ?? resource["uri"] ?? resourceUri,
				),
				enabled: selectedResourceSet.has(capabilityRefId),
				allowed_operations: [],
			};
		}),
		templates: rawTemplates.flatMap((template) => {
			const uriTemplate = String(
				template["uri_template"] ?? template["template"] ?? "",
			);
			const capabilityRefId = getCapabilityId(template, ["ref_id"]);
			if (!capabilityRefId) return [];
			return {
				...template,
				id: capabilityRefId,
				server_id: serverId,
				server_name: serverName,
				uri_template: uriTemplate,
				unique_uri_template: String(
					template["unique_uri_template"] ??
						template["uriTemplate"] ??
						uriTemplate,
				),
				enabled: selectedTemplateSet.has(capabilityRefId),
				allowed_operations: [],
			};
		}),
		failures,
		failureState,
		authentication: lists.authentication ?? null,
	};
}
