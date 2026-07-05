/** Legacy route id for the removed inline ephemeral connect flow. */
export const INSPECTOR_EPHEMERAL_SERVER_ID = "__probe__";

const LEGACY_EPHEMERAL_DRAFT_STORAGE_KEY = "mcp_inspector_ephemeral_draft";

export function isInspectorEphemeralServerId(serverId: string | undefined): boolean {
	return serverId === INSPECTOR_EPHEMERAL_SERVER_ID;
}

export function clearLegacyInspectorEphemeralDraft(): void {
	try {
		sessionStorage.removeItem(LEGACY_EPHEMERAL_DRAFT_STORAGE_KEY);
	} catch {
		// Ignore storage access errors.
	}
}

export function inspectorServerInitial(name: string): string {
	const trimmed = name.trim();
	if (!trimmed) return "?";
	return trimmed.charAt(0).toUpperCase();
}
