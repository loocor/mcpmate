import { useTranslation } from "react-i18next";
import { buildRegistryServerKey } from "../../lib/registry";
import { MARKET_CARD_GRID_CLASS, MarketCard, MarketCardSkeleton } from "./market-card";
import type { ServerGridProps } from "./types";

export function ServerGrid({
	servers,
	installedRegistryServerKeys,
	isInitialLoading,
	isPageLoading,
	isEmpty,
	pagination,
	onServerPreview,
	onServerInstall,
	onServerHide,
	enableBlacklist,
}: ServerGridProps) {
	const { t } = useTranslation();
	const showSkeletonGrid = isInitialLoading || isPageLoading;

	return (
		<div className="min-h-0 flex-1 pt-0.5 overflow-y-auto">
			{showSkeletonGrid ? (
				<div className={MARKET_CARD_GRID_CLASS}>
					{Array.from({ length: pagination.itemsPerPage }, (_, index) => (
						<MarketCardSkeleton
							key={`market-card-skeleton-${index}`}
							enableBlacklist={enableBlacklist}
						/>
					))}
				</div>
			) : isEmpty ? (
				<div className="rounded-xl border border-dashed border-slate-200 bg-white py-12 text-center text-sm text-slate-500 shadow-sm dark:border-slate-700 dark:bg-slate-900 dark:text-slate-400">
					{t("market:emptyState.noEntriesMatched", {
						defaultValue:
							"No entries matched your filters. Try another name or clear the search above.",
					})}
				</div>
			) : (
				<div className={MARKET_CARD_GRID_CLASS}>
					{servers.map((server) => (
						<MarketCard
							key={`${server.name}-${server.version}`}
							server={server}
							isInstalled={installedRegistryServerKeys.has(buildRegistryServerKey(server))}
							onPreview={onServerPreview}
							onInstall={onServerInstall}
							onHide={onServerHide}
							enableBlacklist={enableBlacklist}
						/>
					))}
				</div>
			)}
		</div>
	);
}
