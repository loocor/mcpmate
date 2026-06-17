/**
 * Minimal Chrome extension API shim for popup dev preview in a normal browser tab.
 */
(function installChromeShim(global) {
	if (global.chrome?.runtime?.getURL) {
		return;
	}

	function createStorageArea() {
		const prefix = "mcpmate.chrome-shim.";
		return {
			async get(keys) {
				const result = {};
				const wanted =
					keys === null || keys === undefined
						? null
						: Array.isArray(keys)
							? keys
							: typeof keys === "string"
								? [keys]
								: Object.keys(keys);
				for (let index = 0; index < localStorage.length; index += 1) {
					const storageKey = localStorage.key(index);
					if (!storageKey?.startsWith(prefix)) continue;
					const logicalKey = storageKey.slice(prefix.length);
					if (wanted && !wanted.includes(logicalKey)) continue;
					try {
						result[logicalKey] = JSON.parse(localStorage.getItem(storageKey) || "null");
					} catch {
						result[logicalKey] = null;
					}
				}
				if (typeof keys === "object" && keys !== null && !Array.isArray(keys)) {
					for (const [key, defaultValue] of Object.entries(keys)) {
						if (!Object.prototype.hasOwnProperty.call(result, key)) {
							result[key] = defaultValue;
						}
					}
				}
				return result;
			},
			async set(items) {
				for (const [key, value] of Object.entries(items)) {
					localStorage.setItem(`${prefix}${key}`, JSON.stringify(value));
				}
			},
			async remove(keys) {
				const list = Array.isArray(keys) ? keys : [keys];
				for (const key of list) {
					localStorage.removeItem(`${prefix}${key}`);
				}
			},
		};
	}

	const storage = createStorageArea();
	global.chrome = {
		runtime: {
			getURL(relativePath) {
				const base = global.location.pathname.includes("/dev/")
					? new URL("../", global.location.href)
					: new URL("./", global.location.href);
				return new URL(relativePath, base).href;
			},
		},
		tabs: {
			create({ url }) {
				global.open(url, "_blank", "noopener,noreferrer");
			},
		},
		action: {
			setIcon() { },
		},
		storage: {
			local: storage,
			sync: storage,
		},
	};
})(globalThis);
