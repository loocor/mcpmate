import type { MarketPortalMeta } from "../../lib/store";
import type { RegistryServerEntry } from "../../lib/types";

export interface MarketCardProps {
	server: RegistryServerEntry;
	onPreview: (server: RegistryServerEntry) => void;
	onHide: (server: RegistryServerEntry) => void;
	enableBlacklist: boolean;
}

export type SortOption = "recent" | "name";

// Tab management types
export interface TabItem {
	id: string;
	label: string;
	type: "official" | "third-party";
	url?: string;
	icon?: string;
	closable: boolean;
	portalId?: string;
}

// Market mode types and interfaces
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

// Server Grid Component props
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

// Market search props
export interface MarketSearchProps {
	search: string;
	onSearchChange: (value: string) => void;
	sort: SortOption;
	onSortChange: (value: SortOption) => void;
	onRefresh: () => void;
	isLoading: boolean;
}

// Market tabs props
export interface MarketTabsProps {
	tabs: TabItem[];
	activeTab: string;
	onTabChange: (tabId: string) => void;
	onCloseTab: (tabId: string) => void;
	onAddOfficial: () => void;
	availablePortals: MarketPortalMeta[];
	onAddPortal: (portalId: string) => void;
}

// Market iframe props
export interface MarketIframeProps {
	url: string;
	title: string;
	portalId: string;
	proxyPath: string;
	className?: string;
	refreshKey?: number;
}

// Market data hook return type
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

// Market tabs hook return type
export interface UseMarketTabsReturn {
	tabs: TabItem[];
	activeTab: string;
	setActiveTab: (tabId: string) => void;
	availablePortals: MarketPortalMeta[];
	addPortalTab: (portalId: string) => void;
	addOfficialTab: () => void;
	closeTab: (tabId: string) => void;
	portalMap: Record<string, MarketPortalMeta>;
}

// Market iframe hook return type
export interface UseMarketIframeReturn {
	iframeRef: React.RefObject<HTMLIFrameElement>;
	handleIframeLoad: () => void;
}
