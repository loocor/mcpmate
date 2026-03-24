import { ArrowUp } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ErrorDisplay } from "../../components/error-display";
import { Button } from "../../components/ui/button";
import { ServerInstallWizard } from "../../components/uniimport/server-install-wizard";
import type { ServerInstallDraft } from "../../hooks/use-server-install-pipeline";
import { useServerInstallPipeline } from "../../hooks/use-server-install-pipeline";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { notifyInfo } from "../../lib/notify";
import { useAppStore } from "../../lib/store";
import type { RegistryServerEntry } from "../../lib/types";
import { useMarketData } from "./hooks/use-market-data";
import { MarketSearch } from "./market-search";
import { ServerGrid } from "./server-grid";
import type { RemoteOption, SortOption } from "./types";
import {
	buildDraftFromRemoteOption,
	formatServerName,
	getRegistryIdentity,
	getRemoteTypeLabel,
	normalizeRemoteKind,
	slugifyForConfig,
	useDebouncedValue,
} from "./utils";

export function MarketPage() {
	const { t } = useTranslation();
	usePageTranslations("market");

	const [search, setSearch] = useState("");
	const [sort, setSort] = useState<SortOption>("recent");
	const debouncedSearch = useDebouncedValue(search.trim(), 300);

	const {
		servers,
		isInitialLoading,
		isPageLoading,
		isEmpty,
		fetchError,
		pagination,
		onNextPage,
		onPreviousPage,
		onRefresh,
	} = useMarketData(debouncedSearch, sort);

	const addToMarketBlacklist = useAppStore(
		(state) => state.addToMarketBlacklist,
	);
	const enableMarketBlacklist = useAppStore(
		(state) => state.dashboardSettings.enableMarketBlacklist,
	);

	const [showScrollTop, setShowScrollTop] = useState(false);
	const [drawerServer, setDrawerServer] = useState<RegistryServerEntry | null>(
		null,
	);
	const [drawerOpen, setDrawerOpen] = useState(false);
	const [selectedTransportId, setSelectedTransportId] = useState<string>("");

	const [remoteOptions, setRemoteOptions] = useState<RemoteOption[]>([]);
	const [selectedRemote, setSelectedRemote] = useState<RemoteOption | null>(
		null,
	);
	const handleRefreshClick = useCallback(() => {
		onRefresh();
	}, [onRefresh]);

	const handleHideServer = (entry: RegistryServerEntry) => {
		const identity = getRegistryIdentity(entry);
		const label = formatServerName(entry.name);
		addToMarketBlacklist({
			serverId: identity,
			label,
			hiddenAt: Date.now(),
		});
		notifyInfo(
			t("market:notifications.serverHidden", { defaultValue: "Server hidden" }),
			`${label} will be excluded from Market.`,
		);
	};

	const handleOpenDrawer = (entry: RegistryServerEntry) => {
		setDrawerServer(entry);
		setDrawerOpen(true);
	};

	const handleDrawerChange = useCallback((open: boolean) => {
		setDrawerOpen(open);
		if (!open) {
			setDrawerServer(null);
		}
	}, []);

	const scrollToTop = () => {
		window.scrollTo({ top: 0, behavior: "smooth" });
	};

	const installPipeline = useServerInstallPipeline({
		onImported: () => {
			setDrawerOpen(false);
			setDrawerServer(null);
		},
	});

	useEffect(() => {
		if (!drawerServer) {
			setRemoteOptions([]);
			setSelectedRemote(null);
			return;
		}

		const options: RemoteOption[] = [];

		(drawerServer.remotes ?? []).forEach((remote, idx) => {
			const kind = normalizeRemoteKind(remote.type);
			if (!kind || !remote?.url) return;
			options.push({
				id: `${drawerServer.name}-remote-${idx}`,
				label: `${getRemoteTypeLabel(kind)} • ${remote.url}`,
				kind,
				source: "remote",
				url: remote.url,
				headers: remote.headers ?? null,
				envVars: null,
				packageIdentifier: null,
				packageMeta: null,
			});
		});

		(drawerServer.packages ?? []).forEach((pkg, idx) => {
			const kind = normalizeRemoteKind(pkg.transport?.type);
			if (!kind) return;
			const identifier =
				pkg.identifier ?? pkg.registryType ?? `package-${idx + 1}`;
			options.push({
				id: `${drawerServer.name}-package-${idx}`,
				label: `${getRemoteTypeLabel(kind)} • ${identifier}`,
				kind,
				source: "package",
				url: null,
				headers: null,
				envVars: pkg.environmentVariables ?? null,
				packageIdentifier: identifier,
				packageMeta: pkg,
			});
		});

		setRemoteOptions(options);
		if (options.length > 0) {
			const defaultOption = options[0];
			setSelectedRemote(defaultOption);
			setSelectedTransportId(defaultOption.id);
		} else {
			setSelectedRemote(null);
			setSelectedTransportId("");
		}
	}, [drawerServer]);

	useEffect(() => {
		if (!selectedTransportId || !remoteOptions.length) return;
		const option = remoteOptions.find((opt) => opt.id === selectedTransportId);
		if (option) setSelectedRemote(option);
	}, [selectedTransportId, remoteOptions]);

	const initialDraft = useMemo<ServerInstallDraft | undefined>(() => {
		if (!selectedRemote || !drawerServer) return undefined;
		const fallbackName = slugifyForConfig(drawerServer.name);
		const draft = buildDraftFromRemoteOption(selectedRemote, fallbackName);
		const draftWithMeta: ServerInstallDraft = {
			...draft,
			meta: {
				description: drawerServer.description || "",
				version: drawerServer.version || "",
				websiteUrl: drawerServer.websiteUrl || "",
				repository: drawerServer.repository
					? {
						url: drawerServer.repository.url || "",
						source: "",
						subfolder: "",
						id: "",
					}
					: undefined,
			},
		};

		return draftWithMeta;
	}, [selectedRemote, drawerServer]);

	useEffect(() => {
		const handler = () => {
			setShowScrollTop(window.scrollY > 400);
		};
		handler();
		window.addEventListener("scroll", handler, { passive: true });
		return () => window.removeEventListener("scroll", handler);
	}, []);

	return (
		<>
			<div className="space-y-4">
				<div className="sticky top-0 z-10 -mx-1 rounded-b-xl px-1 backdrop-blur">
					<div className="flex items-center gap-2 min-w-0">
						<p className="flex-1 min-w-0 truncate whitespace-nowrap text-base text-muted-foreground">
							{t("market:title", { defaultValue: "Market" })}
						</p>

						<MarketSearch
							search={search}
							onSearchChange={setSearch}
							sort={sort}
							onSortChange={setSort}
							onRefresh={handleRefreshClick}
							isLoading={isPageLoading}
						/>
					</div>
				</div>

				<ErrorDisplay
					title={t("market:errors.failedToLoadRegistry", {
						defaultValue: "Failed to load registry",
					})}
					error={fetchError ?? null}
					onRetry={handleRefreshClick}
				/>

				<ServerGrid
					servers={servers}
					isInitialLoading={isInitialLoading}
					isPageLoading={isPageLoading}
					isEmpty={isEmpty}
					pagination={pagination}
					onServerPreview={handleOpenDrawer}
					onServerHide={handleHideServer}
					enableBlacklist={enableMarketBlacklist}
					onNextPage={onNextPage}
					onPreviousPage={onPreviousPage}
				/>

				{showScrollTop ? (
					<Button
						variant="outline"
						size="sm"
						onClick={scrollToTop}
						className="fixed bottom-16 right-14 z-30 shadow-lg"
					>
						<ArrowUp className="mr-2 h-4 w-4" />
						{t("market:buttons.top", { defaultValue: "Top" })}
					</Button>
				) : null}
			</div>

			{drawerOpen && remoteOptions.length > 1 && (
				<div className="fixed inset-0 z-50 flex items-end justify-center p-4 sm:items-center">
					<button
						type="button"
						className="fixed inset-0 bg-black/50"
						onClick={() => handleDrawerChange(false)}
						onKeyDown={(e) => {
							if (e.key === "Escape") {
								handleDrawerChange(false);
							}
						}}
						aria-label="Close transport selector"
					/>
					<div className="relative w-full max-w-md rounded-lg bg-white p-6 shadow-lg dark:bg-slate-800">
						<h3 className="text-lg font-semibold mb-4">
							{t("market:transport.selectOption", {
								defaultValue: "Select Transport Option",
							})}
						</h3>
						<div className="space-y-2">
							{remoteOptions.map((option) => (
								<button
									type="button"
									key={option.id}
									onClick={() => {
										setSelectedTransportId(option.id);
										handleDrawerChange(false);
									}}
									className={`w-full text-left p-3 rounded-lg border transition-colors ${selectedTransportId === option.id
											? "border-primary bg-primary/5"
											: "border-slate-200 hover:border-slate-300 dark:border-slate-700 dark:hover:border-slate-600"
										}`}
								>
									<div className="font-medium">{option.label}</div>
									<div className="text-sm text-slate-500 dark:text-slate-400">
										{option.source === "remote"
											? t("market:transport.remoteEndpoint", {
												defaultValue: "Remote endpoint",
											})
											: t("market:transport.packageInstallation", {
												defaultValue: "Package installation",
											})}
									</div>
								</button>
							))}
						</div>
					</div>
				</div>
			)}

			<ServerInstallWizard
				isOpen={drawerOpen}
				onClose={() => handleDrawerChange(false)}
				mode="import"
				initialDraft={initialDraft}
				allowProgrammaticIngest
				pipeline={installPipeline}
			/>
		</>
	);
}
