export const DISCOVERY_CACHE_TTL_MS = 60 * 60 * 1000;
export const DISCOVERY_CACHE_KEY_PREFIX = "mcpmate.discovery.cache";
export const DISCOVERY_SESSION_KEY_PREFIX = "mcpmate.discovery.session";
const SESSION_STORAGE_PREFIX = "mcpmate.session.";

export function discoveryCacheKey({ mode, origin, kind, requestUrl }) {
	return `${DISCOVERY_CACHE_KEY_PREFIX}.${mode}.${origin}.${kind}.${encodeURIComponent(requestUrl)}`;
}

export function discoveryCachePrefix({ mode, origin, kind }) {
	return `${DISCOVERY_CACHE_KEY_PREFIX}.${mode}.${origin}.${kind}.`;
}

export function discoverySessionKey({ mode, origin, locale, kind }) {
	return `${DISCOVERY_SESSION_KEY_PREFIX}.${mode}.${origin}.${locale}.${kind}`;
}

export function isCacheEntryFresh(entry, ttlMs = DISCOVERY_CACHE_TTL_MS) {
	return Boolean(entry?.cachedAt && Date.now() - entry.cachedAt <= ttlMs);
}

function localStorageArea() {
	return typeof chrome !== "undefined" && chrome.storage?.local ? chrome.storage.local : null;
}

function sessionStorageArea() {
	return typeof chrome !== "undefined" && chrome.storage?.session ? chrome.storage.session : null;
}

async function readKeyedValue(area, key, { fallbackLocalStorage = false } = {}) {
	if (area) {
		return (await area.get(key))[key] ?? null;
	}
	if (!fallbackLocalStorage) {
		try {
			const raw = sessionStorage.getItem(`${SESSION_STORAGE_PREFIX}${key}`);
			return raw ? JSON.parse(raw) : null;
		} catch {
			return null;
		}
	}
	try {
		return JSON.parse(localStorage.getItem(key) || "null");
	} catch {
		return null;
	}
}

async function writeKeyedValue(area, key, value, { fallbackLocalStorage = false } = {}) {
	if (area) {
		await area.set({ [key]: value });
		return;
	}
	if (!fallbackLocalStorage) {
		sessionStorage.setItem(`${SESSION_STORAGE_PREFIX}${key}`, JSON.stringify(value));
		return;
	}
	localStorage.setItem(key, JSON.stringify(value));
}

async function removeKeyedValues(area, keys, { fallbackLocalStorage = false } = {}) {
	if (keys.length === 0) return;
	if (area) {
		await area.remove(keys);
		return;
	}
	if (!fallbackLocalStorage) {
		for (const key of keys) {
			sessionStorage.removeItem(`${SESSION_STORAGE_PREFIX}${key}`);
		}
		return;
	}
	for (const key of keys) {
		localStorage.removeItem(key);
	}
}

export async function readDiscoveryCacheEntry(context, kind, requestUrl) {
	const key = discoveryCacheKey({ ...context, kind, requestUrl });
	return readKeyedValue(localStorageArea(), key, { fallbackLocalStorage: true });
}

export async function readDiscoveryCacheData(context, kind, requestUrl, catalogGeneratedAt) {
	const cached = await readDiscoveryCacheEntry(context, kind, requestUrl);
	if (!isCacheEntryFresh(cached)) {
		return null;
	}
	if (
		catalogGeneratedAt &&
		(!cached.catalogGeneratedAt || cached.catalogGeneratedAt !== catalogGeneratedAt)
	) {
		return null;
	}
	return cached.data || null;
}

export async function writeDiscoveryCache(context, kind, requestUrl, data) {
	const key = discoveryCacheKey({ ...context, kind, requestUrl });
	const cached = {
		cachedAt: Date.now(),
		catalogGeneratedAt: typeof data?.generatedAt === "string" ? data.generatedAt : null,
		data,
	};
	await writeKeyedValue(localStorageArea(), key, cached, { fallbackLocalStorage: true });
}

export async function clearDiscoveryCacheForKind(context, kind) {
	const prefix = discoveryCachePrefix({ ...context, kind });
	const area = localStorageArea();
	if (area) {
		const all = await area.get(null);
		const keysToRemove = Object.keys(all).filter((key) => key.startsWith(prefix));
		await removeKeyedValues(area, keysToRemove);
		return;
	}
	const keysToRemove = [];
	for (let index = localStorage.length - 1; index >= 0; index -= 1) {
		const key = localStorage.key(index);
		if (key?.startsWith(prefix)) {
			keysToRemove.push(key);
		}
	}
	await removeKeyedValues(null, keysToRemove, { fallbackLocalStorage: true });
}

export async function pruneExpiredDiscoveryCaches(context) {
	const prefix = `${DISCOVERY_CACHE_KEY_PREFIX}.${context.mode}.${context.origin}.`;
	const area = localStorageArea();
	if (area) {
		const all = await area.get(null);
		const keysToRemove = [];
		for (const [key, value] of Object.entries(all)) {
			if (!key.startsWith(prefix)) continue;
			if (!isCacheEntryFresh(value)) {
				keysToRemove.push(key);
			}
		}
		await removeKeyedValues(area, keysToRemove);
		return;
	}
	const keysToRemove = [];
	for (let index = localStorage.length - 1; index >= 0; index -= 1) {
		const key = localStorage.key(index);
		if (!key?.startsWith(prefix)) continue;
		try {
			const value = JSON.parse(localStorage.getItem(key) || "null");
			if (!isCacheEntryFresh(value)) {
				keysToRemove.push(key);
			}
		} catch {
			keysToRemove.push(key);
		}
	}
	await removeKeyedValues(null, keysToRemove, { fallbackLocalStorage: true });
}

export async function readSessionSnapshot(context, kind, locale) {
	const key = discoverySessionKey({ ...context, locale, kind });
	const snapshot = await readKeyedValue(sessionStorageArea() || localStorageArea(), key);
	if (!snapshot || snapshot.locale !== locale) {
		return null;
	}
	if (!isCacheEntryFresh(snapshot)) {
		return null;
	}
	return snapshot;
}

export async function writeSessionSnapshot(context, kind, locale, { state, scrollTop }) {
	const key = discoverySessionKey({ ...context, locale, kind });
	const snapshot = {
		cachedAt: Date.now(),
		locale,
		scrollTop: Number.isFinite(scrollTop) ? scrollTop : 0,
		state: {
			entries: state.entries,
			hasMore: state.hasMore,
			nextOffset: state.nextOffset,
			catalogGeneratedAt: state.catalogGeneratedAt,
			loaded: true,
		},
	};
	const area = sessionStorageArea() || localStorageArea();
	await writeKeyedValue(area, key, snapshot);
}

export async function clearSessionSnapshots(context, locale) {
	const prefix = `${DISCOVERY_SESSION_KEY_PREFIX}.${context.mode}.${context.origin}.${locale}.`;
	const area = sessionStorageArea() || localStorageArea();
	if (area) {
		const all = await area.get(null);
		const keysToRemove = Object.keys(all).filter((key) => key.startsWith(prefix));
		await removeKeyedValues(area, keysToRemove);
		return;
	}
	const keysToRemove = [];
	for (let index = sessionStorage.length - 1; index >= 0; index -= 1) {
		const storageKey = sessionStorage.key(index);
		if (storageKey?.startsWith(`${SESSION_STORAGE_PREFIX}${prefix}`)) {
			keysToRemove.push(storageKey.slice(SESSION_STORAGE_PREFIX.length));
		}
	}
	await removeKeyedValues(null, keysToRemove);
}
