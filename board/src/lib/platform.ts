declare global {
	interface Window {
		__TAURI__?: unknown;
		__TAURI_IPC__?: unknown;
		__TAURI_INTERNALS__?: unknown;
		__TAURI_METADATA__?: unknown;
	}
}

const TAURI_INDICATOR_KEYS = [
	"__TAURI__",
	"__TAURI_IPC__",
	"__TAURI_INTERNALS__",
	"__TAURI_METADATA__",
	"__MCPMATE_IS_TAURI__",
];

function hasGlobalIndicators(): boolean {
	if (typeof window === "undefined") {
		return false;
	}
	const w = window as Record<string, unknown>;
	return TAURI_INDICATOR_KEYS.some((key) => w[key] !== undefined);
}

function hasSchemeIndicators(): boolean {
	if (typeof window === "undefined") {
		return false;
	}
	const protocol = window.location?.protocol ?? "";
	return protocol.startsWith("tauri") || protocol === "app:";
}

function hasUserAgentIndicators(): boolean {
	if (typeof navigator === "undefined") {
		return false;
	}
	const ua = navigator.userAgent || "";
	return ua.includes("Tauri") || ua.includes("MCPMate");
}

export function isTauriEnvironmentSync(): boolean {
	return hasGlobalIndicators() || hasSchemeIndicators() || hasUserAgentIndicators();
}

export async function detectTauriEnvironment(): Promise<boolean> {
	return isTauriEnvironmentSync();
}

export {};
