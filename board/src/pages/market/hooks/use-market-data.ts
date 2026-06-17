import {
	keepPreviousData,
	useMutation,
	useQuery,
	useQueryClient,
} from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { useCursorPagination } from "../../../hooks/use-cursor-pagination";
import { useCatalogProvider } from "../../../lib/market";
import { getOfficialMeta, getCanonicalRegistryServerId } from "../../../lib/registry";
import { useAppStore } from "../../../lib/store";
import type { RegistryServerEntry } from "../../../lib/types";
import type { UseMarketDataReturn } from "../types";
import {
	type MarketPageSize,
	buildMarketPaginationStorageKey,
	parseMarketListPageParam,
	parseMarketListPerPageParam,
	readStoredMarketPagination,
	writeStoredMarketPagination,
} from "../market-list-pagination-storage";
import {
	MARKET_LIST_GC_MS,
	MARKET_LIST_STALE_MS,
	marketDetailRootKey,
	marketListQueryKey,
	marketListRootKey,
} from "../market-query-keys";

const MAX_WALK_PAGES = 100;

function scrollToTop() {
	window.scrollTo({ top: 0, behavior: "smooth" });
}

interface WalkCursorsResult {
	history: (string | undefined)[];
	finalPage: number;
	hasNext: boolean;
}

async function walkCursorsToPage(
	startCursor: string | undefined,
	startPage: number,
	targetPage: number,
	fetchPage: (cursor: string | undefined) => Promise<{ nextCursor?: string | null }>,
): Promise<WalkCursorsResult> {
	let nextCursor: string | undefined = startCursor;
	let pagePtr = startPage;
	const history: (string | undefined)[] = [];

	while (nextCursor && pagePtr < targetPage && pagePtr < MAX_WALK_PAGES) {
		if (history.length < pagePtr + 1) {
			history.push(nextCursor);
		} else {
			history[pagePtr] = nextCursor;
		}
		pagePtr += 1;

		const result = await fetchPage(nextCursor);
		nextCursor = result.nextCursor ?? undefined;
	}

	return {
		history,
		finalPage: pagePtr,
		hasNext: Boolean(nextCursor),
	};
}

interface RestoredMarketPagination {
	page: number;
	history: (string | undefined)[];
	hasNextPage: boolean;
	needsRebuild: boolean;
}

function restoreMarketPagination(
	providerId: string,
	search: string,
	itemsPerPage: number,
	urlPage: number,
): RestoredMarketPagination {
	const storageKey = buildMarketPaginationStorageKey(providerId, search, itemsPerPage);
	const stored = readStoredMarketPagination(storageKey);
	if (stored && stored.page === urlPage && stored.history.length >= urlPage) {
		return {
			page: urlPage,
			history: stored.history,
			hasNextPage: stored.hasNextPage,
			needsRebuild: false,
		};
	}
	return {
		page: urlPage,
		history: [undefined],
		hasNextPage: urlPage > 1,
		needsRebuild: urlPage > 1,
	};
}

export function useMarketData(
	search: string,
	sort: "recent" | "name",
): UseMarketDataReturn {
	const { provider } = useCatalogProvider();
	const queryClient = useQueryClient();
	const [searchParams, setSearchParams] = useSearchParams();
	const providerId = provider.meta.id;
	const restoredPaginationRef = useRef<RestoredMarketPagination | null>(null);
	if (restoredPaginationRef.current === null) {
		const urlPage = parseMarketListPageParam(searchParams.get("page"));
		const urlPerPage = parseMarketListPerPageParam(searchParams.get("perPage"));
		const urlSearch = searchParams.get("q") ?? "";
		restoredPaginationRef.current = restoreMarketPagination(
			providerId,
			urlSearch,
			urlPerPage,
			urlPage,
		);
	}

	const [itemsPerPage, setItemsPerPage] = useState(() =>
		parseMarketListPerPageParam(searchParams.get("perPage")),
	);
	const [isPaginationActionLoading, setIsPaginationActionLoading] = useState(false);
	const [isRestoringPagination, setIsRestoringPagination] = useState(
		() => restoredPaginationRef.current?.needsRebuild ?? false,
	);
	const marketBlacklist = useAppStore(
		(state) => state.dashboardSettings.marketBlacklist,
	);
	const prevFiltersRef = useRef({ search, sort });

	const pagination = useCursorPagination({
		limit: itemsPerPage,
		initialState: {
			currentPage: restoredPaginationRef.current.page,
			cursorHistory: restoredPaginationRef.current.history,
			hasNextPage: restoredPaginationRef.current.hasNextPage,
		},
	});
	const {
		currentPage,
		currentCursor,
		cursorHistory,
		hasPreviousPage,
		hasNextPage,
		goToNextPage,
		goToPreviousPage,
		resetToFirstPage,
		setHasNextPage,
		setPaginationState,
	} = pagination;

	const listQueryParams = useMemo(
		() => ({
			search,
			page: currentPage,
			limit: itemsPerPage,
			cursor: currentCursor,
		}),
		[search, currentPage, currentCursor, itemsPerPage],
	);

	const listQueryKey = useMemo(
		() => marketListQueryKey(providerId, listQueryParams),
		[providerId, listQueryParams],
	);

	const registryQuery = useQuery({
		queryKey: listQueryKey,
		queryFn: async () => {
			const result = await provider.fetchPage({
				cursor: currentCursor,
				search: search || undefined,
				limit: itemsPerPage,
			});
			return result;
		},
		enabled: !isRestoringPagination,
		staleTime: MARKET_LIST_STALE_MS,
		gcTime: MARKET_LIST_GC_MS,
		placeholderData: keepPreviousData,
	});

	const invalidateCatalogCaches = useCallback(
		async (refetchActiveList: boolean) => {
			await queryClient.invalidateQueries({
				queryKey: marketListRootKey(providerId),
				refetchType: refetchActiveList ? "active" : "none",
			});
			await queryClient.invalidateQueries({
				queryKey: marketDetailRootKey(providerId),
				refetchType: "none",
			});
		},
		[providerId, queryClient],
	);

	const syncMutation = useMutation({
		mutationFn: async () => {
			if (provider.sync) {
				await provider.sync();
			}
			await invalidateCatalogCaches(false);
			return registryQuery.refetch();
		},
	});

	useEffect(() => {
		if (!isRestoringPagination) {
			return;
		}

		const targetPage = currentPage;
		const restored = restoredPaginationRef.current;
		const urlSearch = searchParams.get("q") ?? "";
		let cancelled = false;

		void (async () => {
			setIsPaginationActionLoading(true);
			try {
				// Use stored cursor history to skip pages we already know about.
				// Walk only from the stored page forward, not from page 1.
				const storedHistory = restored?.history ?? [undefined];
				const storedPage = restored?.page ?? 1;
				const startCursor = storedHistory[storedPage - 1];
				const startPage = storedPage;

				const result = await walkCursorsToPage(
					startCursor,
					startPage,
					targetPage,
					(cursor) => provider.fetchPage({
						cursor,
						search: urlSearch || undefined,
						limit: itemsPerPage,
					}),
				);

				if (cancelled) {
					return;
				}

				// Merge stored history with newly walked cursors.
				const fullHistory = [...storedHistory];
				for (let i = 0; i < result.history.length; i++) {
					const pageIdx = startPage + i;
					if (fullHistory.length < pageIdx + 1) {
						fullHistory.push(result.history[i]);
					} else {
						fullHistory[pageIdx] = result.history[i];
					}
				}

				if (result.finalPage === targetPage) {
					setPaginationState(targetPage, fullHistory, result.hasNext);
				} else {
					resetToFirstPage();
				}
			} finally {
				if (!cancelled) {
					setIsRestoringPagination(false);
					setIsPaginationActionLoading(false);
				}
			}
		})();

		return () => {
			cancelled = true;
		};
	}, [
		currentPage,
		isRestoringPagination,
		itemsPerPage,
		provider,
		resetToFirstPage,
		searchParams,
		setPaginationState,
	]);

	useEffect(() => {
		const prev = prevFiltersRef.current;
		if (prev.search === search && prev.sort === sort) {
			return;
		}
		prevFiltersRef.current = { search, sort };
		resetToFirstPage();
	}, [search, sort, resetToFirstPage]);

	useEffect(() => {
		if (isRestoringPagination) {
			return;
		}
		setSearchParams(
			(prev) => {
				const next = new URLSearchParams(prev);
				if (currentPage > 1) {
					next.set("page", String(currentPage));
				} else {
					next.delete("page");
				}
				if (itemsPerPage !== 9) {
					next.set("perPage", String(itemsPerPage));
				} else {
					next.delete("perPage");
				}
				return next;
			},
			{ replace: true },
		);
	}, [currentPage, itemsPerPage, isRestoringPagination, setSearchParams]);

	useEffect(() => {
		if (isRestoringPagination) {
			return;
		}
		writeStoredMarketPagination(
			buildMarketPaginationStorageKey(providerId, search, itemsPerPage),
			{
				page: currentPage,
				history: cursorHistory,
				hasNextPage,
			},
		);
	}, [cursorHistory, currentPage, hasNextPage, isRestoringPagination, itemsPerPage, providerId, search]);

	useEffect(() => {
		if (registryQuery.data) {
			setHasNextPage(Boolean(registryQuery.data.nextCursor));
		}
	}, [registryQuery.data, setHasNextPage]);

	useEffect(() => {
		const nextCursor = registryQuery.data?.nextCursor;
		if (!nextCursor || registryQuery.isFetching || isRestoringPagination) {
			return;
		}

		const nextPage = currentPage + 1;
		void queryClient.prefetchQuery({
			queryKey: marketListQueryKey(providerId, {
				search,
				page: nextPage,
				limit: itemsPerPage,
				cursor: nextCursor,
			}),
			queryFn: () =>
				provider.fetchPage({
					cursor: nextCursor,
					search: search || undefined,
					limit: itemsPerPage,
				}),
			staleTime: MARKET_LIST_STALE_MS,
			gcTime: MARKET_LIST_GC_MS,
		});
	}, [
		currentPage,
		isRestoringPagination,
		itemsPerPage,
		provider,
		providerId,
		queryClient,
		registryQuery.data?.nextCursor,
		registryQuery.isFetching,
		search,
	]);

	const blacklistIds = useMemo(() => {
		return new Set(marketBlacklist.map((entry) => entry.serverId));
	}, [marketBlacklist]);

	const filtered = useMemo(() => {
		if (!registryQuery.data) return [] as RegistryServerEntry[];
		if (blacklistIds.size === 0) return registryQuery.data.entries;
		return registryQuery.data.entries.filter(
			(server) => !blacklistIds.has(getCanonicalRegistryServerId(server)),
		);
	}, [registryQuery.data, blacklistIds]);

	const servers = useMemo(() => {
		const items = [...filtered];
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
	}, [filtered, sort]);

	const isInitialLoading =
		isRestoringPagination || (registryQuery.isLoading && !registryQuery.data);
	const isPageLoading =
		!isRestoringPagination && registryQuery.isFetching && Boolean(registryQuery.data);
	const isEmpty = !isInitialLoading && servers.length === 0;
	const fetchError =
		registryQuery.error instanceof Error ? registryQuery.error : undefined;

	const handleRefresh = useCallback(() => {
		void (async () => {
			await invalidateCatalogCaches(false);
			await registryQuery.refetch();
		})();
	}, [invalidateCatalogCaches, registryQuery]);

	const handleNextPage = useCallback(() => {
		if (!registryQuery.data?.nextCursor) return;
		goToNextPage(registryQuery.data.nextCursor);
		scrollToTop();
	}, [registryQuery.data?.nextCursor, goToNextPage]);

	const handlePreviousPage = useCallback(() => {
		goToPreviousPage();
		scrollToTop();
	}, [goToPreviousPage]);

	const handleFirstPage = useCallback(() => {
		resetToFirstPage();
		scrollToTop();
	}, [resetToFirstPage]);

	const handleItemsPerPageChange = useCallback(
		(nextItemsPerPage: number) => {
			if (nextItemsPerPage === itemsPerPage) {
				return;
			}
			setItemsPerPage(nextItemsPerPage as MarketPageSize);
			resetToFirstPage();
		},
		[itemsPerPage, resetToFirstPage],
	);

	const handleLastPage = useCallback(async () => {
		if (!registryQuery.data?.nextCursor) {
			return;
		}

		setIsPaginationActionLoading(true);
		try {
			const result = await walkCursorsToPage(
				registryQuery.data.nextCursor,
				currentPage,
				Infinity,
				(cursor) => provider.fetchPage({
					cursor,
					search: search || undefined,
					limit: itemsPerPage,
				}),
			);

			// Prepend existing history
			const fullHistory = [...cursorHistory, ...result.history];
			setPaginationState(result.finalPage, fullHistory, false);
			scrollToTop();
		} finally {
			setIsPaginationActionLoading(false);
		}
	}, [currentPage, cursorHistory, itemsPerPage, registryQuery.data?.nextCursor, search, provider, setPaginationState]);

	const handleGoToPage = useCallback(
		async (targetPage: number) => {
			const p = Math.max(1, Math.floor(targetPage));
			if (p === currentPage) {
				return;
			}
			if (p === 1) {
				handleFirstPage();
				return;
			}
			if (p < currentPage) {
				const hist = cursorHistory;
				if (p <= hist.length) {
					setPaginationState(p, hist.slice(0, p), true);
					scrollToTop();
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
				const result = await walkCursorsToPage(
					registryQuery.data.nextCursor,
					currentPage,
					p,
					(cursor) => provider.fetchPage({
						cursor,
						search: search || undefined,
						limit: itemsPerPage,
					}),
				);

				const fullHistory = [...cursorHistory, ...result.history];

				if (result.finalPage !== p) {
					setPaginationState(result.finalPage, fullHistory, result.hasNext);
					scrollToTop();
					return;
				}

				setPaginationState(p, fullHistory, result.hasNext);
				scrollToTop();
			} finally {
				setIsPaginationActionLoading(false);
			}
		},
		[
			currentPage,
			cursorHistory,
			handleFirstPage,
			itemsPerPage,
			registryQuery.data?.nextCursor,
			search,
			provider,
			setPaginationState,
		],
	);

	const totalPages =
		registryQuery.data && !registryQuery.data.nextCursor
			? currentPage
			: null;

	return {
		servers,
		isInitialLoading,
		isPageLoading,
		isEmpty,
		fetchError,
		pagination: {
			currentPage,
			hasPreviousPage,
			hasNextPage,
			itemsPerPage,
			totalPages,
		},
		onNextPage: handleNextPage,
		onPreviousPage: handlePreviousPage,
		onFirstPage: handleFirstPage,
		onLastPage: handleLastPage,
		onGoToPage: handleGoToPage,
		onItemsPerPageChange: handleItemsPerPageChange,
		isPaginationActionLoading: isPaginationActionLoading || isRestoringPagination,
		onRefresh: handleRefresh,
		lastSyncedAt:
			registryQuery.dataUpdatedAt > 0 ? registryQuery.dataUpdatedAt : undefined,
		onSync: async () => {
			if (provider.meta.supportsSync) {
				await syncMutation.mutateAsync();
			} else {
				handleRefresh();
			}
		},
		isSyncing: provider.meta.supportsSync ? syncMutation.isPending : registryQuery.isFetching,
	};
}
