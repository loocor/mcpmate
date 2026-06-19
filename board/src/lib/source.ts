import type { ServerSource } from "./types";

export function isRegistrySource(source?: ServerSource | null): boolean {
	return source?.type === "registry" && Boolean(source.ref);
}

export function registryRef(source?: ServerSource | null): string | null {
	if (source?.type === "registry" && source.ref) {
		return source.ref;
	}
	return null;
}
