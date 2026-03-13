import { useCallback, useRef, useState } from "react";
import type { ManualFormStateJson, ManualServerFormValues } from "../types";
import { cloneArgs, cloneKeyValuePairs } from "../types";

export function useFormState() {
	const [activeTab, setActiveTab] = useState<"core" | "meta">("core");
	const [viewMode, setViewMode] = useState<"form" | "json">("form");
	const [jsonText, setJsonText] = useState<string>(
		`{
  "mcpServers": {
    "example": {
      "type": "stdio",
      "command": "uvx",
      "args": []
    }
  }
}`,
	);
	const [jsonError, setJsonError] = useState<string | null>(null);

	const createInitialFormState = useCallback(
		(): ManualFormStateJson => ({
			name: "",
			kind: "stdio",
			meta: {
				description: "",
				version: "",
				websiteUrl: "",
				repository: {
					url: "",
					source: "",
					subfolder: "",
					id: "",
				},
				icons: [],
			},
			stdio: { command: "", args: [], env: [] },
			sse: { url: "", headers: [] },
			streamable_http: { url: "", headers: [] },
		}),
		[],
	);

	const formStateRef = useRef<ManualFormStateJson>(createInitialFormState());
	const isRestoringRef = useRef(false);
	const lastInitialDraftRef = useRef<string | null>(null);

	const buildFormValuesFromState = useCallback(
		(state: ManualFormStateJson): ManualServerFormValues => {
			const commonMeta = state.meta;
			const base: ManualServerFormValues = {
				name: state.name ?? "",
				kind: state.kind,
				command: "",
				url: "",
				args: [],
				env: [],
				headers: [],
				meta_description: commonMeta.description,
				meta_version: commonMeta.version,
				meta_website_url: commonMeta.websiteUrl,
				meta_repository_url: commonMeta.repository.url,
				meta_repository_source: commonMeta.repository.source,
				meta_repository_subfolder: commonMeta.repository.subfolder,
				meta_repository_id: commonMeta.repository.id,
			};

			if (state.kind === "stdio") {
				base.command = state.stdio.command ?? "";
				base.args = cloneArgs(state.stdio.args);
				base.env = cloneKeyValuePairs(state.stdio.env);
			} else if (state.kind === "sse") {
				base.url = state.sse.url ?? "";
				base.headers = cloneKeyValuePairs(state.sse.headers);
				(base as any).urlParams = cloneKeyValuePairs(state.sse.urlParams);
			} else {
				base.url = state.streamable_http.url ?? "";
				base.headers = cloneKeyValuePairs(state.streamable_http.headers);
				(base as any).urlParams = cloneKeyValuePairs(
					state.streamable_http.urlParams,
				);
			}

			return base;
		},
		[],
	);

	return {
		activeTab,
		setActiveTab,
		viewMode,
		setViewMode,
		jsonText,
		setJsonText,
		jsonError,
		setJsonError,
		formStateRef,
		isRestoringRef,
		lastInitialDraftRef,
		createInitialFormState,
		buildFormValuesFromState,
	};
}
