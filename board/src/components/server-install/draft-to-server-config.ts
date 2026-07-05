import type { ServerInstallDraft } from "../../hooks/use-server-install-pipeline";
import { urlWithMergedSearchParams } from "../../lib/server-import-payload";
import type { MCPServerConfig } from "../../lib/types";

function hasEntries(value?: Record<string, string>): value is Record<string, string> {
	return Boolean(value && Object.keys(value).length > 0);
}

export function draftToServerConfig(
	draft: ServerInstallDraft,
	extra?: Partial<MCPServerConfig>,
): Partial<MCPServerConfig> {
	const url =
		draft.kind === "stdio" || !draft.url
			? undefined
			: hasEntries(draft.urlParams)
				? urlWithMergedSearchParams(draft.url, draft.urlParams)
				: draft.url;

	return {
		name: draft.name,
		kind: draft.kind,
		command: draft.kind === "stdio" ? draft.command : undefined,
		url,
		args: draft.args,
		env: draft.env,
		headers: draft.kind === "stdio" ? undefined : draft.headers,
		source: draft.source,
		meta: draft.meta,
		...extra,
	};
}
