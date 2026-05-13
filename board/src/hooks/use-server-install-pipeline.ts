import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
	extractImportStats,
	serversApi,
} from "../lib/api";
import {
	buildDraftServersImportRequest,
	urlWithMergedSearchParams,
} from "../lib/server-import-payload";
import type { ImportStats } from "../lib/api";
import { notifyError, notifyInfo, notifySuccess } from "../lib/notify";
import { formatNameList, summarizeSkipped } from "../lib/server-import-utils";
import type { ServerMetaInfo } from "../lib/types";

export type InstallSource = "manual" | "ingest" | "market";
export type WizardStep = "form" | "preview" | "result";

export interface ServerInstallDraft {
	name: string;
	serverId?: string;
	kind: "stdio" | "sse" | "streamable_http";
	command?: string;
	args?: string[];
	env?: Record<string, string>;
	url?: string;
	registryServerId?: string;
	headers?: Record<string, string>;
	urlParams?: Record<string, string>;
	meta?: ServerMetaInfo;
}

interface UseServerInstallPipelineOptions {
	onImported?: () => void;
}

interface PreviewState {
	success: boolean;
	data?: any;
	error?: unknown;
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
	const [source, setSource] = useState<InstallSource | null>(null);
	const [isPreviewLoading, setPreviewLoading] = useState(false);
	const [previewState, setPreviewState] = useState<PreviewState | null>(null);
	const [previewError, setPreviewError] = useState<string | null>(null);
	const [isImporting, setImporting] = useState(false);
	const [currentStep, setCurrentStep] = useState<WizardStep>("form");
	const [importResult, setImportResult] = useState<any>(null);
	const [targetProfileId, setTargetProfileId] = useState<string | null>(null);
	const [dryRunResult, setDryRunResult] = useState<any>(null);
	const [dryRunStats, setDryRunStats] = useState<ImportStats | null>(null);
	const [dryRunWarning, setDryRunWarning] = useState<string | null>(null);
	const [isDryRunLoading, setDryRunLoading] = useState(false);
	const [dryRunError, setDryRunError] = useState<string | null>(null);

	const reset = useCallback(() => {
		setDrawerOpen(false);
		setDrafts([]);
		setSource(null);
		setPreviewState(null);
		setPreviewError(null);
		setPreviewLoading(false);
		setImporting(false);
		setCurrentStep("form");
		setImportResult(null);
		setTargetProfileId(null);
		setDryRunResult(null);
		setDryRunStats(null);
		setDryRunWarning(null);
		setDryRunLoading(false);
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

	const begin = useCallback(
		async (items: ServerInstallDraft[], origin: InstallSource) => {
			if (!items.length) {
				notifyError(
					"No servers detected",
					"Provide at least one server to preview",
				);
				return;
			}

			setDrafts(items);
			setSource(origin);
			setPreviewState(null);
			setPreviewError(null);
			setDrawerOpen(true);
			setCurrentStep("preview");
			setPreviewLoading(true);

			try {
				const payload = buildPreviewPayload(items);
				const result = await serversApi.previewServers({
					...payload,
					timeout_ms: 30000, // 30 seconds for stdio servers that need to install dependencies
					// TODO: Implement intelligent timeout handling for server preview
					// - Add user-friendly timeout management UI
					// - Show dependency download progress indication
					// - Allow users to temporarily increase timeout time
					// - Implement smart timeout detection based on server type and dependency requirements
				});
				setPreviewState(result);
			} catch (error) {
				const message =
					error instanceof Error ? error.message : "Preview request failed";
				setPreviewError(message);
				notifyError("Preview failed", message);
			} finally {
				setPreviewLoading(false);
			}
		},
		[buildPreviewPayload],
	);

	const performDryRun = useCallback(async () => {
		if (!drafts.length) return;
		try {
			setDryRunLoading(true);
			setDryRunError(null);
			setDryRunStats(null);
			setDryRunWarning(null);
			const requestBody = buildDraftServersImportRequest({
				drafts,
				targetProfileId,
				dryRun: true,
			});
			const result = await serversApi.importServers(requestBody);
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
			setDryRunStats(null);
			setDryRunWarning(null);
			const message = error instanceof Error ? error.message : String(error ?? "");
			setDryRunError(
				message ||
					t("wizard.result.validationErrorGeneric", {
						defaultValue: "Failed to validate import",
					}),
			);
		} finally {
			setDryRunLoading(false);
		}
	}, [drafts, targetProfileId, t, i18n.language]);

	const confirmImport = useCallback(
		async (overrideTargetProfileId?: string | null) => {
			if (!drafts.length) return false;
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
					targetProfileId: effectiveTargetProfileId,
				});
				const result = await serversApi.importServers(requestBody);
				setImportResult(result);

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
						notifySuccess(
							"Servers installed",
							"Import completed (no changes)",
						);
					}
					if (shouldAutoClose) {
						opts.onImported?.();
					}
					return true;
				}
				notifyError(
					"Import failed",
					String(result.error ?? "Unknown error"),
				);
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
		[drafts, targetProfileId, t, i18n.language, opts],
	);

	const state = useMemo(
		() => ({
			drafts,
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
			setImportResult,
			setPreviewState,
			setPreviewError,
			setPreviewLoading,
			setCurrentStep,
			setTargetProfileId,
			performDryRun,
		}),
		[
			state,
			begin,
			confirmImport,
			reset,
			setDrafts,
			setImportResult,
			setPreviewState,
			setPreviewError,
			setPreviewLoading,
			setCurrentStep,
			setTargetProfileId,
			performDryRun,
		],
	);
}
