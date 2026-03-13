import { isTauriEnvironmentSync } from "./platform";

type ClipboardModule = {
	readText: () => Promise<string>;
	writeText: (text: string) => Promise<void>;
	clear?: () => Promise<void>;
};

let cachedModulePromise: Promise<ClipboardModule | null> | null = null;

async function loadTauriClipboardModule(): Promise<ClipboardModule | null> {
	if (!isTauriEnvironmentSync()) {
		return null;
	}
	if (cachedModulePromise) {
		return cachedModulePromise;
	}
	cachedModulePromise = import("@tauri-apps/plugin-clipboard-manager")
		.then((module) => {
			const { readText, writeText, clear } = module;
			if (typeof readText !== "function" || typeof writeText !== "function") {
				return null;
			}
			return { readText, writeText, clear };
		})
		.catch((error) => {
			console.warn(
				"[clipboard] Failed to load Tauri clipboard module, falling back to navigator API.",
				error,
			);
			return null;
		});
	return cachedModulePromise;
}

async function readFromNavigatorClipboard(): Promise<string | null> {
	if (typeof navigator === "undefined") {
		return null;
	}
	try {
		const text = await navigator.clipboard?.readText?.();
		return text ?? null;
	} catch (error) {
		console.warn("[clipboard] navigator.clipboard.readText failed", error);
		return null;
	}
}

async function readFromTauriInvoke(): Promise<string | null> {
  try {
    const w = globalThis as unknown as { __TAURI__?: any };
    const core = w.__TAURI__?.core;
    if (!core?.invoke) return null;
    const text = await core.invoke<string>("plugin:clipboard-manager|readText");
    return text ?? null;
  } catch (error) {
    console.warn("[clipboard] Tauri core.invoke(readText) failed", error);
    return null;
  }
}

async function writeToNavigatorClipboard(text: string): Promise<void> {
	if (typeof navigator === "undefined") {
		throw new Error("navigator is not available");
	}
	if (!navigator.clipboard?.writeText) {
		throw new Error("navigator.clipboard.writeText is not supported");
	}
	await navigator.clipboard.writeText(text);
}

export async function readClipboardText(): Promise<string | null> {
    const module = await loadTauriClipboardModule();
    if (module?.readText) {
        try {
            const text = await module.readText();
            if (text != null && text !== "") {
                return text;
            }
        } catch (error) {
            console.warn("[clipboard] Tauri readText failed, trying navigator API", error);
        }
    }
    // Fallback to low-level invoke when the ESM wrapper is unavailable in production build.
    if (isTauriEnvironmentSync()) {
        const text = await readFromTauriInvoke();
        if (text != null && text !== "") return text;
    }
    return readFromNavigatorClipboard();
}

export async function writeClipboardText(text: string): Promise<void> {
	const module = await loadTauriClipboardModule();
	if (module?.writeText) {
		try {
			await module.writeText(text);
			return;
		} catch (error) {
			console.warn("[clipboard] Tauri writeText failed, trying navigator API", error);
		}
	}
	await writeToNavigatorClipboard(text);
}

export async function clearClipboard(): Promise<void> {
	const module = await loadTauriClipboardModule();
	if (module?.clear) {
		try {
			await module.clear();
			return;
		} catch (error) {
			console.warn("[clipboard] Tauri clear failed, trying navigator API", error);
		}
	}
	if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
		await navigator.clipboard.writeText("");
	}
}
