import { afterEach, describe, expect, test } from "bun:test";

import { inspectorApi, isInspectorSessionUnavailableError } from "./api";

const originalFetch = globalThis.fetch;

afterEach(() => {
	globalThis.fetch = originalFetch;
});

describe("inspectorApi.sessionRefresh", () => {
	test("renews one explicit inspector session", async () => {
		const requests: Array<{ input: string; init?: RequestInit }> = [];
		globalThis.fetch = ((input: string | URL | Request, init?: RequestInit) => {
			requests.push({ input: String(input), init });
			return Promise.resolve(
				new Response(
					JSON.stringify({
						success: true,
						data: {
							session_id: "session-1",
							server_id: "server-1",
							mode: "native",
							expires_at_epoch_ms: 1_800_000_000_000,
						},
					}),
					{ headers: { "content-type": "application/json" } },
				),
			);
		}) as typeof fetch;

		const response = await inspectorApi.sessionRefresh({
			session_id: "session-1",
		});

		expect(response.data?.expires_at_epoch_ms).toBe(1_800_000_000_000);
		expect(requests).toHaveLength(1);
		expect(requests[0].input).toBe(
			"http://127.0.0.1:8080/api/mcp/inspector/session/refresh",
		);
		expect(requests[0].init).toEqual(
			expect.objectContaining({
				method: "POST",
				body: JSON.stringify({ session_id: "session-1" }),
			}),
		);
	});
});

describe("isInspectorSessionUnavailableError", () => {
	test("only classifies confirmed session loss as unavailable", () => {
		expect(
			isInspectorSessionUnavailableError(
				new Error("Inspector session 'session-1' not found or expired"),
			),
		).toBe(true);
		expect(
			isInspectorSessionUnavailableError(
				new Error(
					"Native Inspector session 'session-1' is no longer connected for server 'server-1'",
				),
			),
		).toBe(true);
		expect(
			isInspectorSessionUnavailableError(
				new Error("Inspector session refresh exceeded 8000 ms"),
			),
		).toBe(false);
		expect(
			isInspectorSessionUnavailableError(
				new Error("API Error: 503 Service Unavailable"),
			),
		).toBe(false);
	});
});
