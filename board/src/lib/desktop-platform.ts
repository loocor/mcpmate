import { isTauriEnvironmentSync } from "./platform";
import type { AdminDiscoveryPlatform } from "./admin-discovery";

type TauriInvoke = (command: string) => Promise<unknown>;

interface NavigatorLike {
	platform?: string;
	userAgent?: string;
	userAgentData?: {
		platform?: string;
	};
}

interface ReadTauriPlatformOptions {
	isTauri?: () => boolean;
	invoke?: TauriInvoke;
}

interface ReadAdminDiscoveryPlatformOptions extends ReadTauriPlatformOptions {
	navigatorLike?: NavigatorLike;
}

export function normalizeDesktopPlatform(value: unknown): AdminDiscoveryPlatform | undefined {
	if (value === "macos" || value === "windows" || value === "linux") {
		return value;
	}
	return undefined;
}

function normalizeBrowserPlatform(value: unknown): AdminDiscoveryPlatform | undefined {
	if (typeof value !== "string") return undefined;
	const lower = value.toLowerCase();
	if (lower.includes("mac")) return "macos";
	if (lower.includes("win")) return "windows";
	if (lower.includes("linux") || lower.includes("x11")) return "linux";
	return undefined;
}

async function defaultInvoke(command: string): Promise<unknown> {
	const { invoke } = await import("@tauri-apps/api/core");
	return invoke(command);
}

export async function readTauriAdminDiscoveryPlatform(
	options: ReadTauriPlatformOptions = {},
): Promise<AdminDiscoveryPlatform | undefined> {
	const isTauri = options.isTauri ?? isTauriEnvironmentSync;
	if (!isTauri()) return undefined;
	const invoke = options.invoke ?? defaultInvoke;
	return normalizeDesktopPlatform(await invoke("mcp_shell_read_platform"));
}

export function readBrowserAdminDiscoveryPlatform(
	navigatorLike: NavigatorLike | undefined =
		typeof navigator === "undefined" ? undefined : navigator,
): AdminDiscoveryPlatform | undefined {
	return (
		normalizeBrowserPlatform(navigatorLike?.userAgentData?.platform) ??
		normalizeBrowserPlatform(navigatorLike?.platform) ??
		normalizeBrowserPlatform(navigatorLike?.userAgent)
	);
}

export async function readAdminDiscoveryPlatform(
	options: ReadAdminDiscoveryPlatformOptions = {},
): Promise<AdminDiscoveryPlatform | undefined> {
	const tauriPlatform = await readTauriAdminDiscoveryPlatform(options);
	return tauriPlatform ?? readBrowserAdminDiscoveryPlatform(options.navigatorLike);
}
