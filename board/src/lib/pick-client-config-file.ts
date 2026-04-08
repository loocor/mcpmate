import { isTauriEnvironmentSync } from "./platform";

export type FileWithFilesystemPath = File & { path?: string };

/**
 * Reads an absolute path when the host exposes it on `File` (e.g. some desktop shells).
 * Standard browser file inputs do not populate this.
 */
export function readAbsolutePathFromFile(file: File): string | null {
	const raw = (file as FileWithFilesystemPath).path;
	if (typeof raw !== "string") {
		return null;
	}
	const trimmed = raw.trim();
	return trimmed.length > 0 ? trimmed : null;
}

/**
 * Opens a native file picker (Tauri desktop only). Returns `null` if cancelled or unavailable.
 */
export async function pickClientConfigFilePath(title: string): Promise<string | null> {
	if (!isTauriEnvironmentSync()) {
		return null;
	}
	const { open } = await import("@tauri-apps/plugin-dialog");
	const selected = await open({
		title,
		multiple: false,
	});
	if (selected == null) {
		return null;
	}
	if (Array.isArray(selected)) {
		return selected[0] ?? null;
	}
	return selected;
}
