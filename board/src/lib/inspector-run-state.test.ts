import { describe, expect, it } from "bun:test";
import { createInspectorLogEntry } from "./inspector-event-log";
import {
	buildInspectorEvidenceBundle,
	buildInspectorRunState,
	serializeInspectorEvidenceBundle,
} from "./inspector-run-state";

describe("buildInspectorRunState", () => {
	it("captures session and capability context for evidence export", () => {
		const events = [
			createInspectorLogEntry({
				data: {
					event: "session_open",
					server_id: "srv-1",
					mode: "native",
				},
			}),
		];
		const state = buildInspectorRunState({
			serverId: "srv-1",
			serverName: "Demo",
			mode: "native",
			sessionId: "sess-1",
			sessionExpiresAtEpochMs: 1_700_000_000_000,
			capability: { kind: "tools", key: "echo", execution_name: "echo" },
			activeCallId: null,
			submitting: false,
			events,
		});
		expect(state.event_count).toBe(1);
		expect(state.capability?.key).toBe("echo");
		expect(state.session_id).toBe("sess-1");

		const bundle = buildInspectorEvidenceBundle(state, events);
		const serialized = serializeInspectorEvidenceBundle(bundle);
		expect(serialized).toContain('"run_state"');
		expect(serialized).toContain('"events"');
	});
});
