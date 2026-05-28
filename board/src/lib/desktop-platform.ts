import { isTauriEnvironmentSync } from "./platform";
import type { AdminDiscoveryPlatform } from "./admin-discovery";

type TauriInvoke = (command: string) => Promise<unknown>;

interface ReadTauriPlatformOptions {
	isTauri?: () => boolean;
	invoke?: TauriInvoke;
}

export function normalizeDesktopPlatform(value: unknown): AdminDiscoveryPlatform | undefined {
	if (value === "macos" || value === "windows" || value === "linux") {
		return value;
	}
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
