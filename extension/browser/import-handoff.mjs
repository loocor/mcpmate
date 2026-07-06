export const HANDOFF_TTL_MS = 10 * 60 * 1000;
export const HANDOFF_STORAGE_KEY_PREFIX = "mcpmate.importHandoff";
export const MCPMATE_IMPORT_SCHEME_URL = "mcpmate://import/server";

export function handoffStorageKey(id) {
	return `${HANDOFF_STORAGE_KEY_PREFIX}.${id}`;
}

export function createHandoffRecord(payload, now = Date.now()) {
	return {
		createdAt: now,
		payload,
	};
}

export function isFreshHandoffRecord(
	record,
	now = Date.now(),
	ttlMs = HANDOFF_TTL_MS,
) {
	return Boolean(
		record &&
			typeof record.createdAt === "number" &&
			now - record.createdAt <= ttlMs &&
			typeof record.payload?.text === "string" &&
			record.payload.text.trim(),
	);
}

export function encodeImportPayload(payload) {
	const json = JSON.stringify(payload);
	const bytes = new TextEncoder().encode(json);
	let binary = "";
	for (let index = 0; index < bytes.length; index += 1) {
		binary += String.fromCharCode(bytes[index]);
	}
	return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

export function buildMcpMateImportUrl(payload) {
	return `${MCPMATE_IMPORT_SCHEME_URL}?p=${encodeURIComponent(
		encodeImportPayload(payload),
	)}`;
}

export function buildHandoffPageUrl(id, runtime = globalThis.chrome?.runtime) {
	const path = `handoff.html?id=${encodeURIComponent(id)}`;
	if (typeof runtime?.getURL === "function") {
		return runtime.getURL(path);
	}
	return path;
}

export function createHandoffId(cryptoLike = globalThis.crypto) {
	if (typeof cryptoLike?.randomUUID === "function") {
		return cryptoLike.randomUUID();
	}
	const random = Math.random().toString(36).slice(2);
	return `${Date.now().toString(36)}-${random}`;
}

function storageArea(chromeLike = globalThis.chrome) {
	return chromeLike?.storage?.local || null;
}

export async function writeHandoffRecord(
	id,
	record,
	chromeLike = globalThis.chrome,
) {
	const key = handoffStorageKey(id);
	const area = storageArea(chromeLike);
	if (area) {
		await area.set({ [key]: record });
		return;
	}
	localStorage.setItem(key, JSON.stringify(record));
}

export async function readHandoffRecord(id, chromeLike = globalThis.chrome) {
	const key = handoffStorageKey(id);
	const area = storageArea(chromeLike);
	if (area) {
		return (await area.get(key))[key] ?? null;
	}
	try {
		return JSON.parse(localStorage.getItem(key) || "null");
	} catch {
		return null;
	}
}

export async function consumeHandoffRecord(
	id,
	chromeLike = globalThis.chrome,
	now = Date.now(),
) {
	const record = await readHandoffRecord(id, chromeLike);
	if (!isFreshHandoffRecord(record, now)) {
		await removeHandoffRecord(id, chromeLike);
		return null;
	}
	await removeHandoffRecord(id, chromeLike);
	return record;
}

export async function removeHandoffRecord(id, chromeLike = globalThis.chrome) {
	const key = handoffStorageKey(id);
	const area = storageArea(chromeLike);
	if (area) {
		await area.remove(key);
		return;
	}
	localStorage.removeItem(key);
}
