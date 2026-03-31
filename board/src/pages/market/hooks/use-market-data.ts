import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useCursorPagination } from "../../../hooks/use-cursor-pagination";
import {
	fetchCachedRegistryServers,
	getOfficialMeta,
	syncRegistry,
} from "../../../lib/registry";
import { useAppStore } from "../../../lib/store";
import type { RegistryServerEntry } from "../../../lib/types";
import type { UseMarketDataReturn } from "../types";
import { getRegistryIdentity } from "../utils";

export function useMarketData(
	search: string,
	sort: "recent" | "name",
): UseMarketDataReturn {
	const queryClient = useQueryClient();
	const [itemsPerPage, setItemsPerPage] = useState(9);
	const [isPaginationActionLoading, setIsPaginationActionLoading] = useState(false);
	const marketBlacklist = useAppStore(
		(state) => state.dashboardSettings.marketBlacklist,
	);

	const handlePaginationReset = useCallback(() => {
		queryClient.removeQueries({ queryKey: ["market", "registry"] });
	}, [queryClient]);

	const pagination = useCursorPagination({
		limit: itemsPerPage,
		onReset: handlePaginationReset,
	});

	const registryQuery = useQuery({
		queryKey: ["market", "registry", search, pagination.currentPage, itemsPerPage],
		queryFn: async () => {
			const result = await fetchCachedRegistryServers({
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
			await syncRegistry();
			queryClient.removeQueries({ queryKey: ["market", "registry"] });
			return registryQuery.refetch();
		},
		onSuccess: () => {},
	});

	// Update pagination state when query data changes
	useEffect(() => {
		if (registryQuery.data?.metadata) {
			pagination.setHasNextPage(
				Boolean(registryQuery.data.metadata.nextCursor),
			);
		}
	}, [registryQuery.data?.metadata, pagination]);

	const blacklistIds = useMemo(() => {
		return new Set(marketBlacklist.map((entry) => entry.serverId));
	}, [marketBlacklist]);

	const servers = useMemo(() => {
		if (!registryQuery.data) return [] as RegistryServerEntry[];
		const dedup = new Map<string, RegistryServerEntry>();

		// Process servers from current page
		for (const serverEntry of registryQuery.data.servers) {
			// Extract the actual server data from the nested structure
			const server = serverEntry.server;
			if (!server) continue;

			// Create a flattened server object with _meta at the top level
			const flattenedServer: RegistryServerEntry = {
				...server,
				_meta: serverEntry._meta,
			};

			const key = getRegistryIdentity(flattenedServer);
			if (blacklistIds.has(key)) {
				continue;
			}
			const official = getOfficialMeta(flattenedServer);
			if (!dedup.has(key)) {
				dedup.set(key, flattenedServer);
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
				dedup.set(key, flattenedServer);
			}
		}
		return Array.from(dedup.values());
	}, [registryQuery.data, blacklistIds]);

	const sortedServers = useMemo(() => {
		const items = [...servers];
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
	}, [servers, sort]);

	const isInitialLoading = registryQuery.isLoading && !registryQuery.data;
	const isPageLoading = registryQuery.isFetching && Boolean(registryQuery.data);
	const isEmpty = !isInitialLoading && sortedServers.length === 0;
	const fetchError =
		registryQuery.error instanceof Error ? registryQuery.error : undefined;

	const handleRefresh = useCallback(() => {
		queryClient.removeQueries({ queryKey: ["market", "registry"] });
		void registryQuery.refetch();
	}, [queryClient, registryQuery]);

	const handleNextPage = useCallback(() => {
		if (!registryQuery.data?.metadata?.nextCursor) return;
		pagination.goToNextPage(registryQuery.data.metadata.nextCursor);
		window.scrollTo({ top: 0, behavior: "smooth" });
	}, [registryQuery.data?.metadata?.nextCursor, pagination]);

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
		if (!registryQuery.data?.metadata?.nextCursor) {
			return;
		}

		setIsPaginationActionLoading(true);
		try {
			let nextCursor: string | undefined = registryQuery.data.metadata.nextCursor;
			let targetPage = pagination.currentPage;
			const history = [...pagination.cursorHistory];

			while (nextCursor) {
				if (history.length < targetPage + 1) {
					history.push(nextCursor);
				} else {
					history[targetPage] = nextCursor;
				}
				targetPage += 1;

				const result = await fetchCachedRegistryServers({
					cursor: nextCursor,
					search: search || undefined,
					limit: itemsPerPage,
				});
				nextCursor = result.metadata?.nextCursor;
			}

			pagination.setPaginationState(targetPage, history, false);
			window.scrollTo({ top: 0, behavior: "smooth" });
		} finally {
			setIsPaginationActionLoading(false);
		}
	}, [itemsPerPage, pagination, registryQuery.data?.metadata?.nextCursor, search]);

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
			if (!registryQuery.data?.metadata?.nextCursor) {
				return;
			}

			setIsPaginationActionLoading(true);
			try {
				let nextCursor: string | undefined = registryQuery.data.metadata.nextCursor;
				let targetPagePtr = pagination.currentPage;
				const history = [...pagination.cursorHistory];

				while (nextCursor && targetPagePtr < p) {
					if (history.length < targetPagePtr + 1) {
						history.push(nextCursor);
					} else {
						history[targetPagePtr] = nextCursor;
					}
					targetPagePtr += 1;

					const result = await fetchCachedRegistryServers({
						cursor: nextCursor,
						search: search || undefined,
						limit: itemsPerPage,
					});
					nextCursor = result.metadata?.nextCursor ?? undefined;
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
			registryQuery.data?.metadata?.nextCursor,
			search,
		],
	);

	const totalPages =
		registryQuery.data && !registryQuery.data.metadata?.nextCursor
			? pagination.currentPage
			: null;

	return {
		servers: sortedServers,
		sortedServers,
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
		lastSyncedAt: registryQuery.data?.last_synced_at,
		onSync: async () => {
			await syncMutation.mutateAsync();
		},
		isSyncing: syncMutation.isPending,
	};
}
