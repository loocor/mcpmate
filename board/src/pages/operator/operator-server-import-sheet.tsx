import { Loader2 } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import type { ServerInstallDraft } from "../../hooks/use-server-install-pipeline";
import type { ImportStats } from "../../lib/api";
import { useAutoAddTargetProfile } from "../../lib/default-profile";
import { Button } from "../../components/ui/button";
import { cn } from "../../lib/utils";
import { operatorNoDragRegionStyle } from "./operator-row-detail-shared";
import type { OperatorServerImportPhase } from "./use-operator-server-import";

function draftKindLabel(
	kind: ServerInstallDraft["kind"],
	t: ReturnType<typeof useTranslation>["t"],
): string {
	switch (kind) {
		case "stdio":
			return t("operator:import.kind.stdio", { defaultValue: "stdio" });
		case "sse":
			return t("operator:import.kind.sse", { defaultValue: "SSE" });
		case "streamable_http":
			return t("operator:import.kind.streamableHttp", {
				defaultValue: "Streamable HTTP",
			});
		default:
			return kind;
	}
}

function draftEndpointSummary(draft: ServerInstallDraft): string {
	if (draft.kind === "stdio") {
		return [draft.command, ...(draft.args ?? [])].filter(Boolean).join(" ") || "—";
	}
	return draft.url?.trim() || "—";
}

function DraftRow({ draft }: { draft: ServerInstallDraft }) {
	const { t } = useTranslation();
	const name = draft.name?.trim() || t("operator:import.unnamed", { defaultValue: "Unnamed" });
	const endpoint = draftEndpointSummary(draft);

	return (
		<li className="rounded-lg border border-slate-200 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900/60">
			<div className="flex items-start justify-between gap-2">
				<p className="truncate text-sm font-medium text-slate-900 dark:text-slate-100">
					{name}
				</p>
				<span className="shrink-0 rounded-full bg-white px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-slate-500 ring-1 ring-slate-200 dark:bg-slate-950 dark:text-slate-400 dark:ring-slate-700">
					{draftKindLabel(draft.kind, t)}
				</span>
			</div>
			<p
				className="mt-1 truncate font-mono text-[11px] text-slate-500 dark:text-slate-400"
				title={endpoint}
			>
				{endpoint}
			</p>
		</li>
	);
}

function DryRunSummary({
	dryRunStats,
	isLoading,
	warning,
	error,
}: {
	dryRunStats: ImportStats | null;
	isLoading: boolean;
	warning: string | null;
	error: string | null;
}) {
	const { t } = useTranslation();

	if (isLoading) {
		return (
			<p className="flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400">
				<Loader2 className="h-3.5 w-3.5 animate-spin" aria-hidden />
				{t("operator:import.dryRun.checking", {
					defaultValue: "Checking install plan…",
				})}
			</p>
		);
	}

	if (error) {
		return <p className="text-xs text-red-600 dark:text-red-400">{error}</p>;
	}

	if (!dryRunStats) {
		return null;
	}

	const willInstall = dryRunStats.importedCount;
	const parts: string[] = [];
	if (willInstall > 0) {
		parts.push(
			t("operator:import.dryRun.willInstall", {
				count: willInstall,
				defaultValue: "{{count}} will install",
			}),
		);
	}
	if (dryRunStats.skippedCount > 0) {
		parts.push(
			t("operator:import.dryRun.willSkip", {
				count: dryRunStats.skippedCount,
				defaultValue: "{{count}} already installed",
			}),
		);
	}

	if (parts.length === 0) {
		return (
			<p className="text-xs text-slate-500 dark:text-slate-400">
				{t("operator:import.dryRun.noChanges", {
					defaultValue: "Nothing new to install.",
				})}
			</p>
		);
	}

	return (
		<div className="space-y-1">
			<p className="text-xs font-medium text-slate-700 dark:text-slate-300">
				{parts.join(" · ")}
			</p>
			{warning ? (
				<p className="text-xs text-amber-700 dark:text-amber-300">{warning}</p>
			) : null}
		</div>
	);
}

function ProfileHint() {
	const { t } = useTranslation();
	const autoAdd = useAutoAddTargetProfile();

	if (!autoAdd.enabled) {
		return (
			<p className="text-xs text-slate-500 dark:text-slate-400">
				{t("operator:import.profileOff", {
					defaultValue: "Auto-add to default profile is off.",
				})}
			</p>
		);
	}

	if (autoAdd.isLoading) {
		return (
			<p className="flex items-center gap-2 text-xs text-slate-500 dark:text-slate-400">
				<Loader2 className="h-3.5 w-3.5 animate-spin" aria-hidden />
				{t("operator:import.profileLoading", {
					defaultValue: "Resolving default profile…",
				})}
			</p>
		);
	}

	if (autoAdd.profileName) {
		return (
			<p className="text-xs text-slate-500 dark:text-slate-400">
				{t("operator:import.profileTarget", {
					name: autoAdd.profileName,
					defaultValue: 'Will add to profile "{{name}}".',
				})}
			</p>
		);
	}

	return (
		<p className="text-xs text-amber-700 dark:text-amber-300">
			{t("operator:import.profileMissing", {
				defaultValue: "No active default profile is available for auto-add.",
			})}
		</p>
	);
}

export function OperatorServerImportSheet({
	canInstall,
	drafts,
	dryRunError,
	dryRunStats,
	dryRunWarning,
	isDryRunLoading,
	onCancel,
	onConfirm,
	open,
	parseError,
	phase,
}: {
	canInstall: boolean;
	drafts: ServerInstallDraft[];
	dryRunError: string | null;
	dryRunStats: ImportStats | null;
	dryRunWarning: string | null;
	isDryRunLoading: boolean;
	onCancel: () => void;
	onConfirm: () => void;
	open: boolean;
	parseError: string | null;
	phase: OperatorServerImportPhase;
}) {
	const { t } = useTranslation();
	const titleId = "operator-server-import-title";
	const isImporting = phase === "importing";
	const isParsing = phase === "parsing";

	if (!open) {
		return null;
	}

	return (
		<>
			<button
				type="button"
				className="absolute inset-0 z-30 bg-black/40"
				style={operatorNoDragRegionStyle}
				aria-label={t("operator:import.cancel", { defaultValue: "Cancel import" })}
				onClick={onCancel}
			/>
			<section
				role="dialog"
				aria-modal="true"
				aria-labelledby={titleId}
				className={cn(
					"absolute inset-x-0 bottom-0 z-40 flex max-h-[min(72%,520px)] flex-col rounded-t-xl border-t border-slate-200 bg-white shadow-2xl dark:border-slate-800 dark:bg-slate-950",
				)}
				style={operatorNoDragRegionStyle}
				data-testid="operator-server-import-sheet"
			>
				<div className="border-b border-slate-100 px-3 py-2.5 dark:border-slate-800">
					<h2
						id={titleId}
						className="text-sm font-semibold text-slate-900 dark:text-slate-100"
					>
						{t("operator:import.title", { defaultValue: "Import servers" })}
					</h2>
					<p className="mt-0.5 text-xs text-slate-500 dark:text-slate-400">
						{t("operator:import.subtitle", {
							defaultValue: "Review detected servers before installing.",
						})}
					</p>
				</div>

				<div className="min-h-0 flex-1 overflow-y-auto px-3 py-3">
					{isParsing ? (
						<p className="flex items-center gap-2 text-sm text-slate-500 dark:text-slate-400">
							<Loader2 className="h-4 w-4 animate-spin" aria-hidden />
							{t("operator:import.parsing", {
								defaultValue: "Parsing dropped configuration…",
							})}
						</p>
					) : parseError ? (
						<p className="text-sm text-red-600 dark:text-red-400">{parseError}</p>
					) : (
						<ul className="space-y-2">
							{drafts.map((draft, index) => (
								<DraftRow
									key={`${draft.name}-${draft.kind}-${index}`}
									draft={draft}
								/>
							))}
						</ul>
					)}

					{!parseError && !isParsing ? (
						<div className="mt-3 space-y-2 border-t border-slate-100 pt-3 dark:border-slate-800">
							<DryRunSummary
								dryRunStats={dryRunStats}
								error={dryRunError}
								isLoading={isDryRunLoading}
								warning={dryRunWarning}
							/>
							<ProfileHint />
						</div>
					) : null}
				</div>

				<div className="flex shrink-0 items-center justify-end gap-2 border-t border-slate-100 px-3 py-2.5 dark:border-slate-800">
					<Button
						type="button"
						variant="ghost"
						size="sm"
						disabled={isImporting}
						onClick={onCancel}
					>
						{t("operator:import.cancel", { defaultValue: "Cancel" })}
					</Button>
					<Button
						type="button"
						size="sm"
						disabled={!canInstall || isImporting || isParsing || Boolean(parseError)}
						onClick={onConfirm}
					>
						{isImporting ? (
							<>
								<Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" aria-hidden />
								{t("operator:import.installing", { defaultValue: "Installing…" })}
							</>
						) : (
							t("operator:import.install", { defaultValue: "Install" })
						)}
					</Button>
				</div>
			</section>
		</>
	);
}
