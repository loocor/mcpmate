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
	CapabilitiesStatsResponse,
	ClearCacheResponse,
	InstallResponse,
	RuntimeCacheResponse,
	RuntimeStatusResponse,
} from "../../lib/types";
import { formatBytes, formatLocalDateTime } from "../../lib/utils";

export function RuntimePage() {
	usePageTranslations("runtime");
    const { t } = useTranslation();
	const qc = useQueryClient();
	const [confirm, setConfirm] = React.useState<
		| { type: "resetAll" }
		| { type: "resetOne"; key: "uv" | "bun" }
		| { type: "install"; key: "uv" | "bun" }
		| { type: "capabilitiesReset" }
		| null
	>(null);

	// Queries
	const runtimeStatusQ = useQuery({
		queryKey: ["runtimeStatus"],
		queryFn: runtimeApi.getStatus,
		refetchInterval: 60_000,
	});

	const runtimeCacheQ = useQuery({
		queryKey: ["runtimeCache"],
		queryFn: runtimeApi.getCache,
		refetchInterval: 60_000,
	});

	const capabilitiesStatsQ = useQuery({
		queryKey: ["capabilities", "stats"],
		queryFn: capabilitiesApi.getStats,
		refetchInterval: 60_000,
	});

	// Mutations
	const resetAllM = useMutation<{ success: boolean }, Error, void>({
		mutationFn: async () => runtimeApi.resetCache("all"),
		onSuccess: () => {
			qc.invalidateQueries({ queryKey: ["runtimeCache"] });
			notifySuccess("Caches reset", "All runtime caches cleared.");
			setConfirm(null);
		},
		onError: (e) => notifyError("Reset failed", e.message),
	});

	const resetOneM = useMutation<ClearCacheResponse, Error, "uv" | "bun">({
		mutationFn: async (kind) => runtimeApi.resetCache(kind),
		onSuccess: (_data, kind) => {
			qc.invalidateQueries({ queryKey: ["runtimeCache"] });
			notifySuccess("Cache reset", `${kind.toUpperCase()} cache cleared.`);
			setConfirm(null);
		},
		onError: (e) => notifyError("Reset failed", e.message),
	});

	const installM = useMutation<InstallResponse, Error, "uv" | "bun">({
		mutationFn: async (kind) =>
			runtimeApi.install({ runtime_type: kind, verbose: true }),
		onSuccess: (data, kind) => {
			qc.invalidateQueries({ queryKey: ["runtimeStatus"] });
			notifySuccess(
				"Install complete",
				`${kind.toUpperCase()}: ${data.message}`,
			);
			setConfirm(null);
		},
		onError: (e) => notifyError("Install failed", e.message),
	});

	// Capabilities cache reset
	const capResetM = useMutation<ClearCacheResponse, Error, void>({
		mutationFn: async () => capabilitiesApi.reset(),
		onSuccess: () => {
			qc.invalidateQueries({ queryKey: ["capabilities", "stats"] });
			notifySuccess(
				"Capabilities cache cleared",
				"Capability data will be rehydrated on next access.",
			);
			setConfirm(null);
		},
		onError: (e) => notifyError("Reset failed", e.message),
	});

	const isBusy =
		resetAllM.isPending || resetOneM.isPending || installM.isPending;

	const status = runtimeStatusQ.data as RuntimeStatusResponse | undefined;
	const cache = runtimeCacheQ.data as RuntimeCacheResponse | undefined;
	const capStats = capabilitiesStatsQ.data as
		| CapabilitiesStatsResponse
		| undefined;

	const kinds: Array<"uv" | "bun"> = ["uv", "bun"];

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
				<div className="grid gap-4 md:grid-cols-2">
					{[0, 1].map((i) => (
						<div
							key={i}
							className="rounded-lg border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-950"
						>
							{/* Status badge only */}
							<div className="flex justify-end mb-4">
								<div className="h-6 w-16 animate-pulse rounded-full bg-slate-100 dark:bg-slate-900" />
							</div>

							{/* Main content blocks - simplified */}
							<div className="space-y-3">
								{/* Main info block */}
								<div className="h-16 animate-pulse rounded bg-slate-100 dark:bg-slate-900" />

								{/* Cache info block */}
								<div className="h-12 animate-pulse rounded bg-slate-100 dark:bg-slate-900" />
							</div>

							{/* Single button */}
							<div className="mt-4 flex justify-start">
								<div className="h-8 w-24 animate-pulse rounded bg-slate-100 dark:bg-slate-900" />
							</div>
						</div>
					))}
				</div>
			) : (
				<div className="grid gap-4 md:grid-cols-2">
					{kinds.map((key) => {
						const st = status?.[key];
						const c = cache?.[key];
						const statusStr = st?.available ? "running" : "stopped";
						const p = st?.path ? st.path.replace(/\\/g, "/") : "";
						const folder = p.includes("/")
							? p.slice(0, p.lastIndexOf("/"))
							: "";

						return (
							<div
								key={key}
								className="rounded-lg border border-slate-200 bg-white p-4 shadow-sm transition-shadow hover:border-primary/40 hover:shadow-md dark:border-slate-800 dark:bg-slate-950"
							>
								<div className="flex items-center justify-between mb-2">
									<div className="flex items-center gap-2">
										<Wrench className="h-4 w-4 text-slate-500" />
										<div className="font-semibold uppercase">
											{t(`runtime:types.${key}`)}
										</div>
									</div>
									<StatusBadge status={statusStr} />
								</div>

								<div className="space-y-1 text-sm">
									<div className="flex items-center justify-between">
										<span className="text-slate-500">
											{t("runtime:labels.version")}
										</span>
										<span>{st?.version || "N/A"}</span>
									</div>
									{folder ? (
										<div className="flex items-center justify-between">
											<span className="text-slate-500">
												{t("runtime:labels.folder")}
											</span>
											<span className="truncate max-w-[60%]" title={folder}>
												{folder}
											</span>
										</div>
									) : null}
									<div className="flex items-center justify-between">
										<span className="text-slate-500">
											{t("runtime:labels.message")}
										</span>
										<span
											className="truncate max-w-[60%]"
											title={st?.message || ""}
										>
											{st?.message || "—"}
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
                                    : "—"}
										</span>
									</div>
								</div>

								<div className="mt-4 flex items-center gap-2">
									<Button
										size="sm"
										disabled={installM.isPending}
										onClick={() => setConfirm({ type: "install", key })}
									>
										<Wrench
											className={`mr-2 h-4 w-4 ${installM.isPending ? "animate-spin" : ""}`}
										/>
										{t("runtime:buttons.installRepair")}
									</Button>
									<Button
										variant="outline"
										size="sm"
										disabled={resetOneM.isPending}
										onClick={() => setConfirm({ type: "resetOne", key })}
									>
										<RefreshCw
											className={`mr-2 h-4 w-4 ${resetOneM.isPending ? "animate-spin" : ""}`}
										/>
										{t("runtime:buttons.resetCache")}
									</Button>
								</div>
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
							<div className="grid gap-2 md:grid-cols-2">
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.dbPath")}
									</span>
									<span
										className="truncate max-w-[60%]"
										title={capStats?.storage?.db_path || ""}
									>
										{capStats?.storage?.db_path || "—"}
									</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.cacheSize")}
									</span>
									<span>{formatBytes(capStats.storage.cache_size_bytes)}</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.lastCleanup")}
									</span>
									<span>
                                {capStats.storage.last_cleanup
                                    ? formatLocalDateTime(capStats.storage.last_cleanup)
                                    : "—"}
									</span>
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
										{t("runtime:capabilities.labels.servers")}
									</span>
									<span>{capStats.storage.tables.servers}</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.tools")}
									</span>
									<span>{capStats.storage.tables.tools}</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.resources")}
									</span>
									<span>{capStats.storage.tables.resources}</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.prompts")}
									</span>
									<span>{capStats.storage.tables.prompts}</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.resourceTemplates")}
									</span>
									<span>{capStats.storage.tables.resourceTemplates}</span>
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
										{t("runtime:capabilities.labels.reads")}
									</span>
									<span>{capStats.metrics.readOperations}</span>
								</div>
								<div className="flex items-center justify-between">
									<span className="text-slate-500">
										{t("runtime:capabilities.labels.writes")}
									</span>
									<span>{capStats.metrics.writeOperations}</span>
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
				title={
					confirm?.type === "resetAll"
						? t("runtime:dialogs.resetAllTitle")
						: confirm?.type === "resetOne"
							? t("runtime:dialogs.resetOneTitle", {
									key: confirm.key.toUpperCase(),
								})
							: confirm?.type === "install"
								? t("runtime:dialogs.installTitle", {
										key: confirm.key.toUpperCase(),
									})
								: t("runtime:capabilities.resetConfirmTitle", {
										defaultValue: "Reset capabilities cache?",
									})
				}
				description={
					confirm?.type === "resetAll"
						? t("runtime:dialogs.resetAllDescription")
						: confirm?.type === "resetOne"
							? t("runtime:dialogs.resetOneDescription", {
									key: confirm.key.toUpperCase(),
								})
							: confirm?.type === "install"
								? t("runtime:dialogs.installDescription", {
										key: confirm.key.toUpperCase(),
									})
								: t("runtime:capabilities.resetConfirmDesc", {
										defaultValue:
											"This clears both memory and on-disk capability cache. It will be repopulated on next access.",
									})
				}
				confirmLabel={
					confirm?.type === "install"
						? t("runtime:dialogs.installRepair")
						: t("runtime:dialogs.confirm")
				}
				cancelLabel={t("runtime:dialogs.cancel")}
				variant={confirm?.type === "install" ? "default" : "destructive"}
				isLoading={
					confirm?.type === "resetAll"
						? resetAllM.isPending
						: confirm?.type === "resetOne"
							? resetOneM.isPending
							: confirm?.type === "install"
								? installM.isPending
								: confirm?.type === "capabilitiesReset"
									? capResetM.isPending
									: false
				}
			/>
		</div>
	);
}

export default RuntimePage;
