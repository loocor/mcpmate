import { afterEach, describe, expect, test } from "bun:test";

import {
	inspectorApi,
	type InspectorTemplateReadRequest,
} from "./api";

const originalFetch = globalThis.fetch;

afterEach(() => {
	globalThis.fetch = originalFetch;
});

describe("inspectorApi.templateRead", () => {
	test("posts the template and arguments without a client-expanded URI", async () => {
		const requests: Array<{ input: string; init?: RequestInit }> = [];
		globalThis.fetch = ((input: string | URL | Request, init?: RequestInit) => {
			requests.push({ input: String(input), init });
			return Promise.resolve(
				new Response(
					JSON.stringify({
						success: true,
						data: {
							expanded_uri: "test://dynamic/42",
							result: { contents: [] },
						},
					}),
					{ headers: { "content-type": "application/json" } },
				),
			);
		}) as typeof fetch;

		const request: InspectorTemplateReadRequest = {
			uri_template: "test://dynamic/{resourceId}",
			arguments: { resourceId: 42 },
			mode: "native",
			server_id: "server-1",
			session_id: "session-1",
			timeout_ms: 8_000,
		};
		expect(inspectorApi.templateRead).toBeTypeOf("function");
		await inspectorApi.templateRead(request);

		expect(requests).toHaveLength(1);
		expect(requests[0].input).toBe(
			"http://127.0.0.1:8080/api/mcp/inspector/template/read",
		);
		expect(requests[0].init).toEqual(
			expect.objectContaining({
				method: "POST",
				body: JSON.stringify({
					uri_template: "test://dynamic/{resourceId}",
					arguments: { resourceId: 42 },
					mode: "native",
					server_id: "server-1",
					session_id: "session-1",
					timeout_ms: 8_000,
				}),
			}),
		);
	});
});
