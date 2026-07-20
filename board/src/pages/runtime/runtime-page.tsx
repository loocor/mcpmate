import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { RefreshCw, Wrench } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import { ConfirmDialog } from "../../components/confirm-dialog";
import { StatusBadge } from "../../components/status-badge";
import { Button } from "../../components/ui/button";
import {
	Card,
	CardContent,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import { capabilitiesApi, runtimeApi } from "../../lib/api";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { notifyError, notifySuccess } from "../../lib/notify";
import type {
	ClearCacheResponse,
	InstallResponse,
} from "../../lib/types";
import { formatBytes, formatLocalDateTime, formatPathWithTilde } from "../../lib/utils";

type RuntimeKind = "uv" | "bun" | "node";

const RUNTIME_KINDS: RuntimeKind[] = ["node", "bun", "uv"];

type ConfirmState =
	| { type: "resetAll" }
	| { type: "resetOne"; key: RuntimeKind }
	| { type: "install"; key: RuntimeKind }
	| { type: "capabilitiesReset" }
	| null;

function normalizeRuntimeKind(
	runtimeType: string | null | undefined,
): RuntimeKind | null {
	switch (runtimeType?.trim().toLowerCase()) {
		case "node":
			return "node";
		case "bun":
			return "bun";
		case "uv":
			return "uv";
		default:
			return null;
	}
}

function getRuntimeBadgeStatus(
	managedReady: boolean,
	hasSystemFallback: boolean,
): "running" | "fallback" | "offline" {
	if (managedReady) {
		return "running";
	}

	if (hasSystemFallback) {
		return "fallback";
	}

	return "offline";
}

export function RuntimePage() {
	usePageTranslations("runtime");
	const { t } = useTranslation();
	const qc = useQueryClient();
	const [confirm, setConfirm] = React.useState<ConfirmState>(null);

	const runtimeStatusQ = useQuery({
		queryKey: ["runtimeStatus"],
		queryFn: runtimeApi.getStatus,
		refetchInterval: 60_000,
		retry: false,
		refetchOnWindowFocus: false,
	});

	const runtimeCacheQ = useQuery({
		queryKey: ["runtimeCache"],
		queryFn: runtimeApi.getCache,
		refetchInterval: 60_000,
		retry: false,
		refetchOnWindowFocus: false,
	});

	const capabilitiesStatsQ = useQuery({
		queryKey: ["capabilities", "stats"],
		queryFn: capabilitiesApi.getStats,
		refetchInterval: 60_000,
		retry: false,
		refetchOnWindowFocus: false,
	});

	const resetAllM = useMutation<{ success: boolean }, Error, void>({
		mutationFn: async () => runtimeApi.resetCache("all"),
		onSuccess: () => {
			qc.invalidateQueries({ queryKey: ["runtimeCache"] });
			notifySuccess(
				t("runtime:toasts.resetAllTitle", {
					defaultValue: "Caches reset",
				}),
				t("runtime:toasts.resetAllDescription", {
					defaultValue: "All runtime caches cleared.",
				}),
			);
			setConfirm(null);
		},
		onError: (e) => {
			qc.invalidateQueries({ queryKey: ["runtimeCache"] });
			notifyError(
				t("runtime:toasts.errorResetTitle", {
					defaultValue: "Reset failed",
				}),
				e.message,
			);
		},
	});

	const resetOneM = useMutation<ClearCacheResponse, Error, RuntimeKind>({
		mutationFn: async (kind) => runtimeApi.resetCache(kind),
		onSuccess: (_data, kind) => {
			qc.invalidateQueries({ queryKey: ["runtimeCache"] });
			notifySuccess(
				t("runtime:toasts.resetOneTitle", {
					defaultValue: "Cache reset",
				}),
				t("runtime:toasts.resetOneDescription", {
					defaultValue: "{{runtime}} cache cleared.",
					runtime: getRuntimeLabel(kind),
				}),
			);
			setConfirm(null);
		},
		onError: (e) => {
			qc.invalidateQueries({ queryKey: ["runtimeCache"] });
			notifyError(
				t("runtime:toasts.errorResetTitle", {
					defaultValue: "Reset failed",
				}),
				e.message,
			);
		},
	});

	const installM = useMutation<InstallResponse, Error, RuntimeKind>({
		mutationFn: async (kind) =>
			runtimeApi.install({ runtime_type: kind, verbose: true }),
		onSuccess: (data, kind) => {
			qc.invalidateQueries({ queryKey: ["runtimeStatus"] });
			qc.invalidateQueries({ queryKey: ["runtimeCache"] });

			const resolvedKind = normalizeRuntimeKind(data.runtime_type);
			if (resolvedKind !== kind) {
				notifyError(
					t("runtime:toasts.errorInstallTitle", {
						defaultValue: "Install failed",
					}),
					t("runtime:toasts.installTargetMismatch", {
						defaultValue:
							"Requested {{requested}}, but the server reported {{actual}}.",
						requested: getRuntimeLabel(kind),
						actual: resolvedKind ? getRuntimeLabel(resolvedKind) : data.runtime_type,
					}),
				);
				setConfirm(null);
				return;
			}

			notifySuccess(
				t("runtime:toasts.installTitle", {
					defaultValue: "Install complete",
				}),
				t("runtime:toasts.installDescription", {
					defaultValue: "{{runtime}}: {{message}}",
					runtime: getRuntimeLabel(kind),
					message: data.message,
				}),
			);
			setConfirm(null);
		},
		onError: (e) => {
			qc.invalidateQueries({ queryKey: ["runtimeStatus"] });
			qc.invalidateQueries({ queryKey: ["runtimeCache"] });
			notifyError(
				t("runtime:toasts.errorInstallTitle", {
					defaultValue: "Install failed",
				}),
				e.message,
			);
		},
	});

	const capResetM = useMutation<ClearCacheResponse, Error, void>({
		mutationFn: async () => capabilitiesApi.reset(),
		onSuccess: () => {
			qc.invalidateQueries({ queryKey: ["capabilities", "stats"] });
			notifySuccess(
				t("runtime:toasts.capabilitiesResetTitle", {
					defaultValue: "Capabilities cache cleared",
				}),
				t("runtime:toasts.capabilitiesResetDescription", {
					defaultValue:
						"Memory caches were cleared and durable capability snapshots were invalidated.",
				}),
			);
			setConfirm(null);
		},
		onError: (e) =>
			notifyError(
				t("runtime:toasts.errorResetTitle", {
					defaultValue: "Reset failed",
				}),
				e.message,
			),
	});

	const isBusy =
		resetAllM.isPending || resetOneM.isPending || installM.isPending;
	const status = runtimeStatusQ.data;
	const userHome = status?.user_home ?? null;
	const cache = runtimeCacheQ.data;
	const capStats = capabilitiesStatsQ.data;
	const getRuntimeLabel = (kind: RuntimeKind) =>
		t(`runtime:types.${kind}`, { defaultValue: kind.toUpperCase() });

	let confirmTitle = t("runtime:capabilities.resetConfirmTitle", {
		defaultValue: "Reset capabilities cache?",
	});
	let confirmDescription = t("runtime:capabilities.resetConfirmDesc", {
		defaultValue:
			"This clears the node-local memory cache and invalidates the durable SQLite catalog. The next access must revalidate upstream capabilities.",
	});
	let confirmLabel = t("runtime:dialogs.confirm");
	let confirmVariant: "default" | "destructive" = "destructive";
	let confirmLoading = false;

	if (confirm?.type === "resetAll") {
		confirmTitle = t("runtime:dialogs.resetAllTitle");
		confirmDescription = t("runtime:dialogs.resetAllDescription");
		confirmLoading = resetAllM.isPending;
	} else if (confirm?.type === "resetOne") {
		confirmTitle = t("runtime:dialogs.resetOneTitle", {
			key: getRuntimeLabel(confirm.key),
		});
		confirmDescription = t("runtime:dialogs.resetOneDescription", {
			key: getRuntimeLabel(confirm.key),
		});
		confirmLoading = resetOneM.isPending;
	} else if (confirm?.type === "install") {
		const managedReady = status?.[confirm.key]?.available ?? false;
		if (managedReady) {
			confirmTitle = t("runtime:dialogs.reinstallTitle", {
				key: getRuntimeLabel(confirm.key),
			});
			confirmDescription = t("runtime:dialogs.reinstallDescription", {
				key: getRuntimeLabel(confirm.key),
			});
			confirmLabel = t("runtime:dialogs.reinstallConfirm");
		} else {
			confirmTitle = t("runtime:dialogs.installTitle", {
				key: getRuntimeLabel(confirm.key),
			});
			confirmDescription = t("runtime:dialogs.installDescription", {
				key: getRuntimeLabel(confirm.key),
			});
			confirmLabel = t("runtime:dialogs.installConfirm");
		}
		confirmVariant = "default";
		confirmLoading = installM.isPending;
	} else if (confirm?.type === "capabilitiesReset") {
		confirmLoading = capResetM.isPending;
	}

	return (
		<div className="space-y-4">
			<div className="flex items-center gap-2 min-w-0">
				<p className="flex-1 min-w-0 truncate whitespace-nowrap text-base text-muted-foreground">
					{t("runtime:title")}
				</p>
				<div className="flex items-center gap-2 flex-shrink-0">
					<Button
						variant="outline"
						size="sm"
						disabled={isBusy || runtimeCacheQ.isLoading}
						onClick={() => setConfirm({ type: "resetAll" })}
					>
						<RefreshCw
							className={`mr-2 h-4 w-4 ${resetAllM.isPending ? "animate-spin" : ""}`}
						/>
						{t("runtime:buttons.resetAllCaches")}
					</Button>
				</div>
			</div>

			{runtimeStatusQ.isLoading || runtimeCacheQ.isLoading ? (
				<div className="grid auto-rows-fr gap-4 md:grid-cols-3">
					{[0, 1, 2].map((i) => (
						<div
							key={i}
							className="flex h-full min-h-0 flex-col rounded-lg border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-700 dark:bg-slate-900"
						>
							<div className="mb-4 flex shrink-0 justify-end">
								<div className="h-6 w-16 animate-pulse rounded-full bg-slate-100 dark:bg-slate-900" />
							</div>

							<div className="min-h-0 flex-1 space-y-3">
								<div className="h-16 animate-pulse rounded bg-slate-100 dark:bg-slate-900" />
								<div className="h-12 animate-pulse rounded bg-slate-100 dark:bg-slate-900" />
							</div>

							<div className="mt-4 flex w-full shrink-0 items-center justify-between gap-2">
								<div className="h-9 w-[7.5rem] animate-pulse rounded-md bg-slate-100 dark:bg-slate-900" />
								<div className="h-9 w-[7.5rem] animate-pulse rounded-md bg-slate-100 dark:bg-slate-900" />
							</div>
						</div>
					))}
				</div>
			) : (
				<div className="grid auto-rows-fr gap-4 md:grid-cols-3">
					{RUNTIME_KINDS.map((key) => {
						const st = status?.[key];
						const c = cache?.[key];
						const managedReady = Boolean(st?.available);
						const systemPath = (st?.system_fallback_path ?? "").trim();
						const hasSystemFallback = Boolean(systemPath) && !managedReady;
						const badgeStatus = getRuntimeBadgeStatus(
								managedReady,
								hasSystemFallback,
							);
						const badgeLabel = hasSystemFallback
							? t("runtime:status.fallback", { defaultValue: "Fallback" })
							: undefined;
						const executablePath =
							(st?.path ?? "").trim() || (hasSystemFallback ? systemPath : "");
						const pathForDisplay = formatPathWithTilde(executablePath, userHome);

						return (
							<div
								key={key}
								className="flex h-full min-h-0 flex-col rounded-lg border border-slate-200 bg-white p-4 shadow-sm transition-shadow hover:border-primary/40 hover:shadow-md dark:border-slate-700 dark:bg-slate-900"
							>
								<div className="mb-2 flex shrink-0 items-center justify-between">
									<div className="flex items-center gap-2">
										<Wrench className="h-4 w-4 text-slate-500" />
										<div className="font-semibold uppercase">
											{t(`runtime:types.${key}`)}
										</div>
									</div>
									<StatusBadge status={badgeStatus} statusLabel={badgeLabel} />
								</div>

								<div className="min-h-0 flex-1 space-y-1 text-sm">
									<div className="flex items-center justify-between">
										<span className="text-slate-500">
											{t("runtime:labels.version")}
										</span>
										<span>
											{st?.version ||
												t("runtime:fallbacks.notAvailable", {
													defaultValue: "N/A",
												})}
										</span>
									</div>
									<div className="flex items-center justify-between">
										<span className="text-slate-500">
											{t("runtime:labels.path", { defaultValue: "Path" })}
										</span>
										<span
											className="truncate max-w-[60%]"
											title={executablePath || undefined}
										>
											{pathForDisplay ||
												t("runtime:fallbacks.empty", { defaultValue: "—" })}
										</span>
									</div>
									<div className="flex items-center justify-between">
										<span className="text-slate-500">
											{t("runtime:labels.message")}
										</span>
										<span
											className="truncate max-w-[60%]"
											title={st?.message || ""}
										>
											{st?.message || t("runtime:fallbacks.empty", { defaultValue: "—" })}
										</span>
									</div>

									<div className="mt-3 font-medium">
										{t("runtime:labels.cache")}
									</div>
									<div className="flex items-center justify-between">
										<span className="text-slate-500">
											{t("runtime:labels.size")}
										</span>
										<span>{formatBytes(c?.size_bytes || 0)}</span>
									</div>
									<div className="flex items-center justify-between">
										<span className="text-slate-500">
											{t("runtime:labels.packages")}
										</span>
										<span>{c?.package_count ?? 0}</span>
									</div>
									<div className="flex items-center justify-between">
										<span className="text-slate-500">
											{t("runtime:labels.lastModified")}
										</span>
										<span>
											{c?.last_modified
												? formatLocalDateTime(c.last_modified)
												: t("runtime:fallbacks.empty", { defaultValue: "—" })}
										</span>
									</div>
								</div>

								<footer className="mt-4 flex w-full shrink-0 items-center justify-between gap-2">
									<Button
										size="sm"
										className="shrink-0"
										disabled={installM.isPending}
										onClick={() => setConfirm({ type: "install", key })}
									>
										<Wrench
											className={`mr-2 h-4 w-4 ${installM.isPending ? "animate-spin" : ""}`}
										/>
										{managedReady
											? t("runtime:buttons.reinstall", {
												defaultValue: "Reinstall",
											})
											: t("runtime:buttons.install", {
												defaultValue: "Install",
											})}
									</Button>
									<Button
										variant="outline"
										size="sm"
										className="shrink-0"
										disabled={resetOneM.isPending}
										onClick={() => setConfirm({ type: "resetOne", key })}
									>
										<RefreshCw
											className={`mr-2 h-4 w-4 ${resetOneM.isPending ? "animate-spin" : ""}`}
										/>
										{t("runtime:buttons.resetCache")}
									</Button>
								</footer>
							</div>
						);
					})}
				</div>
			)}

			{/* Capabilities Cache */}
			<Card>
				<CardHeader>
					<div className="flex items-center justify-between">
						<CardTitle>{t("runtime:capabilities.title")}</CardTitle>
						<Button
							variant="outline"
							size="sm"
							disabled={capResetM.isPending || capabilitiesStatsQ.isLoading}
							onClick={() => setConfirm({ type: "capabilitiesReset" })}
						>
							<RefreshCw
								className={`mr-2 h-4 w-4 ${capResetM.isPending ? "animate-spin" : ""}`}
							/>
							{t("runtime:capabilities.reset", {
								defaultValue: "Reset Capabilities",
							})}
						</Button>
					</div>
				</CardHeader>
				<CardContent>
					{capabilitiesStatsQ.isLoading ? (
						<div className="space-y-4">
							{/* Simplified content blocks */}
							<div className="h-20 animate-pulse rounded bg-slate-100 dark:bg-slate-900" />
							<div className="h-16 animate-pulse rounded bg-slate-100 dark:bg-slate-900" />
						</div>
					) : capStats ? (
						<div className="space-y-4 text-sm">
							<div className="grid gap-2 md:grid-cols-3">
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.rawSnapshots")}
									</span>
									<span>{capStats.storage.memory.rawSnapshotEntries}</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.projections")}
									</span>
									<span>{capStats.storage.memory.projectionEntries}</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.records")}
									</span>
									<span>{capStats.storage.catalog.records}</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.generated")}
									</span>
									<span>
										{formatLocalDateTime(capStats.generatedAt)}
									</span>
								</div>
							</div>

							<div className="mt-2 grid gap-2 md:grid-cols-3">
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.snapshots")}
									</span>
									<span>{capStats.storage.catalog.snapshots}</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.tools")}
									</span>
									<span>{capStats.storage.catalog.tools}</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.resources")}
									</span>
									<span>{capStats.storage.catalog.resources}</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.prompts")}
									</span>
									<span>{capStats.storage.catalog.prompts}</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.resourceTemplates")}
									</span>
									<span>{capStats.storage.catalog.resourceTemplates}</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.invalidatedSnapshots")}
									</span>
									<span>{capStats.storage.catalog.invalidatedSnapshots}</span>
								</div>
							</div>

							<div className="mt-4 grid gap-2 md:grid-cols-3">
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.totalQueries")}
									</span>
									<span>{capStats.metrics.totalQueries}</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.cacheHits")}
									</span>
									<span>{capStats.metrics.cacheHits}</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.cacheMisses")}
									</span>
									<span>{capStats.metrics.cacheMisses}</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.hitRatio")}
									</span>
									<span>{capStats.metrics.hitRatio.toFixed(2)}</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.singleFlightWaits")}
									</span>
									<span>{capStats.metrics.singleFlightWaits}</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.evictions")}
									</span>
									<span>{capStats.metrics.evictions}</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.invalidations")}
									</span>
									<span>{capStats.metrics.cacheInvalidations}</span>
								</div>
							</div>

							{/* Reset Capabilities button removed as requested */}
						</div>
					) : (
						<p className="text-sm text-slate-500">
							{t("runtime:capabilities.noData")}
						</p>
					)}
				</CardContent>
			</Card>
			{/* Global confirmation dialog */}
			<ConfirmDialog
				isOpen={confirm !== null}
				onClose={() => setConfirm(null)}
				onConfirm={async () => {
					if (!confirm) return;
					if (confirm.type === "resetAll") {
						resetAllM.mutate();
					} else if (confirm.type === "resetOne") {
						resetOneM.mutate(confirm.key);
					} else if (confirm.type === "install") {
						installM.mutate(confirm.key);
					} else if (confirm.type === "capabilitiesReset") {
						capResetM.mutate();
					}
				}}
				title={confirmTitle}
				description={confirmDescription}
				confirmLabel={confirmLabel}
				cancelLabel={t("runtime:dialogs.cancel")}
				variant={confirmVariant}
				isLoading={confirmLoading}
			/>
		</div>
	);
}

export default RuntimePage;
