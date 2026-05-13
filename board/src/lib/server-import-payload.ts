import type { ServerInstallDraft } from "../hooks/use-server-install-pipeline";
import { serializeMetaForApi } from "./api";

/** Body shape for `POST /api/mcp/servers/import` (matches backend `ServersImportReq`). */
export type ServersImportRequestBody = {
	mcpServers?: Record<string, unknown>;
	client_identifier?: string;
	selected_server_names?: string[];
	target_profile_id?: string | null;
	dry_run?: boolean;
};

export function buildClientServersImportRequest(init: {
	clientIdentifier: string;
	selectedServerNames: string[];
	targetProfileId?: string | null;
	dryRun?: boolean;
}): ServersImportRequestBody {
	const body: ServersImportRequestBody = {
		client_identifier: init.clientIdentifier,
		selected_server_names: init.selectedServerNames,
	};
	if (init.targetProfileId) {
		body.target_profile_id = init.targetProfileId;
	}
	if (init.dryRun) {
		body.dry_run = true;
	}
	return body;
}

/** Build `mcpServers` map from install wizard drafts (preview/import). */
export function buildMcpServersImportBodyFromDrafts(
	items: ServerInstallDraft[],
): Pick<ServersImportRequestBody, "mcpServers"> {
	const payload: Record<string, unknown> = {};
	for (const item of items) {
		const metaPayload = serializeMetaForApi(item.meta);
		const entry: Record<string, unknown> = {
			type: item.kind,
		};
		if (item.kind === "stdio" && item.command) {
			entry.command = item.command;
		}
		if (item.kind !== "stdio" && item.url) {
			entry.url = item.urlParams && Object.keys(item.urlParams).length
				? urlWithMergedSearchParams(item.url, item.urlParams)
				: item.url;
		}
		if (item.args?.length) {
			entry.args = item.args;
		}
		if (item.env && Object.keys(item.env).length) {
			entry.env = item.env;
		}
		if (item.registryServerId) {
			entry.registry_server_id = item.registryServerId;
		}
		if (metaPayload) {
			entry.meta = metaPayload;
		}
		payload[item.name] = entry;
	}
	return { mcpServers: payload };
}

export function buildDraftServersImportRequest(init: {
	drafts: ServerInstallDraft[];
	targetProfileId?: string | null;
	dryRun?: boolean;
}): ServersImportRequestBody {
	const body: ServersImportRequestBody = buildMcpServersImportBodyFromDrafts(
		init.drafts,
	);
	if (init.targetProfileId) {
		body.target_profile_id = init.targetProfileId;
	}
	if (init.dryRun) {
		body.dry_run = true;
	}
	return body;
}

const ABSOLUTE_HTTP_URL = /^https?:/i;

export function urlWithMergedSearchParams(
	baseUrl: string,
	params: Record<string, string>,
): string {
	const isAbsolute = ABSOLUTE_HTTP_URL.test(baseUrl);
	try {
		const parsed = new URL(
			baseUrl,
			isAbsolute ? undefined : "http://dummy.local",
		);
		for (const [key, value] of Object.entries(params)) {
			parsed.searchParams.set(key, value);
		}
		if (isAbsolute) {
			return parsed.toString();
		}
		// For relative URLs, use pathname + search to preserve existing query params
		const search = parsed.searchParams.toString();
		return search ? `${parsed.pathname}?${search}` : parsed.pathname;
	} catch {
		// Fallback: manually merge params, preserving existing query string
		const separator = baseUrl.includes("?") ? "&" : "?";
		const qs = new URLSearchParams(params).toString();
		return `${baseUrl}${separator}${qs}`;
	}
}
