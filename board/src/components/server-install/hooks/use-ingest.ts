import { useCallback, useState } from "react";
import type { ServerInstallDraft } from "../../../hooks/use-server-install-pipeline";
import {
	normalizeIngestPayload,
	type ServerIngestPayload,
} from "../../../lib/install-normalizer";
import { notifyError } from "../../../lib/notify";
import type { ManualFormStateJson } from "../types";
import { DEFAULT_INGEST_MESSAGE } from "../types";
import { draftToFormState } from "./draft-to-form-state";

interface UseIngestProps {
	ingestEnabled: boolean;
	allowProgrammaticIngest: boolean;
	formStateRef: React.MutableRefObject<ManualFormStateJson>;
	buildFormValuesFromState: (state: ManualFormStateJson) => any;
	reset: (values?: any, options?: any) => void;
	sessionEpochRef?: React.MutableRefObject<number>;
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
	sessionEpochRef,
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
		setIsIngesting(false);
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

	const isSessionStale = useCallback(
		(epochAtStart: number) =>
			Boolean(sessionEpochRef && sessionEpochRef.current !== epochAtStart),
		[sessionEpochRef],
	);

	const finalizeIngest = useCallback(
		async (drafts: ServerInstallDraft[], epochAtStart: number) => {
			if (isSessionStale(epochAtStart)) {
				return;
			}
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
			isSessionStale,
			markIngestSuccess,
			onSubmitMultiple,
			resolvedMessages.noneDetectedError,
			resolvedMessages.noneDetectedTitle,
			resolvedMessages.noneDetectedDescription,
		],
	);

	const handleIngestPayload = useCallback(
		async (payload: ServerIngestPayload) => {
			if (!canIngestProgrammatically) return;
			const epochAtStart = sessionEpochRef?.current ?? 0;
			try {
				setIsIngesting(true);
				setIngestError(null);
				const drafts = await normalizeIngestPayload(payload);
				if (isSessionStale(epochAtStart)) {
					return;
				}
				await finalizeIngest(drafts, epochAtStart);
			} catch (error) {
				if (isSessionStale(epochAtStart)) {
					return;
				}
				const message =
					error instanceof Error
						? error.message
						: resolvedMessages.parseFailedFallback;
				setIngestError(message);
				notifyError(resolvedMessages.parseFailedTitle, message);
			} finally {
				if (!isSessionStale(epochAtStart)) {
					setIsIngesting(false);
				}
			}
		},
		[
			canIngestProgrammatically,
			finalizeIngest,
			isSessionStale,
			sessionEpochRef,
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
