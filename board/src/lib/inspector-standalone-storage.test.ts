import { afterEach, beforeEach, describe, expect, it } from "bun:test";
import {
	clearInspectorStandaloneConnectionTargetKey,
	clearInspectorStandaloneEvents,
	loadInspectorStandaloneConnectionTargetKey,
	loadInspectorStandaloneEvents,
	loadInspectorStandaloneUiState,
	saveInspectorStandaloneConnectionTargetKey,
	saveInspectorStandaloneEvents,
	saveInspectorStandaloneUiState,
} from "./inspector-standalone-storage";
import { createInspectorLogEntry } from "./inspector-event-log";

const SERVER_ID = "test-server";

function createMemoryStorage(): Storage {
	const store = new Map<string, string>();
	return {
		get length() {
			return store.size;
		},
		clear() {
			store.clear();
		},
		getItem(key: string) {
			return store.get(key) ?? null;
		},
		key(index: number) {
			return Array.from(store.keys())[index] ?? null;
		},
		removeItem(key: string) {
			store.delete(key);
		},
		setItem(key: string, value: string) {
			store.set(key, value);
		},
	};
}

beforeEach(() => {
	Object.defineProperty(globalThis, "window", {
		value: {
			sessionStorage: createMemoryStorage(),
			localStorage: createMemoryStorage(),
		},
		configurable: true,
	});
});

afterEach(() => {
	window.sessionStorage.clear();
	window.localStorage.clear();
});

describe("inspector standalone storage", () => {
	it("persists ui state in session storage", () => {
		saveInspectorStandaloneUiState({
			mode: "proxy",
			capabilitySearch: "fetch",
			kindFilters: ["tools"],
			eventsPanelExpanded: true,
			eventsPanelHeight: 320,
			eventsPanelPinned: true,
			eventsSearch: "error",
		});
		expect(loadInspectorStandaloneUiState()).toEqual({
			mode: "proxy",
			capabilitySearch: "fetch",
			kindFilters: ["tools"],
			eventsPanelExpanded: true,
			eventsPanelHeight: 320,
			eventsPanelPinned: true,
			eventsSearch: "error",
		});
	});

	it("persists and clears event logs per server", () => {
		const entry = createInspectorLogEntry({
			data: {
				event: "session_open",
				server_id: SERVER_ID,
				mode: "native",
			},
			request: { server_id: SERVER_ID },
			response: { session_id: "sess-1" },
		});
		saveInspectorStandaloneEvents(SERVER_ID, [entry]);
		expect(loadInspectorStandaloneEvents(SERVER_ID)).toEqual([entry]);
		clearInspectorStandaloneEvents(SERVER_ID);
		expect(loadInspectorStandaloneEvents(SERVER_ID)).toEqual([]);
	});

	it("persists the last connected target in local storage", () => {
		saveInspectorStandaloneConnectionTargetKey("scratch:scratch-fetch");
		expect(loadInspectorStandaloneConnectionTargetKey()).toBe("scratch:scratch-fetch");

		saveInspectorStandaloneConnectionTargetKey("managed:managed-fetch");
		expect(loadInspectorStandaloneConnectionTargetKey()).toBe("managed:managed-fetch");

		clearInspectorStandaloneConnectionTargetKey();
		expect(loadInspectorStandaloneConnectionTargetKey()).toBeNull();
	});

	it("ignores invalid connected target keys", () => {
		window.localStorage.setItem("mcp_inspector_standalone_connected_target", "scratch:");
		expect(loadInspectorStandaloneConnectionTargetKey()).toBeNull();

		window.localStorage.setItem("mcp_inspector_standalone_connected_target", "legacy-fetch");
		expect(loadInspectorStandaloneConnectionTargetKey()).toBeNull();
	});
});
