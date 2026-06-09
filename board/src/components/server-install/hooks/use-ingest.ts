import { useCallback, useState } from "react";
import type { ServerInstallDraft } from "../../../hooks/use-server-install-pipeline";
import { normalizeIngestResult } from "../../../lib/install-normalizer";
import { notifyError } from "../../../lib/notify";
import type { ManualFormStateJson } from "../types";
import { DEFAULT_INGEST_MESSAGE } from "../types";
import { draftToFormState } from "./draft-to-form-state";

type IngestPayload = {
	text?: string;
	buffer?: ArrayBuffer;
	fileName?: string;
};

interface UseIngestProps {
	ingestEnabled: boolean;
	allowProgrammaticIngest: boolean;
	formStateRef: React.MutableRefObject<ManualFormStateJson>;
	buildFormValuesFromState: (state: ManualFormStateJson) => any;
	reset: (values?: any, options?: any) => void;
	onSubmitMultiple?: (drafts: ServerInstallDraft[]) => Promise<void> | void;
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

	const markIngestSuccess = useCallback(() => {
		setIngestError(null);
		setIsIngestSuccess(true);
		setIsDropZoneCollapsed(true);
		setIngestMessage(resolvedMessages.success);
	}, [resolvedMessages.success]);

	const applySingleDraftToForm = useCallback(
		(draft: ServerInstallDraft) => {
			const nextState = draftToFormState(draft);

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
				markIngestSuccess();
				return;
			}
			markIngestSuccess();
			await onSubmitMultiple?.(drafts);
		},
		[
			applySingleDraftToForm,
			markIngestSuccess,
			onSubmitMultiple,
			resolvedMessages.noneDetectedError,
			resolvedMessages.noneDetectedTitle,
			resolvedMessages.noneDetectedDescription,
		],
	);

	const handleIngestPayload = useCallback(
		async (payload: {
			text?: string;
			buffer?: ArrayBuffer;
			fileName?: string;
			payloads?: IngestPayload[];
		}) => {
			if (!canIngestProgrammatically) return;
			try {
				setIsIngesting(true);
				setIngestError(null);
				const payloads = payload.payloads ?? [payload];
				const draftGroups = await Promise.all(
					payloads.map((item) => normalizeIngestResult(item)),
				);
				const drafts = draftGroups.flat();
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
		markIngestSuccess,
		applySingleDraftToForm,
		handleIngestPayload,
	};
}
