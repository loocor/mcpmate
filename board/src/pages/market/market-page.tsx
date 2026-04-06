import { ArrowUp } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { ErrorDisplay } from "../../components/error-display";
import { Button } from "../../components/ui/button";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { notifyInfo } from "../../lib/notify";
import { useAppStore } from "../../lib/store";
import type { RegistryServerEntry } from "../../lib/types";
import { useMarketData } from "./hooks/use-market-data";
import { MarketSearch } from "./market-search";
import { ServerGrid } from "./server-grid";
import type { SortOption } from "./types";
import { formatServerName, getRegistryIdentity, useDebouncedValue } from "./utils";

export function MarketPage() {
	const { t } = useTranslation();
	usePageTranslations("market");
	const navigate = useNavigate();

	const [search, setSearch] = useState("");
	const [sort, setSort] = useState<SortOption>("recent");
	const debouncedSearch = useDebouncedValue(search.trim(), 300);
	const [showScrollTop, setShowScrollTop] = useState(false);

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
		lastSyncedAt,
		onSync,
		isSyncing,
	} = useMarketData(debouncedSearch, sort);

	const addToMarketBlacklist = useAppStore((state) => state.addToMarketBlacklist);
	const enableMarketBlacklist = useAppStore(
		(state) => state.dashboardSettings.enableMarketBlacklist,
	);

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
			navigate(`/market/${encodeURIComponent(entry.name)}`);
		},
		[navigate],
	);

	useEffect(() => {
		const handler = () => setShowScrollTop(window.scrollY > 400);
		handler();
		window.addEventListener("scroll", handler, { passive: true });
		return () => window.removeEventListener("scroll", handler);
	}, []);

	return (
		<div className="min-w-0 space-y-4">
			<div className="sticky top-0 z-10 rounded-b-xl backdrop-blur">
				<div className="flex items-center gap-2 min-w-0">
					<p className="flex-1 min-w-0 truncate whitespace-nowrap text-base text-muted-foreground">
						{t("market:title", { defaultValue: "Market" })}
					</p>
					<MarketSearch
						search={search}
						onSearchChange={setSearch}
						sort={sort}
						onSortChange={setSort}
						isLoading={isPageLoading}
						lastSyncedAt={lastSyncedAt}
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
				isInitialLoading={isInitialLoading}
				isPageLoading={isPageLoading}
				isEmpty={isEmpty}
				pagination={pagination}
				onServerPreview={handleOpenDetailPage}
				onServerInstall={handleOpenDetailPage}
				onServerHide={handleHideServer}
				enableBlacklist={enableMarketBlacklist}
				onNextPage={onNextPage}
				onPreviousPage={onPreviousPage}
				onFirstPage={onFirstPage}
				onLastPage={onLastPage}
				onGoToPage={onGoToPage}
				onItemsPerPageChange={onItemsPerPageChange}
				isPaginationActionLoading={isPaginationActionLoading}
			/>

			{showScrollTop ? (
				<Button
					variant="outline"
					size="sm"
					onClick={() => window.scrollTo({ top: 0, behavior: "smooth" })}
					className="fixed bottom-16 right-14 z-30 shadow-lg"
				>
					<ArrowUp className="mr-2 h-4 w-4" />
					{t("market:buttons.top", { defaultValue: "Top" })}
				</Button>
			) : null}
		</div>
	);
}
