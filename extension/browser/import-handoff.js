/** Classic-script build of import-handoff.mjs for content script use. Keep in sync. */
(function (global) {
	const HANDOFF_TTL_MS = 10 * 60 * 1000;
	const HANDOFF_STORAGE_KEY_PREFIX = "mcpmate.importHandoff";
	const MCPMATE_IMPORT_SCHEME_URL = "mcpmate://import/server";

	function handoffStorageKey(id) {
		return `${HANDOFF_STORAGE_KEY_PREFIX}.${id}`;
	}

	function createHandoffRecord(payload, now = Date.now()) {
		return {
			createdAt: now,
			payload,
		};
	}

	function isFreshHandoffRecord(record, now = Date.now(), ttlMs = HANDOFF_TTL_MS) {
		return Boolean(
			record &&
				typeof record.createdAt === "number" &&
				now - record.createdAt <= ttlMs &&
				typeof record.payload?.text === "string" &&
				record.payload.text.trim(),
		);
	}

	function encodeImportPayload(payload) {
		const json = JSON.stringify(payload);
		const bytes = new TextEncoder().encode(json);
		let binary = "";
		for (let index = 0; index < bytes.length; index += 1) {
			binary += String.fromCharCode(bytes[index]);
		}
		return btoa(binary)
			.replace(/\+/g, "-")
			.replace(/\//g, "_")
			.replace(/=+$/, "");
	}

	function buildMcpMateImportUrl(payload) {
		return `${MCPMATE_IMPORT_SCHEME_URL}?p=${encodeURIComponent(
			encodeImportPayload(payload),
		)}`;
	}

	function buildHandoffPageUrl(id, runtime = global.chrome?.runtime) {
		const path = `handoff.html?id=${encodeURIComponent(id)}`;
		if (typeof runtime?.getURL === "function") {
			return runtime.getURL(path);
		}
		return path;
	}

	function createHandoffId(cryptoLike = global.crypto) {
		if (typeof cryptoLike?.randomUUID === "function") {
			return cryptoLike.randomUUID();
		}
		const random = Math.random().toString(36).slice(2);
		return `${Date.now().toString(36)}-${random}`;
	}

	function storageArea(chromeLike = global.chrome) {
		return chromeLike?.storage?.local || null;
	}

	async function writeHandoffRecord(id, record, chromeLike = global.chrome) {
		const key = handoffStorageKey(id);
		const area = storageArea(chromeLike);
		if (area) {
			await area.set({ [key]: record });
			return;
		}
		localStorage.setItem(key, JSON.stringify(record));
	}

	async function readHandoffRecord(id, chromeLike = global.chrome) {
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

	async function consumeHandoffRecord(id, chromeLike = global.chrome, now = Date.now()) {
		const record = await readHandoffRecord(id, chromeLike);
		if (!isFreshHandoffRecord(record, now)) {
			await removeHandoffRecord(id, chromeLike);
			return null;
		}
		await removeHandoffRecord(id, chromeLike);
		return record;
	}

	async function removeHandoffRecord(id, chromeLike = global.chrome) {
		const key = handoffStorageKey(id);
		const area = storageArea(chromeLike);
		if (area) {
			await area.remove(key);
			return;
		}
		localStorage.removeItem(key);
	}

	global.__MCPMATE_IMPORT_HANDOFF__ = {
		HANDOFF_TTL_MS,
		buildHandoffPageUrl,
		buildMcpMateImportUrl,
		consumeHandoffRecord,
		createHandoffId,
		createHandoffRecord,
		handoffStorageKey,
		isFreshHandoffRecord,
		readHandoffRecord,
		removeHandoffRecord,
		writeHandoffRecord,
	};
})(globalThis);
