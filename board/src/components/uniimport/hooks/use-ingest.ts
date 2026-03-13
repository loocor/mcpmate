import { useCallback, useState } from "react";
import type { ServerInstallDraft } from "../../../hooks/use-server-install-pipeline";
import { normalizeIngestResult } from "../../../lib/install-normalizer";
import { notifyError } from "../../../lib/notify";
import type { ManualFormStateJson } from "../types";
import { DEFAULT_INGEST_MESSAGE } from "../types";

interface UseIngestProps {
	ingestEnabled: boolean;
	allowProgrammaticIngest: boolean;
	formStateRef: React.MutableRefObject<ManualFormStateJson>;
	buildFormValuesFromState: (state: ManualFormStateJson) => any;
	reset: (values?: any, options?: any) => void;
	onSubmitMultiple?: (drafts: ServerInstallDraft[]) => Promise<void> | void;
	onClose: () => void;
	messages?: {
		defaultMessage?: string;
		parsingDropped?: string;
		parsingPasted?: string;
		success?: string;
		noneDetectedError?: string;
		noneDetectedTitle?: string;
		noneDetectedDescription?: string;
		parseFailedFallback?: string;
		parseFailedTitle?: string;
	};
}

export function useIngest({
	ingestEnabled,
	allowProgrammaticIngest,
	formStateRef,
	buildFormValuesFromState,
	reset,
	onSubmitMultiple,
	onClose,
	messages,
}: UseIngestProps) {
	const resolvedMessages = {
		defaultMessage: DEFAULT_INGEST_MESSAGE,
		parsingDropped: "Parsing dropped text",
		parsingPasted: "Parsing pasted content",
		success: "Server configuration loaded successfully",
		noneDetectedError: "No servers detected in the input",
		noneDetectedTitle: "No servers detected",
		noneDetectedDescription:
			"We could not find any server definitions in the input.",
		parseFailedFallback: "Failed to parse input",
		parseFailedTitle: "Parsing failed",
		...messages,
	};
	const [isIngesting, setIsIngesting] = useState(false);
	const [ingestMessage, setIngestMessage] = useState<string>(
		resolvedMessages.defaultMessage,
	);
	const [ingestError, setIngestError] = useState<string | null>(null);
	const [isIngestSuccess, setIsIngestSuccess] = useState(false);
	const [isDropZoneCollapsed, setIsDropZoneCollapsed] = useState(
		!ingestEnabled,
	);
	const [isDragOver, setIsDragOver] = useState(false);

	const canIngestProgrammatically = ingestEnabled || allowProgrammaticIngest;

	// Reset ingest state to default
	const resetIngestState = useCallback(() => {
		setIngestError(null);
		setIsIngestSuccess(false);
		setIsDropZoneCollapsed(!ingestEnabled);
		setIsDragOver(false);
		setIngestMessage(resolvedMessages.defaultMessage);
	}, [ingestEnabled, resolvedMessages.defaultMessage]);

	const applySingleDraftToForm = useCallback(
		(draft: ServerInstallDraft) => {
			const nextState: ManualFormStateJson = {
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
			};

			nextState.name = draft.name ?? "";
			nextState.kind = draft.kind;
			nextState.meta = {
				description: draft.meta?.description ?? "",
				version: draft.meta?.version ?? "",
				websiteUrl: draft.meta?.websiteUrl ?? "",
				repository: {
					url: draft.meta?.repository?.url ?? "",
					source: draft.meta?.repository?.source ?? "",
					subfolder: draft.meta?.repository?.subfolder ?? "",
					id: draft.meta?.repository?.id ?? "",
				},
				icons: (draft.meta?.icons || [])
					.map((it) => ({
						src: String(
							(it as any).src ?? (it as any).url ?? (it as any).href ?? "",
						),
						mimeType:
							(it as any).mimeType ?? (it as any).mime_type ?? undefined,
						sizes: (it as any).sizes ?? (it as any).size ?? undefined,
					}))
					.filter((it) => it.src),
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
			} else if (draft.kind === "sse") {
				nextState.sse = {
					url: draft.url ?? "",
					headers: Object.entries(draft.headers || {}).map(([key, value]) => ({
						key,
						value,
					})),
					urlParams: Object.entries((draft as any)?.urlParams || {}).map(
						([key, value]) => ({ key, value: String(value ?? "") }),
					),
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

			formStateRef.current = nextState;
			reset(buildFormValuesFromState(nextState), {
				keepDirty: true,
				keepTouched: true,
				keepIsSubmitted: true,
				keepErrors: true,
				keepSubmitCount: true,
			});
		},
		[buildFormValuesFromState, reset],
	);

	const finalizeIngest = useCallback(
		async (drafts: ServerInstallDraft[]) => {
			if (!drafts.length) {
				setIngestError(resolvedMessages.noneDetectedError);
				notifyError(
					resolvedMessages.noneDetectedTitle,
					resolvedMessages.noneDetectedDescription,
				);
				return;
			}
			if (drafts.length === 1) {
				applySingleDraftToForm(drafts[0]);
				setIsIngestSuccess(true);
				setIsDropZoneCollapsed(true);
				setIngestMessage(resolvedMessages.success);
				setIngestError(null);
				return;
			}
			onSubmitMultiple?.(drafts);
			onClose();
		},
		[
			applySingleDraftToForm,
			onSubmitMultiple,
			onClose,
			resolvedMessages.noneDetectedError,
			resolvedMessages.noneDetectedTitle,
			resolvedMessages.noneDetectedDescription,
			resolvedMessages.success,
		],
	);

	const handleIngestPayload = useCallback(
		async (payload: {
			text?: string;
			buffer?: ArrayBuffer;
			fileName?: string;
		}) => {
			if (!canIngestProgrammatically) return;
			try {
				setIsIngesting(true);
				setIngestError(null);
				const drafts = await normalizeIngestResult(payload);
				await finalizeIngest(drafts);
			} catch (error) {
				const message =
					error instanceof Error
						? error.message
						: resolvedMessages.parseFailedFallback;
				setIngestError(message);
				notifyError(resolvedMessages.parseFailedTitle, message);
			} finally {
				setIsIngesting(false);
			}
		},
		[
			canIngestProgrammatically,
			finalizeIngest,
			resolvedMessages.parseFailedFallback,
			resolvedMessages.parseFailedTitle,
		],
	);

	return {
		isIngesting,
		ingestMessage,
		setIngestMessage,
		ingestError,
		setIngestError,
		isIngestSuccess,
		setIsIngestSuccess,
		isDropZoneCollapsed,
		setIsDropZoneCollapsed,
		isDragOver,
		setIsDragOver,
		canIngestProgrammatically,
		resetIngestState,
		applySingleDraftToForm,
		handleIngestPayload,
	};
}
