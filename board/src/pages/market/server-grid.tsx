import { AlertCircle, RefreshCw, Search } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import {
	EmptyState,
	FullHeightEmptyStateCard,
} from "../../components/page-layout";
import { Button } from "../../components/ui/button";
import { buildRegistryServerKey } from "../../lib/registry";
import { MARKET_CARD_GRID_CLASS, MarketCard, MarketCardSkeleton } from "./market-card";
import type { ServerGridProps } from "./types";

interface MarketGridStateProps {
	icon: ReactNode;
	title: string;
	description: string;
	action: ReactNode;
}

function MarketGridState({
	icon,
	title,
	description,
	action,
}: MarketGridStateProps): ReactNode {
	return (
		<FullHeightEmptyStateCard>
			<EmptyState
				icon={icon}
				title={title}
				description={description}
				action={action}
			/>
		</FullHeightEmptyStateCard>
	);
}

export function ServerGrid({
	servers,
	installedRegistryServerKeys,
	isInitialLoading,
	isPageLoading,
	isEmpty,
	fetchError,
	hasActiveSearch,
	pagination,
	onRetry,
	onClearSearch,
	onServerPreview,
	onServerInstall,
	onServerHide,
	enableBlacklist,
}: ServerGridProps) {
	const { t } = useTranslation();
	const showSkeletonGrid = isInitialLoading || isPageLoading;

	let content: ReactNode;
	if (showSkeletonGrid) {
		content = (
			<div className={MARKET_CARD_GRID_CLASS}>
				{Array.from({ length: pagination.itemsPerPage }, (_, index) => (
					<MarketCardSkeleton
						key={`market-card-skeleton-${index}`}
						enableBlacklist={enableBlacklist}
					/>
				))}
			</div>
		);
	} else if (fetchError) {
		content = (
			<MarketGridState
				icon={<AlertCircle className="h-12 w-12" />}
				title={t("market:errors.failedToLoadRegistry", {
					defaultValue: "Failed to load registry",
				})}
				description={fetchError.message}
				action={
					<Button onClick={onRetry} variant="outline">
						<RefreshCw className="mr-2 h-4 w-4" />
						{t("market:buttons.retry", { defaultValue: "Retry" })}
					</Button>
				}
			/>
		);
	} else if (isEmpty) {
		content = (
			<MarketGridState
				icon={<Search className="h-12 w-12" />}
				title={t("market:emptyState.noEntriesTitle", {
					defaultValue: "No servers found",
				})}
				description={t("market:emptyState.noEntriesMatched", {
					defaultValue:
						"No entries matched your filters. Try another name or clear the search above.",
				})}
				action={
					hasActiveSearch ? (
						<Button onClick={onClearSearch} variant="outline">
							{t("market:emptyState.clearSearch", { defaultValue: "Clear search" })}
						</Button>
					) : (
						<Button onClick={onRetry} variant="outline">
							<RefreshCw className="mr-2 h-4 w-4" />
							{t("market:buttons.refresh", { defaultValue: "Refresh" })}
						</Button>
					)
				}
			/>
		);
	} else {
		content = (
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
		);
	}

	return (
		<div className="min-h-0 flex-1 overflow-y-auto pt-0.5">
			{content}
		</div>
	);
}
