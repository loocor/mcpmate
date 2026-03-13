import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
	extractImportStats,
	serializeMetaForApi,
	serversApi,
} from "../lib/api";
import type { ImportStats } from "../lib/api";
import { notifyError, notifyInfo, notifySuccess } from "../lib/notify";
import { formatNameList, summarizeSkipped } from "../lib/server-import-utils";
import type { ServerMetaInfo } from "../lib/types";

export type InstallSource = "manual" | "ingest" | "market";
export type WizardStep = "form" | "preview" | "result";

export interface ServerInstallDraft {
	name: string;
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

export function useServerInstallPipeline(
	opts: UseServerInstallPipelineOptions = {},
) {
	const { t } = useTranslation("servers");
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
				kind: item.kind,
				command: item.kind === "stdio" ? (item.command ?? null) : null,
				args: item.args?.length ? item.args : null,
				env: item.env && Object.keys(item.env).length ? item.env : null,
				url:
					item.kind !== "stdio"
						? (() => {
								if (!item.url) return null;
								if (!item.urlParams || !Object.keys(item.urlParams).length)
									return item.url;
								try {
									const u = new URL(
										item.url,
										/^https?:/i.test(item.url)
											? undefined
											: "http://dummy.local",
									);
									for (const [k, v] of Object.entries(item.urlParams)) {
										u.searchParams.set(k, v);
									}
									return /^https?:/i.test(item.url)
										? u.toString()
										: `${item.url}?${u.searchParams.toString()}`;
								} catch {
									// Fallback: naive concatenation
									const qs = new URLSearchParams(item.urlParams).toString();
									return `${item.url}?${qs}`;
								}
							})()
						: null,
				headers:
					item.kind !== "stdio" &&
					item.headers &&
					Object.keys(item.headers).length
						? item.headers
						: null,
			})),
		};
	}, []);

	const buildImportPayload = useCallback((items: ServerInstallDraft[]) => {
		const payload: Record<string, any> = {};
		for (const item of items) {
			const metaPayload = serializeMetaForApi(item.meta);
			const entry: Record<string, unknown> = {
				type: item.kind,
			};
			if (item.kind === "stdio" && item.command) {
				entry.command = item.command;
			}
			if (item.kind !== "stdio" && item.url) {
				// Compose full URL with urlParams (same behavior as preview)
				if (item.urlParams && Object.keys(item.urlParams).length) {
					try {
						const u = new URL(
							item.url,
							/^https?:/i.test(item.url) ? undefined : "http://dummy.local",
						);
						for (const [k, v] of Object.entries(item.urlParams)) {
							u.searchParams.set(k, v);
						}
						entry.url = /^https?:/i.test(item.url)
							? u.toString()
							: `${item.url}?${u.searchParams.toString()}`;
					} catch {
						const qs = new URLSearchParams(
							item.urlParams as Record<string, string>,
						).toString();
						entry.url = `${item.url}?${qs}`;
					}
				} else {
					entry.url = item.url;
				}
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
			// Do not send a separate url_params field: backend import path doesn't consume it today.
			if (metaPayload) {
				entry.meta = metaPayload;
			}
			payload[item.name] = entry;
		}
		return { mcpServers: payload };
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
			const payload = buildImportPayload(drafts);
			const requestBody = {
				...payload,
				dry_run: true,
				...(targetProfileId ? { target_profile_id: targetProfileId } : {}),
			};
			const result = await serversApi.importServers(requestBody);
			setDryRunResult(result);
			const stats = extractImportStats(result);
			setDryRunStats(stats);

			const skipSummary = summarizeSkipped(stats.skippedDetails);
			const skipFallback = formatNameList(stats.skippedServers);
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
				const failedNames = formatNameList(stats.failedServers);
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
	}, [drafts, buildImportPayload, targetProfileId]);

	const confirmImport = useCallback(async () => {
		if (!drafts.length) return false;
		try {
			setImporting(true);
			setCurrentStep("result");
			const payload = buildImportPayload(drafts);
			const requestBody = targetProfileId
				? { ...payload, target_profile_id: targetProfileId }
				: payload;
			const result = await serversApi.importServers(requestBody);
			setImportResult(result);

			const didSucceed =
				typeof result?.success === "boolean"
					? result.success
					: (result as { status?: string })?.status === "success" ||
						!("error" in (result ?? {}));
			if (didSucceed) {
				const stats = extractImportStats(result);
				const { importedCount, skippedCount, skippedServers, skippedDetails } =
					stats;
				const skippedSummary = summarizeSkipped(skippedDetails);
				const fallbackList = formatNameList(skippedServers);
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
	}, [drafts, buildImportPayload, opts, targetProfileId]);

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
			setPreviewState,
			setCurrentStep,
			setTargetProfileId,
			performDryRun,
		}),
		[
			state,
			begin,
			confirmImport,
			reset,
			setPreviewState,
			setCurrentStep,
			setTargetProfileId,
			performDryRun,
		],
	);
}
