import { normalizeIconList } from "../../../lib/install-normalizer";
import type { ServerInstallDraft } from "../../../hooks/use-server-install-pipeline";
import type { ManualFormStateJson } from "../types";

export function draftToFormState(draft: ServerInstallDraft): ManualFormStateJson {
	const nextState: ManualFormStateJson = {
		name: draft.name ?? "",
		kind: draft.kind,
		meta: {
			description: draft.meta?.description ?? "",
			version: draft.meta?.version ?? "",
			websiteUrl: draft.meta?.websiteUrl ?? "",
			repository: {
				url: draft.meta?.repository?.url ?? "",
				source: draft.meta?.repository?.source ?? "",
				subfolder: draft.meta?.repository?.subfolder ?? "",
				id: draft.meta?.repository?.id ?? "",
			},
			icons: normalizeIconList(draft.meta?.icons).map((icon) => ({
			src: icon.src,
			mimeType: icon.mimeType ?? undefined,
			sizes: icon.sizes ?? undefined,
		})),
		},
		stdio: { command: "", args: [], env: [] },
		streamable_http: { url: "", headers: [], urlParams: [] },
	};

	if (draft.kind === "stdio") {
		nextState.stdio = {
			command: draft.command ?? "",
			args: (draft.args || []).map((value) => ({ value })),
			env: Object.entries(draft.env || {}).map(([key, value]) => ({
				key,
				value,
			})),
		};
	} else {
		nextState.streamable_http = {
			url: draft.url ?? "",
			headers: Object.entries(draft.headers || {}).map(([key, value]) => ({
				key,
				value,
			})),
			urlParams: Object.entries((draft as any)?.urlParams || {}).map(
				([key, value]) => ({ key, value: String(value ?? "") }),
			),
		};
	}

	return nextState;
}
