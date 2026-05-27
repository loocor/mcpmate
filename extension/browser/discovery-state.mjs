export const DISCOVERY_SURFACE = "extension";
export const DISCOVERY_PAGE_SIZE = 6;
export const PAGEABLE_DISCOVERY_KINDS = new Set(["servers", "clients"]);

export function isPageableDiscoveryKind(kind) {
	return PAGEABLE_DISCOVERY_KINDS.has(kind);
}

export function discoveryQueryForPage({ kind, limit = DISCOVERY_PAGE_SIZE, offset = 0 }) {
	if (!isPageableDiscoveryKind(kind)) {
		return { surface: DISCOVERY_SURFACE };
	}
	return {
		limit,
		offset,
		surface: DISCOVERY_SURFACE,
	};
}

export function buildDiscoveryUrl(endpoint, query) {
	const url = new URL(endpoint);
	for (const [key, value] of Object.entries(query)) {
		if (value !== undefined && value !== null) {
			url.searchParams.set(key, String(value));
		}
	}
	return url.toString();
}

export function responseMetadata(data) {
	return data?.metadata || data?.meta || {};
}

export function discoveryPageState({ kind, entries, metadata, limit, offset }) {
	const hasMore =
		isPageableDiscoveryKind(kind) &&
		entries.length > 0 &&
		metadata?.mode === "page" &&
		metadata?.hasMore === true;
	let nextOffset = null;
	if (hasMore) {
		nextOffset = Number.isFinite(metadata?.nextOffset)
			? metadata.nextOffset
			: offset + limit;
	}
	return {
		entries,
		hasMore,
		nextOffset,
	};
}

export function nextDiscoveryPageState(current, page, { reset }) {
	return {
		entries: reset ? page.entries : [...current.entries, ...page.entries],
		hasMore: page.hasMore,
		nextOffset: page.nextOffset,
	};
}

export function entriesForPageRender(nextState, page, { reset }) {
	return reset ? nextState.entries : page.entries;
}

export function shouldClearEntriesBeforeLoad(current, { reset }) {
	return reset && current.entries.length === 0;
}

export function shouldStartPullRefresh({
	button,
	pointerType,
	scrollTop,
	panelName,
	selectionType,
}) {
	return (
		button === 0 &&
		pointerType === "touch" &&
		scrollTop === 0 &&
		panelName !== "settings" &&
		selectionType !== "Range"
	);
}
