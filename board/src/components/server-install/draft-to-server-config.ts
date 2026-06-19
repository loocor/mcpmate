import type { ServerInstallDraft } from "../../hooks/use-server-install-pipeline";
import type { MCPServerConfig } from "../../lib/types";

export function draftToServerConfig(
	draft: ServerInstallDraft,
	extra?: Partial<MCPServerConfig>,
): Partial<MCPServerConfig> {
	return {
		name: draft.name,
		kind: draft.kind,
		command: draft.kind === "stdio" ? draft.command : undefined,
		url: draft.kind === "stdio" ? undefined : draft.url,
		args: draft.args,
		env: draft.env,
		headers: draft.kind === "stdio" ? undefined : draft.headers,
		source: draft.source,
		meta: draft.meta,
		...extra,
	};
}
