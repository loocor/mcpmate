import { useCallback, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
	extractImportStats,
	serversApi,
  systemApi,
	type ServersImportResponse,
	type ServersPreviewResponse,
} from "../lib/api";
import {
	buildDraftServersImportRequest,
	urlWithMergedSearchParams,
} from "../lib/server-import-payload";
import type { ImportStats } from "../lib/api";
import { notifyError, notifyInfo, notifySuccess } from "../lib/notify";
import { formatNameList, summarizeSkipped } from "../lib/server-import-utils";
import type { ServerMetaInfo, ServerSource } from "../lib/types";

export type InstallSource = "manual" | "ingest" | "market";
export type WizardStep = "form" | "preview" | "result";

export interface ServerInstallDraft {
	name: string;
	/** Original imported label when the user explicitly applies a namespace suggestion. */
	originalName?: string;
	serverId?: string;
	kind: "stdio" | "sse" | "streamable_http";
	command?: string;
	args?: string[];
	env?: Record<string, string>;
	url?: string;
	source?: ServerSource;
	headers?: Record<string, string>;
	urlParams?: Record<string, string>;
	meta?: ServerMetaInfo;
}

export interface WizardImportResult {
	success: boolean;
	summary?: { imported_count: number; skipped_count: number };
	servers?: Record<string, { id: string; status: string }>;
	error?: string;
}

interface UseServerInstallPipelineOptions {
	onImported?: () => void;
}

function hasEntries(
	value?: Record<string, string>,
): value is Record<string, string> {
	return Boolean(value && Object.keys(value).length > 0);
}

export function useServerInstallPipeline(
	opts: UseServerInstallPipelineOptions = {},
) {
	const { t, i18n } = useTranslation("servers");
	const [isDrawerOpen, setDrawerOpen] = useState(false);
	const [drafts, setDrafts] = useState<ServerInstallDraft[]>([]);
	const [selectedDraftNames, setSelectedDraftNames] = useState<string[]>([]);
	const [source, setSource] = useState<InstallSource | null>(null);
	const [isPreviewLoading, setPreviewLoading] = useState(false);
  const [previewState, setPreviewState] =
    useState<ServersPreviewResponse | null>(null);
	const [previewError, setPreviewError] = useState<string | null>(null);
	const [isImporting, setImporting] = useState(false);
	const [currentStep, setCurrentStep] = useState<WizardStep>("form");
	const [importResult, setImportResult] = useState<WizardImportResult | null>(
		null,
	);
	const [targetProfileId, setTargetProfileId] = useState<string | null>(null);
  const [dryRunResult, setDryRunResult] =
    useState<ServersImportResponse | null>(null);
	const [dryRunStats, setDryRunStats] = useState<ImportStats | null>(null);
	const [dryRunWarning, setDryRunWarning] = useState<string | null>(null);
	const [isDryRunLoading, setDryRunLoading] = useState(false);
	const [dryRunError, setDryRunError] = useState<string | null>(null);

	const previewGenerationRef = useRef(0);

	const clearResults = useCallback(() => {
		previewGenerationRef.current++;
		setPreviewLoading(false);
		setPreviewState(null);
		setPreviewError(null);
		setImportResult(null);
		setDryRunResult(null);
		setDryRunStats(null);
		setDryRunWarning(null);
		setDryRunError(null);
	}, []);

	const reset = useCallback(() => {
		setDrawerOpen(false);
		setDrafts([]);
		setSelectedDraftNames([]);
		setSource(null);
		clearResults();
		setImporting(false);
		setCurrentStep("form");
		setTargetProfileId(null);
		setDryRunLoading(false);
	}, [clearResults]);

	const selectedDraftNameSet = useMemo(
		() => new Set(selectedDraftNames),
		[selectedDraftNames],
	);
	const selectedDrafts = useMemo(
		() => drafts.filter((draft) => selectedDraftNameSet.has(draft.name)),
		[drafts, selectedDraftNameSet],
	);

	const setDraftCollection = useCallback(
		(items: ServerInstallDraft[], origin: InstallSource | null) => {
			setDrafts(items);
			setSelectedDraftNames(items.map((item) => item.name));
			setSource(origin);
			clearResults();
		},
		[clearResults],
	);

  const updateDraft = useCallback(
    (draft: ServerInstallDraft, previousName?: string) => {
		const matchName = previousName ?? draft.name;
		setDrafts((current) =>
			current.map((item) => (item.name === matchName ? draft : item)),
		);
		setSelectedDraftNames((current) =>
			current.map((name) => (name === matchName ? draft.name : name)),
		);
		clearResults();
    },
    [clearResults],
  );

	const updateSelectedDraftNames = useCallback((names: string[]) => {
		setSelectedDraftNames(names);
		setImportResult(null);
		setDryRunResult(null);
		setDryRunStats(null);
		setDryRunWarning(null);
		setDryRunError(null);
	}, []);

	const buildPreviewPayload = useCallback((items: ServerInstallDraft[]) => {
		return {
			include_details: true,
			servers: items.map((item) => ({
				name: item.name,
				server_id: item.serverId ?? null,
				kind: item.kind,
				command: item.kind === "stdio" ? (item.command ?? null) : null,
				args: item.args?.length ? item.args : null,
				env: hasEntries(item.env) ? item.env : null,
				url:
					item.kind !== "stdio" && item.url
						? hasEntries(item.urlParams)
							? urlWithMergedSearchParams(item.url, item.urlParams)
							: item.url
						: null,
				headers:
					item.kind !== "stdio" && hasEntries(item.headers)
						? item.headers
						: null,
			})),
		};
	}, []);

	const previewDrafts = useCallback(
		async (items: ServerInstallDraft[]) => {
			if (!items.length) {
				notifyError(
					"No servers selected",
					"Select at least one server to preview.",
				);
				return;
			}

			const gen = ++previewGenerationRef.current;
			setPreviewState(null);
			setPreviewError(null);
			setPreviewLoading(true);

			try {
				const payload = buildPreviewPayload(items);
        const settings = await systemApi.getSettings();
				const result = await serversApi.previewServers({
					...payload,
          timeout_ms: settings.inspector_timeout_ms,
				});
				if (gen !== previewGenerationRef.current) return;
				setPreviewState(result);
			} catch (error) {
				if (gen !== previewGenerationRef.current) return;
				const message =
					error instanceof Error ? error.message : "Preview request failed";
				setPreviewError(message);
				notifyError("Preview failed", message);
			} finally {
				if (gen === previewGenerationRef.current) {
					setPreviewLoading(false);
				}
			}
		},
		[buildPreviewPayload],
	);

	const begin = useCallback(
		async (items: ServerInstallDraft[], origin: InstallSource) => {
			if (!items.length) {
				notifyError(
					"No servers detected",
					"Provide at least one server to preview",
				);
				return;
			}

			setDraftCollection(items, origin);
			setDrawerOpen(true);
			setCurrentStep("preview");
			await previewDrafts(items);
		},
		[previewDrafts, setDraftCollection],
	);

	const performDryRun = useCallback(async () => {
		if (!selectedDrafts.length) {
			setDryRunError(
				t("wizard.result.validationNoSelection", {
					defaultValue: "Select at least one server to validate.",
				}),
			);
			return;
		}
		const generation = previewGenerationRef.current;
		try {
			setDryRunLoading(true);
			setDryRunError(null);
			setDryRunStats(null);
			setDryRunWarning(null);
			const requestBody = buildDraftServersImportRequest({
				drafts,
				selectedDraftNames,
				targetProfileId,
				dryRun: true,
			});
			const result = await serversApi.importServers(requestBody);
			if (generation !== previewGenerationRef.current) {
				return;
			}
			setDryRunResult(result);
			const stats = extractImportStats(result);
			setDryRunStats(stats);

			const skipSummary = summarizeSkipped(stats.skippedDetails, t);
			const skipFallback = formatNameList(stats.skippedServers, t);
			if (stats.skippedCount > 0) {
				const baseKey =
					stats.skippedCount === 1
						? "wizard.result.skipSummary.baseSingle"
						: "wizard.result.skipSummary.baseMultiple";
				const base = t(baseKey, { count: stats.skippedCount });
				const detail = skipSummary || skipFallback;
				const combined = detail
					? t("wizard.result.skipSummary.withDetail", { base, detail })
					: base;
				const suffix =
					stats.importedCount === 0 && stats.failedCount === 0
						? ` ${t("wizard.result.skipSummary.suffixAlreadyInstalled")}`
						: "";
				setDryRunWarning(`${combined}${suffix}`.trim());
			} else {
				setDryRunWarning(null);
			}

			// Check if dry-run indicates any issues
			if (stats.failedCount > 0) {
				const failedNames = formatNameList(stats.failedServers, t);
				setDryRunError(
					t("wizard.result.failedSummary", {
						count: stats.failedCount,
						servers:
							failedNames ||
							t("wizard.result.failedSummaryFallback", {
								count: stats.failedCount,
							}),
					}),
				);
			} else {
				setDryRunError(null);
			}
		} catch (error) {
			if (generation !== previewGenerationRef.current) {
				return;
			}
			setDryRunStats(null);
			setDryRunWarning(null);
      const message =
        error instanceof Error ? error.message : String(error ?? "");
			setDryRunError(
				message ||
					t("wizard.result.validationErrorGeneric", {
						defaultValue: "Failed to validate import",
					}),
			);
		} finally {
			if (generation === previewGenerationRef.current) {
				setDryRunLoading(false);
			}
		}
	}, [
		drafts,
		selectedDraftNames,
		selectedDrafts.length,
		targetProfileId,
		t,
		i18n.language,
	]);

	const confirmImport = useCallback(
		async (overrideTargetProfileId?: string | null) => {
			if (!selectedDrafts.length) {
				notifyError(
					"No servers selected",
					"Select at least one server before importing.",
				);
				return false;
			}
			// Allow callers (e.g., the install wizard's `handleImport`) to
			// provide a freshly resolved target profile id — useful when
			// auto-add is enabled but the pipeline state has not been updated
			// via `setTargetProfileId` yet (state updates would be stale
			// within the same handler tick).
			const effectiveTargetProfileId =
				overrideTargetProfileId !== undefined
					? overrideTargetProfileId
					: targetProfileId;
			try {
				setImporting(true);
				setCurrentStep("result");
				const requestBody = buildDraftServersImportRequest({
					drafts,
					selectedDraftNames,
					targetProfileId: effectiveTargetProfileId,
				});
				const result = await serversApi.importServers(requestBody);
				setImportResult({
					success: result.success,
					error: typeof result.error === "string" ? result.error : undefined,
				});

				const didSucceed =
					typeof result?.success === "boolean"
						? result.success
						: (result as { status?: string })?.status === "success" ||
							!("error" in (result ?? {}));
				if (didSucceed) {
					const stats = extractImportStats(result);
					const {
						importedCount,
						skippedCount,
						skippedServers,
						skippedDetails,
					} = stats;
					const skippedSummary = summarizeSkipped(skippedDetails, t);
					const fallbackList = formatNameList(skippedServers, t);
					const skippedDescription = skippedSummary
						? skippedSummary
						: skippedCount > 0
							? `${skippedCount} server${skippedCount > 1 ? "s" : ""} skipped${fallbackList ? ` (${fallbackList})` : ""}`
							: "";
					const shouldAutoClose = importedCount > 0;
					if (importedCount > 0) {
						const parts: string[] = [
							`${importedCount} server${importedCount > 1 ? "s" : ""} imported`,
						];
						if (skippedCount > 0) {
							parts.push(skippedDescription);
						}
						notifySuccess("Servers installed", parts.join("; "));
					} else if (skippedCount > 0) {
						notifyInfo(
							"No new servers installed",
							skippedDescription ||
								`${skippedCount} server${skippedCount > 1 ? "s" : ""} skipped (already installed).`,
						);
					} else {
            notifySuccess("Servers installed", "Import completed (no changes)");
					}
					if (shouldAutoClose) {
						opts.onImported?.();
					}
					return true;
				}
        notifyError("Import failed", String(result.error ?? "Unknown error"));
				return false;
			} catch (error) {
				const message =
					error instanceof Error ? error.message : String(error ?? "");
				notifyError("Import failed", message || "Unexpected error");
				return false;
			} finally {
				setImporting(false);
			}
		},
		[
			drafts,
			selectedDraftNames,
			selectedDrafts.length,
			targetProfileId,
			t,
			i18n.language,
			opts,
		],
	);

	const state = useMemo(
		() => ({
			drafts,
			selectedDraftNames,
			selectedDrafts,
			source,
			previewState,
			previewError,
			isPreviewLoading,
			isImporting,
			open: isDrawerOpen,
			currentStep,
			importResult,
			targetProfileId,
			dryRunResult,
			dryRunStats,
			dryRunWarning,
			isDryRunLoading,
			dryRunError,
		}),
		[
			drafts,
			selectedDraftNames,
			selectedDrafts,
			source,
			previewState,
			previewError,
			isPreviewLoading,
			isImporting,
			isDrawerOpen,
			currentStep,
			importResult,
			targetProfileId,
			dryRunResult,
			dryRunStats,
			dryRunWarning,
			isDryRunLoading,
			dryRunError,
		],
	);

	return useMemo(
		() => ({
			state,
			begin,
			confirmImport,
			close: reset,
			reset,
			setDrafts,
			setDraftCollection,
			updateDraft,
			setSelectedDraftNames: updateSelectedDraftNames,
			setImportResult,
			setPreviewState,
			setPreviewError,
			setPreviewLoading,
			setCurrentStep,
			setTargetProfileId,
			previewDrafts,
			performDryRun,
		}),
		[
			state,
			begin,
			confirmImport,
			reset,
			setDraftCollection,
			updateDraft,
			updateSelectedDraftNames,
			setDrafts,
			setImportResult,
			setPreviewState,
			setPreviewError,
			setPreviewLoading,
			setCurrentStep,
			setTargetProfileId,
			previewDrafts,
			performDryRun,
		],
	);
}
