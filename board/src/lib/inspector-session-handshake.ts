import type { InspectorMode } from "./inspector-capability";
import {
	createInspectorLogEntry,
	type CreateInspectorLogEntryInput,
	type InspectorLogEventEntry,
} from "./inspector-event-log";
import type { InspectorSessionHandshakeData } from "./types";

export type AppendInspectorLogEvent = (
	input: CreateInspectorLogEntryInput,
) => void;

function handshakeMessageToLogInput(
	message: InspectorSessionHandshakeData["messages"][number],
	context: {
		serverId: string;
		mode: InspectorMode;
		sessionId: string;
	},
): CreateInspectorLogEntryInput {
	const isOutbound = message.direction === "outbound";

	return {
		data: {
			event: "mcp_exchange",
			direction: message.direction === "inbound" ? "inbound" : "outbound",
			method: message.method,
			server_id: context.serverId,
			mode: context.mode,
			session_id: context.sessionId,
		},
		request: isOutbound ? message.payload : undefined,
		response: !isOutbound ? message.payload : undefined,
	};
}

export function buildSessionHandshakeLogEntries(
	handshake: InspectorSessionHandshakeData,
	context: {
		serverId: string;
		mode: InspectorMode;
		sessionId: string;
	},
): InspectorLogEventEntry[] {
	return handshake.messages.map((message) =>
		createInspectorLogEntry(handshakeMessageToLogInput(message, context)),
	);
}

export function appendSessionHandshakeEvents(
	handshake: InspectorSessionHandshakeData | null | undefined,
	context: {
		serverId: string;
		mode: InspectorMode;
		sessionId: string;
	},
	appendEvent: AppendInspectorLogEvent,
): void {
	if (!handshake?.messages.length) {
		return;
	}
	for (const message of handshake.messages) {
		appendEvent(handshakeMessageToLogInput(message, context));
	}
}
