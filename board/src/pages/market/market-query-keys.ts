/** Shared React Query keys for Market catalog surfaces. */

export const MARKET_LIST_STALE_MS = 5 * 60 * 1000;
export const MARKET_LIST_GC_MS = 30 * 60 * 1000;
export const MARKET_DETAIL_STALE_MS = 5 * 60 * 1000;
export const MARKET_README_STALE_MS = 15 * 60 * 1000;

export interface MarketListQueryParams {
	search: string;
	page: number;
	limit: number;
	cursor: string | undefined;
}

export function marketListRootKey(providerId: string) {
	return ["market", providerId, "list"] as const;
}

export function marketListQueryKey(
	providerId: string,
	params: MarketListQueryParams,
) {
	return [...marketListRootKey(providerId), params] as const;
}

export function marketDetailRootKey(providerId: string) {
	return ["market", providerId, "detail"] as const;
}

export function marketDetailQueryKey(providerId: string, registryKey: string) {
	return [...marketDetailRootKey(providerId), registryKey] as const;
}

export function marketReadmeQueryKey(
	providerId: string,
	repositoryUrl: string,
	repositorySubfolder: string,
) {
	return [
		"market",
		providerId,
		"readme",
		"v2",
		repositoryUrl,
		repositorySubfolder,
	] as const;
}
