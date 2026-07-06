import type { InspectorMode } from "./inspector-capability";
import type { InspectorLogEventEntry } from "./inspector-event-log";

export type InspectorRunCapabilityTarget = {
	kind: string;
	key: string;
	execution_name?: string;
};

export type InspectorRunState = {
	server_id: string;
	server_name: string;
	mode: InspectorMode;
	session_id?: string;
	session_expires_at_epoch_ms?: number;
	capability?: InspectorRunCapabilityTarget;
	active_call_id?: string | null;
	submitting: boolean;
	event_count: number;
	last_event_at_ms?: number;
};

export type InspectorEvidenceBundle = {
	run_state: InspectorRunState;
	events: InspectorLogEventEntry[];
};

export function buildInspectorRunState(input: {
	serverId: string;
	serverName: string;
	mode: InspectorMode;
	sessionId?: string;
	sessionExpiresAtEpochMs?: number;
	capability?: InspectorRunCapabilityTarget | null;
	activeCallId?: string | null;
	submitting: boolean;
	events: InspectorLogEventEntry[];
}): InspectorRunState {
	const lastEvent = input.events.at(-1);
	return {
		server_id: input.serverId,
		server_name: input.serverName,
		mode: input.mode,
		session_id: input.sessionId,
		session_expires_at_epoch_ms: input.sessionExpiresAtEpochMs,
		capability: input.capability ?? undefined,
		active_call_id: input.activeCallId ?? null,
		submitting: input.submitting,
		event_count: input.events.length,
		last_event_at_ms: lastEvent?.timestamp,
	};
}

export function buildInspectorEvidenceBundle(
	runState: InspectorRunState,
	events: InspectorLogEventEntry[],
): InspectorEvidenceBundle {
	return {
		run_state: runState,
		events,
	};
}

export function serializeInspectorEvidenceBundle(bundle: InspectorEvidenceBundle): string {
	return JSON.stringify(bundle, null, 2);
}

export function serializeInspectorEventEntry(entry: InspectorLogEventEntry): string {
	return JSON.stringify(entry, null, 2);
}
