export const MARKET_PAGE_SIZE_OPTIONS = [9, 27, 54, 72] as const;

export type MarketPageSize = (typeof MARKET_PAGE_SIZE_OPTIONS)[number];

const STORAGE_PREFIX = "mcpmate.market.pagination";

export interface StoredMarketPagination {
	page: number;
	history: (string | undefined)[];
	hasNextPage: boolean;
}

export function buildMarketPaginationStorageKey(
	providerId: string,
	search: string,
	itemsPerPage: number,
): string {
	return `${STORAGE_PREFIX}:${providerId}:${search}:${itemsPerPage}`;
}

export function readStoredMarketPagination(
	key: string,
): StoredMarketPagination | null {
	if (typeof window === "undefined") {
		return null;
	}
	try {
		const raw = sessionStorage.getItem(key);
		if (!raw) {
			return null;
		}
		const parsed = JSON.parse(raw) as StoredMarketPagination;
		if (
			typeof parsed.page !== "number" ||
			!Array.isArray(parsed.history) ||
			typeof parsed.hasNextPage !== "boolean"
		) {
			return null;
		}
		return parsed;
	} catch {
		return null;
	}
}

export function writeStoredMarketPagination(
	key: string,
	state: StoredMarketPagination,
): void {
	if (typeof window === "undefined") {
		return;
	}
	try {
		sessionStorage.setItem(key, JSON.stringify(state));
	} catch {
		/* noop */
	}
}

export function parseMarketListPageParam(value: string | null): number {
	const parsed = Number.parseInt(value ?? "1", 10);
	if (!Number.isFinite(parsed) || parsed < 1) {
		return 1;
	}
	return parsed;
}

export function parseMarketListPerPageParam(value: string | null): MarketPageSize {
	const parsed = Number.parseInt(value ?? "9", 10);
	if (MARKET_PAGE_SIZE_OPTIONS.includes(parsed as MarketPageSize)) {
		return parsed as MarketPageSize;
	}
	return 9;
}

export const MARKET_LIST_RETURN_SEARCH_KEY = "mcpmate.market.listReturnSearch";

export interface MarketDetailLocationState {
	marketListSearch?: string;
}

export function isMarketDetailPath(pathname: string): boolean {
	return pathname.startsWith("/market/") && pathname !== "/market";
}

export function normalizeMarketListSearch(search: string): string {
	if (!search || search === "?") {
		return "";
	}
	return search.startsWith("?") ? search : `?${search}`;
}

export function buildMarketListPath(search = ""): string {
	const normalized = normalizeMarketListSearch(search);
	return normalized ? `/market${normalized}` : "/market";
}

export function rememberMarketListReturnSearch(search: string): void {
	if (typeof window === "undefined") {
		return;
	}
	try {
		sessionStorage.setItem(
			MARKET_LIST_RETURN_SEARCH_KEY,
			normalizeMarketListSearch(search),
		);
	} catch {
		/* noop */
	}
}

export function readRememberedMarketListReturnSearch(): string {
	if (typeof window === "undefined") {
		return "";
	}
	try {
		return normalizeMarketListSearch(
			sessionStorage.getItem(MARKET_LIST_RETURN_SEARCH_KEY) ?? "",
		);
	} catch {
		return "";
	}
}

export function resolveMarketListReturnPath(
	pathname: string,
	state: unknown,
): string {
	const marketState = state as MarketDetailLocationState | null;
	if (marketState?.marketListSearch != null) {
		return buildMarketListPath(marketState.marketListSearch);
	}
	if (isMarketDetailPath(pathname)) {
		const remembered = readRememberedMarketListReturnSearch();
		if (remembered) {
			return buildMarketListPath(remembered);
		}
	}
	return "/market";
}

export function resolveMarketListHref(
	pathname: string,
	search: string,
	state: unknown,
): string {
	if (pathname === "/market") {
		return buildMarketListPath(search);
	}
	if (isMarketDetailPath(pathname)) {
		return resolveMarketListReturnPath(pathname, state);
	}
	return "/market";
}
