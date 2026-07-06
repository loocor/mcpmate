import { describe, expect, it } from "bun:test";
import {
	appendSessionHandshakeEvents,
	buildSessionHandshakeLogEntries,
} from "./inspector-session-handshake";
import type { InspectorSessionHandshakeData } from "./types";

const sampleHandshake: InspectorSessionHandshakeData = {
	protocol_version: "2025-03-26",
	server_name: "demo",
	server_title: "Demo Server",
	messages: [
		{
			direction: "outbound",
			method: "initialize",
			payload: { jsonrpc: "2.0", id: 1, method: "initialize", params: {} },
		},
		{
			direction: "inbound",
			method: "initialize",
			payload: { jsonrpc: "2.0", id: 1, result: { protocolVersion: "2025-03-26" } },
		},
		{
			direction: "outbound",
			method: "notifications/initialized",
			payload: { jsonrpc: "2.0", method: "notifications/initialized" },
		},
	],
};

describe("inspector session handshake", () => {
	it("builds MCP exchange log entries in handshake order", () => {
		const entries = buildSessionHandshakeLogEntries(sampleHandshake, {
			serverId: "srv-1",
			mode: "native",
			sessionId: "sess-1",
		});
		expect(entries).toHaveLength(3);
		expect(entries[0]?.data).toMatchObject({
			event: "mcp_exchange",
			direction: "outbound",
			method: "initialize",
		});
		expect(entries[1]?.response).toBeTruthy();
		expect(entries[2]?.request).toMatchObject({
			method: "notifications/initialized",
		});
	});

	it("appends handshake events through the provided callback", () => {
		const appended: string[] = [];
		appendSessionHandshakeEvents(
			sampleHandshake,
			{ serverId: "srv-1", mode: "native", sessionId: "sess-1" },
			(input) => {
				if (input.data.event === "mcp_exchange") {
					appended.push(input.data.method);
				}
			},
		);
		expect(appended).toEqual(["initialize", "initialize", "notifications/initialized"]);
	});
});
