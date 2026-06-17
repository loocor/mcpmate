import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useCursorPagination } from "../../../hooks/use-cursor-pagination";
import { useCatalogProvider } from "../../../lib/market";
import { getOfficialMeta } from "../../../lib/registry";
import { useAppStore } from "../../../lib/store";
import type { RegistryServerEntry } from "../../../lib/types";
import type { UseMarketDataReturn } from "../types";
import { getRegistryIdentity } from "../utils";

export function useMarketData(
	search: string,
	sort: "recent" | "name",
): UseMarketDataReturn {
	const { provider } = useCatalogProvider();
	const queryClient = useQueryClient();
	const [itemsPerPage, setItemsPerPage] = useState(9);
	const [isPaginationActionLoading, setIsPaginationActionLoading] = useState(false);
	const marketBlacklist = useAppStore(
		(state) => state.dashboardSettings.marketBlacklist,
	);

	const handlePaginationReset = useCallback(() => {
		queryClient.removeQueries({ queryKey: ["market", provider.meta.id] });
	}, [queryClient, provider.meta.id]);

	const pagination = useCursorPagination({
		limit: itemsPerPage,
		onReset: handlePaginationReset,
	});

	const registryQuery = useQuery({
		queryKey: ["market", provider.meta.id, search, pagination.currentPage, itemsPerPage],
		queryFn: async () => {
			const result = await provider.fetchPage({
				cursor: pagination.currentCursor,
				search: search || undefined,
				limit: pagination.itemsPerPage,
			});
			return result;
		},
		staleTime: 1000 * 60 * 5,
	});

	const syncMutation = useMutation({
		mutationFn: async () => {
			if (provider.sync) {
				await provider.sync();
			}
			queryClient.removeQueries({ queryKey: ["market", provider.meta.id] });
			return registryQuery.refetch();
		},
	});

	// Update pagination state when query data changes
	useEffect(() => {
		if (registryQuery.data) {
			pagination.setHasNextPage(
				Boolean(registryQuery.data.nextCursor),
			);
		}
	}, [registryQuery.data, pagination]);

	const blacklistIds = useMemo(() => {
		return new Set(marketBlacklist.map((entry) => entry.serverId));
	}, [marketBlacklist]);

	const deduped = useMemo(() => {
		if (!registryQuery.data) return [] as RegistryServerEntry[];
		const dedup = new Map<string, RegistryServerEntry>();

		for (const server of registryQuery.data.entries) {
			const key = getRegistryIdentity(server);
			if (blacklistIds.has(key)) {
				continue;
			}
			const official = getOfficialMeta(server);
			if (!dedup.has(key)) {
				dedup.set(key, server);
				continue;
			}
			const existing = dedup.get(key);
			const existingTs = existing && getOfficialMeta(existing)?.updatedAt;
			const candidateTs = official?.updatedAt;
			if (
				existing &&
				existingTs &&
				candidateTs &&
				Date.parse(candidateTs) > Date.parse(existingTs)
			) {
				dedup.set(key, server);
			}
		}
		return Array.from(dedup.values());
	}, [registryQuery.data, blacklistIds]);

	const servers = useMemo(() => {
		const items = [...deduped];
		if (sort === "recent") {
			items.sort((a, b) => {
				const metaA = getOfficialMeta(a);
				const metaB = getOfficialMeta(b);
				const tsA = metaA?.updatedAt ?? metaA?.publishedAt;
				const tsB = metaB?.updatedAt ?? metaB?.publishedAt;
				return Date.parse(tsB || "0") - Date.parse(tsA || "0");
			});
		} else {
			items.sort((a, b) => a.name.localeCompare(b.name));
		}
		return items;
	}, [deduped, sort]);

	const isInitialLoading = registryQuery.isLoading && !registryQuery.data;
	const isPageLoading = registryQuery.isFetching && Boolean(registryQuery.data);
	const isEmpty = !isInitialLoading && servers.length === 0;
	const fetchError =
		registryQuery.error instanceof Error ? registryQuery.error : undefined;

	const handleRefresh = useCallback(() => {
		queryClient.removeQueries({ queryKey: ["market", provider.meta.id] });
		void registryQuery.refetch();
	}, [queryClient, provider.meta.id, registryQuery]);

	const handleNextPage = useCallback(() => {
		if (!registryQuery.data?.nextCursor) return;
		pagination.goToNextPage(registryQuery.data.nextCursor);
		window.scrollTo({ top: 0, behavior: "smooth" });
	}, [registryQuery.data?.nextCursor, pagination]);

	const handlePreviousPage = useCallback(() => {
		pagination.goToPreviousPage();
		window.scrollTo({ top: 0, behavior: "smooth" });
	}, [pagination]);

	const handleFirstPage = useCallback(() => {
		pagination.resetToFirstPage();
		window.scrollTo({ top: 0, behavior: "smooth" });
	}, [pagination]);

	const handleItemsPerPageChange = useCallback(
		(nextItemsPerPage: number) => {
			if (nextItemsPerPage === itemsPerPage) {
				return;
			}
			setItemsPerPage(nextItemsPerPage);
			pagination.resetToFirstPage();
		},
		[itemsPerPage, pagination],
	);

	const handleLastPage = useCallback(async () => {
		if (!registryQuery.data?.nextCursor) {
			return;
		}

		setIsPaginationActionLoading(true);
		try {
			let nextCursor: string | undefined = registryQuery.data.nextCursor;
			let targetPage = pagination.currentPage;
			const history = [...pagination.cursorHistory];

			const MAX_PAGES = 100;
			while (nextCursor && targetPage < MAX_PAGES) {
				if (history.length < targetPage + 1) {
					history.push(nextCursor);
				} else {
					history[targetPage] = nextCursor;
				}
				targetPage += 1;

				const result = await provider.fetchPage({
					cursor: nextCursor,
					search: search || undefined,
					limit: itemsPerPage,
				});
				nextCursor = result.nextCursor;
			}

			pagination.setPaginationState(targetPage, history, false);
			window.scrollTo({ top: 0, behavior: "smooth" });
		} finally {
			setIsPaginationActionLoading(false);
		}
	}, [itemsPerPage, pagination, registryQuery.data?.nextCursor, search, provider]);

	const handleGoToPage = useCallback(
		async (targetPage: number) => {
			const p = Math.max(1, Math.floor(targetPage));
			if (p === pagination.currentPage) {
				return;
			}
			if (p === 1) {
				handleFirstPage();
				return;
			}
			if (p < pagination.currentPage) {
				const hist = pagination.cursorHistory;
				if (p <= hist.length) {
					pagination.setPaginationState(p, hist.slice(0, p), true);
					window.scrollTo({ top: 0, behavior: "smooth" });
				} else {
					handleFirstPage();
				}
				return;
			}
			if (!registryQuery.data?.nextCursor) {
				return;
			}

			setIsPaginationActionLoading(true);
			try {
				let nextCursor: string | undefined = registryQuery.data.nextCursor;
				let targetPagePtr = pagination.currentPage;
				const history = [...pagination.cursorHistory];

				const MAX_PAGES = 100;
				const clampedTarget = Math.min(p, MAX_PAGES);
				while (nextCursor && targetPagePtr < clampedTarget) {
					if (history.length < targetPagePtr + 1) {
						history.push(nextCursor);
					} else {
						history[targetPagePtr] = nextCursor;
					}
					targetPagePtr += 1;

					const result = await provider.fetchPage({
						cursor: nextCursor,
						search: search || undefined,
						limit: itemsPerPage,
					});
					nextCursor = result.nextCursor ?? undefined;
				}

				if (targetPagePtr !== p) {
					pagination.setPaginationState(targetPagePtr, history, Boolean(nextCursor));
					window.scrollTo({ top: 0, behavior: "smooth" });
					return;
				}

				pagination.setPaginationState(p, history, Boolean(nextCursor));
				window.scrollTo({ top: 0, behavior: "smooth" });
			} finally {
				setIsPaginationActionLoading(false);
			}
		},
		[
			handleFirstPage,
			itemsPerPage,
			pagination,
			registryQuery.data?.nextCursor,
			search,
			provider,
		],
	);

	const totalPages =
		registryQuery.data && !registryQuery.data.nextCursor
			? pagination.currentPage
			: null;

	return {
		servers,
		isInitialLoading,
		isPageLoading,
		isEmpty,
		fetchError,
		pagination: {
			currentPage: pagination.currentPage,
			hasPreviousPage: pagination.hasPreviousPage,
			hasNextPage: pagination.hasNextPage,
			itemsPerPage: pagination.itemsPerPage,
			totalPages,
		},
		onNextPage: handleNextPage,
		onPreviousPage: handlePreviousPage,
		onFirstPage: handleFirstPage,
		onLastPage: handleLastPage,
		onGoToPage: handleGoToPage,
		onItemsPerPageChange: handleItemsPerPageChange,
		isPaginationActionLoading,
		onRefresh: handleRefresh,
		lastSyncedAt: registryQuery.data?.lastSyncedAt,
		onSync: provider.meta.supportsSync
			? async () => { await syncMutation.mutateAsync(); }
			: async () => { handleRefresh(); },
		isSyncing: provider.meta.supportsSync ? syncMutation.isPending : registryQuery.isFetching,
	};
}
