import type { RegistryServerEntry } from "../../lib/types";

export interface MarketCardProps {
	server: RegistryServerEntry;
	onPreview: (server: RegistryServerEntry) => void;
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
	isInitialLoading: boolean;
	isPageLoading: boolean;
	isEmpty: boolean;
	pagination: {
		currentPage: number;
		hasPreviousPage: boolean;
		hasNextPage: boolean;
		itemsPerPage: number;
	};
	onServerPreview: (server: RegistryServerEntry) => void;
	onServerHide: (server: RegistryServerEntry) => void;
	enableBlacklist: boolean;
	onNextPage: () => void;
	onPreviousPage: () => void;
}

export interface MarketSearchProps {
	search: string;
	onSearchChange: (value: string) => void;
	sort: SortOption;
	onSortChange: (value: SortOption) => void;
	onRefresh: () => void;
	isLoading: boolean;
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
	};
	onNextPage: () => void;
	onPreviousPage: () => void;
	onRefresh: () => void;
}
