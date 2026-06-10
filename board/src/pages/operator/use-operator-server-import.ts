import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import type { ServerInstallDraft } from "../../hooks/use-server-install-pipeline";
import { extractImportStats, serversApi, type ImportStats } from "../../lib/api";
import { resolveAutoAddTargetProfileId } from "../../lib/default-profile";
import {
	normalizeIngestPayload,
	type ServerIngestPayload,
} from "../../lib/install-normalizer";
import { notifyError, notifyInfo, notifySuccess } from "../../lib/notify";
import { buildDraftServersImportRequest } from "../../lib/server-import-payload";
import {
	canIngestFromDataTransfer,
	extractPayloadFromDataTransfer,
	formatServerUniImportTransferError,
} from "../../lib/server-uni-import-transfer";
import { formatNameList, summarizeSkipped } from "../../lib/server-import-utils";
import { useAppStore } from "../../lib/store";

export type OperatorServerImportPhase =
	| "idle"
	| "parsing"
	| "ready"
	| "importing";

function buildDryRunSummary(
	stats: ImportStats,
	t: ReturnType<typeof useTranslation>["t"],
): { warning: string | null; error: string | null } {
	const skipSummary = summarizeSkipped(stats.skippedDetails, t);
	const skipFallback = formatNameList(stats.skippedServers, t);
	let warning: string | null = null;
	let error: string | null = null;

	if (stats.skippedCount > 0) {
		const detail = skipSummary || skipFallback;
		const base =
			stats.skippedCount === 1
				? t("operator:import.dryRun.skipSingle", {
						defaultValue: "{{count}} already installed",
						count: stats.skippedCount,
					})
				: t("operator:import.dryRun.skipMultiple", {
						defaultValue: "{{count}} already installed",
						count: stats.skippedCount,
					});
		warning = detail
			? t("operator:import.dryRun.skipWithDetail", {
					base,
					detail,
					defaultValue: "{{base}} ({{detail}})",
				})
			: base;
	}

	if (stats.failedCount > 0) {
		const failedNames = formatNameList(stats.failedServers, t);
		error = t("operator:import.dryRun.failed", {
			count: stats.failedCount,
			servers:
				failedNames ||
				t("operator:import.dryRun.failedFallback", {
					count: stats.failedCount,
					defaultValue: "{{count}} server(s)",
				}),
			defaultValue: "{{count}} server(s) failed validation: {{servers}}",
		});
	}

	return { warning, error };
}

function buildSkippedInstallDescription(
	stats: Pick<ImportStats, "skippedCount" | "skippedDetails" | "skippedServers">,
	t: ReturnType<typeof useTranslation>["t"],
): string {
	const skippedSummary = summarizeSkipped(stats.skippedDetails, t);
	if (skippedSummary) {
		return skippedSummary;
	}
	if (stats.skippedCount <= 0) {
		return "";
	}

	const fallbackList = formatNameList(stats.skippedServers, t);
	if (fallbackList) {
		return t("operator:import.installSkippedWithDetail", {
			count: stats.skippedCount,
			servers: fallbackList,
			defaultValue: "{{count}} server skipped ({{servers}})",
		});
	}

	return t("operator:import.installSkipped", {
		count: stats.skippedCount,
		defaultValue: "{{count}} server skipped",
	});
}

async function resolveTargetProfileId(): Promise<string | null> {
	return resolveAutoAddTargetProfileId({
		autoAddEnabled:
			useAppStore.getState().dashboardSettings.autoAddServerToDefaultProfile,
	});
}

export function useOperatorServerImport({
	onImported,
}: {
	onImported?: () => void;
}) {
	const { t } = useTranslation();
	const [open, setOpen] = useState(false);
	const [phase, setPhase] = useState<OperatorServerImportPhase>("idle");
	const [drafts, setDrafts] = useState<ServerInstallDraft[]>([]);
	const [parseError, setParseError] = useState<string | null>(null);
	const [dryRunStats, setDryRunStats] = useState<ImportStats | null>(null);
	const [dryRunWarning, setDryRunWarning] = useState<string | null>(null);
	const [dryRunError, setDryRunError] = useState<string | null>(null);
	const [isDryRunLoading, setDryRunLoading] = useState(false);

	const reset = useCallback(() => {
		setOpen(false);
		setPhase("idle");
		setDrafts([]);
		setParseError(null);
		setDryRunStats(null);
		setDryRunWarning(null);
		setDryRunError(null);
		setDryRunLoading(false);
	}, []);

	const runDryRun = useCallback(
		async (items: ServerInstallDraft[]) => {
			setDryRunLoading(true);
			setDryRunStats(null);
			setDryRunWarning(null);
			setDryRunError(null);
			try {
				const targetProfileId = await resolveTargetProfileId();
				const result = await serversApi.importServers(
					buildDraftServersImportRequest({
						drafts: items,
						targetProfileId,
						dryRun: true,
					}),
				);
				const stats = extractImportStats(result);
				setDryRunStats(stats);
				const summary = buildDryRunSummary(stats, t);
				setDryRunWarning(summary.warning);
				setDryRunError(summary.error);
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				setDryRunError(
					message ||
						t("operator:import.dryRun.validationError", {
							defaultValue: "Failed to validate import",
						}),
				);
			} finally {
				setDryRunLoading(false);
				setPhase("ready");
			}
		},
		[t],
	);

	const ingestPayload = useCallback(
		async (payload: ServerIngestPayload) => {
			setOpen(true);
			setPhase("parsing");
			setParseError(null);
			setDrafts([]);
			setDryRunStats(null);
			setDryRunWarning(null);
			setDryRunError(null);
			try {
				const parsed = await normalizeIngestPayload(payload);
				if (!parsed.length) {
					const title = t("servers:manual.ingest.noneDetectedTitle", {
						defaultValue: "No servers detected",
					});
					const message = t("servers:manual.ingest.noneDetectedDescription", {
						defaultValue:
							"We could not find any server definitions in the input.",
					});
					setParseError(message);
					setPhase("ready");
					notifyError(title, message);
					return;
				}
				setDrafts(parsed);
				await runDryRun(parsed);
			} catch (error) {
				const title = t("servers:manual.ingest.parseFailedTitle", {
					defaultValue: "Parsing failed",
				});
				const message =
					error instanceof Error
						? error.message
						: t("servers:manual.ingest.parseFailedFallback", {
								defaultValue: "Failed to parse input",
							});
				setParseError(message);
				setPhase("ready");
				notifyError(title, message);
			}
		},
		[runDryRun, t],
	);

	const handleImportDrop = useCallback(
		async (dataTransfer: DataTransfer) => {
			if (!canIngestFromDataTransfer(dataTransfer)) {
				notifyError(
					t("servers:notifications.importUnsupported.title", {
						defaultValue: "Unsupported content",
					}),
					t("servers:notifications.importUnsupported.message", {
						defaultValue:
							"Drop text, JSON snippets, URLs, or config files to use Uni-Import.",
					}),
				);
				return;
			}
			let payload;
			try {
				payload = await extractPayloadFromDataTransfer(dataTransfer);
			} catch (error) {
				notifyError(
					t("servers:notifications.importUnsupported.title", {
						defaultValue: "Unsupported content",
					}),
					formatServerUniImportTransferError(
						error,
						t,
						"servers:notifications.importRejections",
					),
				);
				return;
			}
			if (!payload) {
				notifyError(
					t("servers:notifications.importEmpty.title", {
						defaultValue: "Nothing to import",
					}),
					t("servers:notifications.importEmpty.message", {
						defaultValue:
							"We could not detect any usable configuration from the dropped content.",
					}),
				);
				return;
			}
			await ingestPayload(payload);
		},
		[ingestPayload, t],
	);

	const confirmInstall = useCallback(async () => {
		if (!drafts.length || parseError || dryRunError) {
			return false;
		}
		setPhase("importing");
		try {
			const targetProfileId = await resolveTargetProfileId();
			const result = await serversApi.importServers(
				buildDraftServersImportRequest({
					drafts,
					targetProfileId,
				}),
			);
			const stats = extractImportStats(result);
			const didSucceed =
				typeof result?.success === "boolean"
					? result.success
					: (result as { status?: string })?.status === "success" ||
						!("error" in (result ?? {}));

			if (!didSucceed) {
				notifyError(
					t("operator:import.installFailed", { defaultValue: "Import failed" }),
					String(result.error ?? "Unknown error"),
				);
				setPhase("ready");
				return false;
			}

			const { importedCount, skippedCount } = stats;
			const skippedDescription = buildSkippedInstallDescription(stats, t);

			if (importedCount > 0) {
				const parts = [
					t("operator:import.installSuccess", {
						count: importedCount,
						defaultValue: "{{count}} server installed",
					}),
				];
				if (skippedCount > 0) {
					parts.push(skippedDescription);
				}
				notifySuccess(
					t("operator:import.installSuccessTitle", {
						defaultValue: "Servers installed",
					}),
					parts.join("; "),
				);
				onImported?.();
				reset();
				return true;
			}

			if (skippedCount > 0) {
				notifyInfo(
					t("operator:import.installSkippedTitle", {
						defaultValue: "No new servers installed",
					}),
					skippedDescription ||
						t("operator:import.installSkippedAll", {
							count: skippedCount,
							defaultValue: "{{count}} server(s) skipped (already installed).",
						}),
				);
			}

			reset();
			return importedCount > 0;
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			notifyError(
				t("operator:import.installFailed", { defaultValue: "Import failed" }),
				message ||
					t("operator:import.unexpectedError", {
						defaultValue: "Unexpected error",
					}),
			);
			setPhase("ready");
			return false;
		}
	}, [drafts, dryRunError, onImported, parseError, reset, t]);

	const canInstall =
		drafts.length > 0 &&
		!parseError &&
		!dryRunError &&
		!isDryRunLoading &&
		phase !== "importing" &&
		(dryRunStats?.failedCount ?? 0) === 0 &&
		((dryRunStats?.importedCount ?? drafts.length) > 0 ||
			(dryRunStats === null && drafts.length > 0));

	// When dry-run completes with zero to import and all skipped, disable install
	const nothingToInstall =
		dryRunStats !== null &&
		dryRunStats.importedCount === 0 &&
		dryRunStats.failedCount === 0;

	return {
		open,
		phase,
		drafts,
		parseError,
		dryRunStats,
		dryRunWarning,
		dryRunError,
		isDryRunLoading,
		canInstall: canInstall && !nothingToInstall,
		handleImportDrop,
		confirmInstall,
		cancel: reset,
	};
}
