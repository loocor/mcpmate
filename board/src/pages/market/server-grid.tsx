import { Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Pagination } from "../../components/pagination";
import { Card, CardFooter, CardHeader } from "../../components/ui/card";
import { MarketCard } from "./market-card";
import type { ServerGridProps } from "./types";

export function ServerGrid({
	servers,
	isInitialLoading,
	isPageLoading,
	isEmpty,
	pagination,
	onServerPreview,
	onServerHide,
	enableBlacklist,
	onNextPage,
	onPreviousPage,
}: ServerGridProps) {
	const { t } = useTranslation();
	return (
		<>
			{/* Loading Skeleton */}
			{isInitialLoading ? (
				<div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
					{Array.from({ length: 9 }, (_, index) => {
						const uniqueKey = `skeleton-card-${Date.now()}-${index}`;
						return (
							<Card
								key={uniqueKey}
								className="group flex h-full cursor-pointer flex-col overflow-hidden border border-slate-200 transition-all duration-200 hover:border-primary/40 hover:shadow-xl hover:-translate-y-0.5 dark:border-slate-800"
							>
								<CardHeader className="p-4">
									<div className="grid grid-cols-1 grid-rows-1">
										<div className="flex items-start gap-3 col-start-1 row-start-1">
											<div className="h-12 w-12 rounded-[10px] bg-slate-200 animate-pulse dark:bg-slate-700" />
											<div className="flex-1 space-y-2">
												<div className="h-5 w-3/4 rounded bg-slate-200 animate-pulse dark:bg-slate-700" />
												<div className="h-3 w-1/2 rounded bg-slate-200 animate-pulse dark:bg-slate-700" />
												<div className="h-3 w-full rounded bg-slate-200 animate-pulse dark:bg-slate-700" />
												<div className="h-3 w-2/3 rounded bg-slate-200 animate-pulse dark:bg-slate-700" />
											</div>
										</div>
										<div className="col-start-1 row-start-1 flex justify-end items-start pt-1 pr-1">
											<div className="h-6 w-16 rounded-full bg-slate-200 animate-pulse dark:bg-slate-700" />
										</div>
									</div>
								</CardHeader>
								<CardFooter className="flex items-center justify-between gap-2 px-4 pb-4 pt-0 mt-auto">
									<div className="flex items-center gap-3">
										<div className="w-12"></div>
										<div className="h-5 w-5 rounded-full bg-slate-200 animate-pulse dark:bg-slate-700" />
									</div>
									<div className="flex items-center gap-3">
										<div className="h-5 w-5 rounded-full bg-slate-200 animate-pulse dark:bg-slate-700" />
									</div>
								</CardFooter>
							</Card>
						);
					})}
				</div>
			) : null}

			{/* Empty State */}
			{isEmpty ? (
				<div className="rounded-xl border border-dashed border-slate-200 bg-white py-12 text-center text-sm text-slate-500 shadow-sm dark:border-slate-800 dark:bg-slate-950 dark:text-slate-400">
					{t("market:emptyState.noEntriesMatched", {
						defaultValue:
							"No entries matched your filters. Try another name or clear the search above.",
					})}
				</div>
			) : null}

			{/* Server Cards Grid */}
			{!isInitialLoading && !isEmpty ? (
				<div className="relative">
					{isPageLoading ? (
						<div className="absolute inset-0 z-10 flex items-center justify-center bg-white/80 backdrop-blur-sm dark:bg-slate-950/80">
							<div className="flex items-center gap-2 rounded-lg bg-white px-4 py-2 shadow-lg dark:bg-slate-800">
								<Loader2 className="h-4 w-4 animate-spin" />
								<span className="text-sm font-medium">
									{t("market:emptyState.loading", { defaultValue: "Loading..." })}
								</span>
							</div>
						</div>
					) : null}
					<div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
						{servers.map((server) => {
							return (
								<MarketCard
									key={`${server.name}-${server.version}`}
									server={server}
									onPreview={onServerPreview}
									onHide={onServerHide}
									enableBlacklist={enableBlacklist}
								/>
							);
						})}
					</div>
				</div>
			) : null}

			{/* Pagination */}
			{!isEmpty ? (
				<Pagination
					currentPage={pagination.currentPage}
					hasPreviousPage={pagination.hasPreviousPage}
					hasNextPage={pagination.hasNextPage}
					isLoading={isInitialLoading || isPageLoading}
					itemsPerPage={pagination.itemsPerPage}
					currentPageItemCount={servers.length}
					onPreviousPage={onPreviousPage}
					onNextPage={onNextPage}
					className="mt-6"
				/>
			) : null}
		</>
	);
}
