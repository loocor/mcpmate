import type { RegistryServerEntry } from "../../lib/types";

export interface MarketCardProps {
	server: RegistryServerEntry;
	isInstalled: boolean;
	onPreview: (server: RegistryServerEntry) => void;
	onInstall: (server: RegistryServerEntry) => void;
	onHide: (server: RegistryServerEntry) => void;
	enableBlacklist: boolean;
}

export type SortOption = "recent" | "name";

export interface RemoteOption {
	id: string;
	label: string;
	kind: string;
	source: "remote" | "package";
	url: string | null;
	headers: Array<{
		name: string;
		isRequired?: boolean;
		description?: string;
	}> | null;
	envVars: Array<{
		name: string;
		isRequired?: boolean;
		description?: string;
	}> | null;
	packageIdentifier: string | null;
	packageMeta: unknown;
}

export interface ServerGridProps {
	servers: RegistryServerEntry[];
	installedRegistryServerKeys: Set<string>;
	isInitialLoading: boolean;
	isPageLoading: boolean;
	isEmpty: boolean;
	pagination: {
		currentPage: number;
		hasPreviousPage: boolean;
		hasNextPage: boolean;
		itemsPerPage: number;
		totalPages: number | null;
	};
	onServerPreview: (server: RegistryServerEntry) => void;
	onServerInstall: (server: RegistryServerEntry) => void;
	onServerHide: (server: RegistryServerEntry) => void;
	enableBlacklist: boolean;
	onNextPage: () => void;
	onPreviousPage: () => void;
	onFirstPage: () => void;
	onLastPage: () => Promise<void>;
	onGoToPage: (page: number) => Promise<void>;
	onItemsPerPageChange: (itemsPerPage: number) => void;
	isPaginationActionLoading: boolean;
}

export interface MarketSearchProps {
	search: string;
	onSearchChange: (value: string) => void;
	sort: SortOption;
	onSortChange: (value: SortOption) => void;
	isLoading: boolean;
	lastSyncedAt?: string;
	onSync: () => Promise<void>;
	isSyncing: boolean;
}

export interface UseMarketDataReturn {
	servers: RegistryServerEntry[];
	sortedServers: RegistryServerEntry[];
	isInitialLoading: boolean;
	isPageLoading: boolean;
	isEmpty: boolean;
	fetchError: Error | undefined;
	pagination: {
		currentPage: number;
		hasPreviousPage: boolean;
		hasNextPage: boolean;
		itemsPerPage: number;
		totalPages: number | null;
	};
	onNextPage: () => void;
	onPreviousPage: () => void;
	onFirstPage: () => void;
	onLastPage: () => Promise<void>;
	onGoToPage: (page: number) => Promise<void>;
	onItemsPerPageChange: (itemsPerPage: number) => void;
	isPaginationActionLoading: boolean;
	onRefresh: () => void;
	lastSyncedAt?: string;
	onSync: () => Promise<void>;
	isSyncing: boolean;
}
