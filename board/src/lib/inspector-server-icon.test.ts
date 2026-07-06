import { describe, expect, it } from "bun:test";
import {
	extractHandshakeServerIconSrc,
	resolveInspectorServerIconSrc,
} from "./inspector-server-icon";
import type { InspectorSessionHandshakeData, ServerSummary } from "./types";

describe("inspector server icon resolution", () => {
	it("prefers runtime handshake icons over recorded metadata", () => {
		const server: Pick<ServerSummary, "icons" | "meta" | "name"> = {
			name: "Demo",
			icons: [{ src: "https://example.com/builtin.png" }],
			meta: { icons: [{ src: "https://example.com/recorded.png" }] },
		};

		expect(
			resolveInspectorServerIconSrc(server, {
				runtimeIconSrc: "https://example.com/runtime.png",
			}),
		).toBe("https://example.com/runtime.png");
	});

	it("prefers built-in icons over MCPMate recorded icons", () => {
		const server: Pick<ServerSummary, "icons" | "meta" | "name"> = {
			name: "Demo",
			icons: [{ src: "https://example.com/builtin.png" }],
			meta: { icons: [{ src: "https://example.com/recorded.png" }] },
		};

		expect(resolveInspectorServerIconSrc(server)).toBe(
			"https://example.com/builtin.png",
		);
	});

	it("falls back to recorded metadata icons and extras", () => {
		expect(
			resolveInspectorServerIconSrc({
				name: "Demo",
				meta: { icons: [{ src: "https://example.com/recorded.png" }] },
			}),
		).toBe("https://example.com/recorded.png");

		expect(
			resolveInspectorServerIconSrc({
				name: "Demo",
				meta: { extras: { iconUrl: "https://example.com/extras.png" } },
			}),
		).toBe("https://example.com/extras.png");
	});

	it("extracts upstream icons from initialize handshake responses", () => {
		const handshake: InspectorSessionHandshakeData = {
			protocol_version: "2025-03-26",
			server_name: "demo",
			messages: [
				{
					direction: "inbound",
					method: "initialize",
					payload: {
						jsonrpc: "2.0",
						id: 1,
						result: {
							serverInfo: {
								name: "demo",
								icons: [{ src: "https://example.com/upstream.png" }],
							},
						},
					},
				},
			],
		};

		expect(extractHandshakeServerIconSrc(handshake)).toBe(
			"https://example.com/upstream.png",
		);
	});
});
