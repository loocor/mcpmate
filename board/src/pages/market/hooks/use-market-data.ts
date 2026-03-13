import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useRef } from "react";
import { useCursorPagination } from "../../../hooks/use-cursor-pagination";
import { fetchRegistryServers, getOfficialMeta } from "../../../lib/registry";
import { useAppStore } from "../../../lib/store";
import type { RegistryServerEntry } from "../../../lib/types";
import type { UseMarketDataReturn } from "../types";
import { getRegistryIdentity } from "../utils";

export function useMarketData(
	search: string,
	sort: "recent" | "name",
): UseMarketDataReturn {
	const queryClient = useQueryClient();
	const marketBlacklist = useAppStore(
		(state) => state.dashboardSettings.marketBlacklist,
	);

	const handlePaginationReset = useCallback(() => {
		// Clear query cache when resetting pagination
		queryClient.removeQueries({ queryKey: ["market", "registry"] });
	}, [queryClient]);

	const pagination = useCursorPagination({
		limit: 9,
		onReset: handlePaginationReset,
	});

	const registryQuery = useQuery({
		queryKey: ["market", "registry", search, pagination.currentPage],
		queryFn: async () => {
			const result = await fetchRegistryServers({
				cursor: pagination.currentCursor,
				search: search || undefined,
				limit: pagination.itemsPerPage,
			});

			return result;
		},
		staleTime: 1000 * 60 * 5,
	});

	// Update pagination state when query data changes
	useEffect(() => {
		if (registryQuery.data?.metadata) {
			pagination.setHasNextPage(
				Boolean(registryQuery.data.metadata.nextCursor),
			);
		}
	}, [registryQuery.data?.metadata, pagination.setHasNextPage]);

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
		registryQuery.refetch();
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
		},
		onNextPage: handleNextPage,
		onPreviousPage: handlePreviousPage,
		onRefresh: handleRefresh,
	};
}
