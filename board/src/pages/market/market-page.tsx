import { useCallback, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";
import { ErrorDisplay } from "../../components/error-display";
import { Pagination } from "../../components/pagination";
import { serversApi } from "../../lib/api";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { notifyInfo } from "../../lib/notify";
import { buildRegistryServerKey, matchesInstalledRegistryServer } from "../../lib/registry";
import { useAppStore } from "../../lib/store";
import type { RegistryServerEntry } from "../../lib/types";
import { useUrlSearch, useUrlState } from "../../lib/hooks/use-url-state";
import { useMarketData } from "./hooks/use-market-data";
import { rememberMarketListReturnSearch } from "./market-list-pagination-storage";
import { MARKET_PAGE_SIZE_OPTIONS } from "./market-list-pagination-storage";
import { MarketSearch } from "./market-search";
import { ServerGrid } from "./server-grid";
import type { SortOption } from "./types";
import { formatServerName, getRegistryIdentity, useDebouncedValue } from "./utils";

export function MarketPage() {
	const { t } = useTranslation();
	usePageTranslations("market");
	const navigate = useNavigate();
	const location = useLocation();
	const { search, setSearch } = useUrlSearch();
	const [sort, setSort] = useUrlState<SortOption>({
		paramName: "sort",
		defaultValue: "recent",
		validate: (value) => value === "recent" || value === "name",
	});
	const debouncedSearch = useDebouncedValue(search.trim(), 300);
	const installedServersQuery = useQuery({
		queryKey: ["servers"],
		queryFn: () => serversApi.getAll(),
		staleTime: 30_000,
	});

	const {
		servers,
		isInitialLoading,
		isPageLoading,
		isEmpty,
		fetchError,
		pagination,
		onNextPage,
		onPreviousPage,
		onFirstPage,
		onLastPage,
		onGoToPage,
		onItemsPerPageChange,
		isPaginationActionLoading,
		onRefresh,
		onSync,
		isSyncing,
	} = useMarketData(debouncedSearch, sort);

	const addToMarketBlacklist = useAppStore((state) => state.addToMarketBlacklist);
	const enableMarketBlacklist = useAppStore(
		(state) => state.dashboardSettings.enableMarketBlacklist,
	);

	const installedRegistryServerKeys = useMemo(() => {
		const installedServers = installedServersQuery.data?.servers ?? [];
		const installedKeys = new Set<string>();
		for (const registryServer of servers) {
			if (installedServers.some((installedServer) => matchesInstalledRegistryServer(registryServer, installedServer))) {
				installedKeys.add(buildRegistryServerKey(registryServer));
			}
		}
		return installedKeys;
	}, [installedServersQuery.data?.servers, servers]);

	const handleHideServer = useCallback(
		(entry: RegistryServerEntry) => {
			const identity = getRegistryIdentity(entry);
			const label = formatServerName(entry.name);
			addToMarketBlacklist({ serverId: identity, label, hiddenAt: Date.now() });
			notifyInfo(
				t("market:notifications.serverHidden", { defaultValue: "Server hidden" }),
				`${label} will be excluded from Market.`,
			);
		},
		[addToMarketBlacklist, t],
	);

	const handleOpenDetailPage = useCallback(
		(entry: RegistryServerEntry) => {
			rememberMarketListReturnSearch(location.search);
			navigate(`/market/${encodeURIComponent(entry.name)}`, {
				state: { marketListSearch: location.search },
			});
		},
		[location.search, navigate],
	);

	return (
		<div className="flex h-full min-h-0 flex-col gap-3.5">
			<div className="shrink-0 py-1">
				<div className="flex items-center gap-2 min-w-0 overflow-visible">
					<p className="flex-1 min-w-0 truncate whitespace-nowrap text-base text-muted-foreground">
						{t("market:title", { defaultValue: "Market" })}
					</p>
					<MarketSearch
						search={search}
						onSearchChange={setSearch}
						sort={sort}
						onSortChange={setSort}
						isLoading={isPageLoading}
						onSync={onSync}
						isSyncing={isSyncing}
					/>
				</div>
			</div>

			<ErrorDisplay
				title={t("market:errors.failedToLoadRegistry", {
					defaultValue: "Failed to load registry",
				})}
				error={fetchError ?? null}
				onRetry={onRefresh}
			/>

			<ServerGrid
				servers={servers}
				installedRegistryServerKeys={installedRegistryServerKeys}
				isInitialLoading={isInitialLoading}
				isPageLoading={isPageLoading}
				isEmpty={isEmpty}
				pagination={pagination}
				onServerPreview={handleOpenDetailPage}
				onServerInstall={handleOpenDetailPage}
				onServerHide={handleHideServer}
				enableBlacklist={enableMarketBlacklist}
			/>

			{!isEmpty || isInitialLoading || isPageLoading ? (
				<Pagination
					currentPage={pagination.currentPage}
					hasPreviousPage={pagination.hasPreviousPage}
					hasNextPage={pagination.hasNextPage}
					isLoading={isInitialLoading || isPageLoading || isPaginationActionLoading}
					itemsPerPage={pagination.itemsPerPage}
					currentPageItemCount={servers.length}
					totalPages={pagination.totalPages}
					disableLastPageWhenTotalUnknown
					onGoToPage={onGoToPage}
					onItemsPerPageChange={onItemsPerPageChange}
					onPreviousPage={onPreviousPage}
					onFirstPage={onFirstPage}
					onNextPage={onNextPage}
					onLastPage={onLastPage}
					pageSizeOptions={[...MARKET_PAGE_SIZE_OPTIONS]}
					className="shrink-0 pb-1"
				/>
			) : null}
		</div>
	);
}
