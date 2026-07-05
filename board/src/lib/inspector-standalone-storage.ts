import type { InspectorMode } from "./inspector-capability";
import type { InspectorLogEventEntry } from "./inspector-event-log";

const UI_STORAGE_KEY = "mcp_inspector_standalone_ui";
const CONNECTION_TARGET_STORAGE_KEY = "mcp_inspector_standalone_connected_target";
const EVENTS_STORAGE_PREFIX = "mcp_inspector_standalone_events:";

export type InspectorStandaloneCapabilityKind = "tools" | "resources" | "prompts" | "templates";
export type InspectorStandaloneConnectionTargetKey = `managed:${string}` | `scratch:${string}`;

export type InspectorStandaloneUiState = {
	mode: InspectorMode;
	capabilitySearch: string;
	kindFilters: InspectorStandaloneCapabilityKind[];
	eventsPanelExpanded: boolean;
	eventsPanelHeight: number;
	eventsPanelPinned: boolean;
	eventsSearch: string;
};

const DEFAULT_UI_STATE: InspectorStandaloneUiState = {
	mode: "native",
	capabilitySearch: "",
	kindFilters: [],
	eventsPanelExpanded: false,
	eventsPanelHeight: 280,
	eventsPanelPinned: false,
	eventsSearch: "",
};

function eventsStorageKey(serverId: string): string {
	return `${EVENTS_STORAGE_PREFIX}${serverId}`;
}

function readJson<T>(raw: string | null): T | null {
	if (!raw) {
		return null;
	}
	try {
		return JSON.parse(raw) as T;
	} catch {
		return null;
	}
}

function isCapabilityKind(value: unknown): value is InspectorStandaloneCapabilityKind {
	return (
		value === "tools" ||
		value === "resources" ||
		value === "prompts" ||
		value === "templates"
	);
}

function isConnectionTargetKey(
	value: unknown,
): value is InspectorStandaloneConnectionTargetKey {
	return (
		typeof value === "string" &&
		(value.startsWith("managed:") || value.startsWith("scratch:")) &&
		value.split(":")[1]?.trim().length > 0
	);
}

export function loadInspectorStandaloneUiState(): InspectorStandaloneUiState {
	if (typeof window === "undefined") {
		return DEFAULT_UI_STATE;
	}
	const stored = readJson<Partial<InspectorStandaloneUiState>>(
		window.sessionStorage.getItem(UI_STORAGE_KEY),
	);
	if (!stored) {
		return DEFAULT_UI_STATE;
	}
	return {
		mode: stored.mode === "proxy" ? "proxy" : DEFAULT_UI_STATE.mode,
		capabilitySearch:
			typeof stored.capabilitySearch === "string"
				? stored.capabilitySearch
				: DEFAULT_UI_STATE.capabilitySearch,
		kindFilters: Array.isArray(stored.kindFilters)
			? stored.kindFilters.filter(isCapabilityKind)
			: DEFAULT_UI_STATE.kindFilters,
		eventsPanelExpanded:
			typeof stored.eventsPanelExpanded === "boolean"
				? stored.eventsPanelExpanded
				: DEFAULT_UI_STATE.eventsPanelExpanded,
		eventsPanelHeight:
			typeof stored.eventsPanelHeight === "number" && stored.eventsPanelHeight > 0
				? stored.eventsPanelHeight
				: DEFAULT_UI_STATE.eventsPanelHeight,
		eventsPanelPinned:
			typeof stored.eventsPanelPinned === "boolean"
				? stored.eventsPanelPinned
				: DEFAULT_UI_STATE.eventsPanelPinned,
		eventsSearch:
			typeof stored.eventsSearch === "string"
				? stored.eventsSearch
				: DEFAULT_UI_STATE.eventsSearch,
	};
}

export function saveInspectorStandaloneUiState(state: InspectorStandaloneUiState): void {
	if (typeof window === "undefined") {
		return;
	}
	window.sessionStorage.setItem(UI_STORAGE_KEY, JSON.stringify(state));
}

export function loadInspectorStandaloneConnectionTargetKey():
	| InspectorStandaloneConnectionTargetKey
	| null {
	if (typeof window === "undefined") {
		return null;
	}
	const stored = window.localStorage.getItem(CONNECTION_TARGET_STORAGE_KEY);
	return isConnectionTargetKey(stored) ? stored : null;
}

export function saveInspectorStandaloneConnectionTargetKey(
	targetKey: InspectorStandaloneConnectionTargetKey,
): void {
	if (typeof window === "undefined") {
		return;
	}
	window.localStorage.setItem(CONNECTION_TARGET_STORAGE_KEY, targetKey);
}

export function clearInspectorStandaloneConnectionTargetKey(): void {
	if (typeof window === "undefined") {
		return;
	}
	window.localStorage.removeItem(CONNECTION_TARGET_STORAGE_KEY);
}

export function loadInspectorStandaloneEvents(serverId: string): InspectorLogEventEntry[] {
	if (!serverId || typeof window === "undefined") {
		return [];
	}
	const stored = readJson<InspectorLogEventEntry[]>(
		window.localStorage.getItem(eventsStorageKey(serverId)),
	);
	if (!Array.isArray(stored)) {
		return [];
	}
	return stored;
}

export function saveInspectorStandaloneEvents(
	serverId: string,
	events: InspectorLogEventEntry[],
): void {
	if (!serverId || typeof window === "undefined") {
		return;
	}
	try {
		window.localStorage.setItem(eventsStorageKey(serverId), JSON.stringify(events));
	} catch (error) {
		console.warn("Failed to persist inspector events", error);
	}
}

export function clearInspectorStandaloneEvents(serverId: string): void {
	if (!serverId || typeof window === "undefined") {
		return;
	}
	window.localStorage.removeItem(eventsStorageKey(serverId));
}
