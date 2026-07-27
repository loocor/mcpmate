import { afterEach, describe, expect, test } from "bun:test";

import { serversApi } from "./api";
import { getServerDisplayName } from "./server-display";

const originalFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = originalFetch;
});

describe("serversApi.getServer", () => {
  test("preserves standard upstream server information", async () => {
    globalThis.fetch = async () =>
      new Response(
        JSON.stringify({
          data: {
            id: "server-everything",
            name: "everything",
            status: "connected",
            server_info: {
              name: "everything-server",
              title: "Everything Reference Server",
              version: "1.0.0",
            },
          },
        }),
        {
          headers: { "content-type": "application/json" },
        },
      );

    const server = await serversApi.getServer("server-everything");

    expect(server.server_info).toEqual({
      name: "everything-server",
      title: "Everything Reference Server",
      version: "1.0.0",
    });
    expect(getServerDisplayName(server)).toBe("Everything Reference Server");
  });

  test("preserves capability catalog revisions for direct exposure updates", async () => {
    globalThis.fetch = async () =>
      new Response(
        JSON.stringify({
          data: {
            id: "server-sequential-thinking",
            name: "sequential-thinking-server",
            status: "connected",
            source_revision_set: {
              "server-sequential-thinking": 11,
            },
          },
        }),
        {
          headers: { "content-type": "application/json" },
        },
      );

    const server = await serversApi.getServer("server-sequential-thinking");

    expect(server.source_revision_set).toEqual({
      "server-sequential-thinking": 11,
    });
  });

	test("preserves the typed capability lifecycle summary", async () => {
		const kind = {
			declaration: "supported",
			inventory: "complete",
			currentCount: 1,
			currentAvailable: true,
			lastError: null,
		};
		globalThis.fetch = async () =>
			new Response(
				JSON.stringify({
					data: {
						id: "server-a",
						name: "server-a",
						status: "idle",
						capability: {
							snapshotState: "ready",
							revision: 7,
							observedAt: "2026-07-20T10:00:00Z",
							tools: kind,
							prompts: { ...kind, currentCount: 0 },
							resources: {
								...kind,
								declaration: "unsupported",
								currentCount: 0,
								currentAvailable: false,
							},
							resourceTemplates: { ...kind, currentCount: 0 },
						},
					},
				}),
				{ headers: { "content-type": "application/json" } },
			);

		const server = await serversApi.getServer("server-a");

		expect(server.capability?.revision).toBe(7);
		expect(server.capability?.tools.currentCount).toBe(1);
		expect(server.capability?.resources.declaration).toBe("unsupported");
	});

	test("rejects the legacy boolean and count capability summary", async () => {
		globalThis.fetch = async () =>
			new Response(
				JSON.stringify({
					data: {
						id: "server-a",
						name: "server-a",
						status: "idle",
						capability: {
							supports_tools: true,
							tools_count: 1,
						},
					},
				}),
				{ headers: { "content-type": "application/json" } },
			);

		expect(serversApi.getServer("server-a")).rejects.toThrow(
			"Invalid server capability snapshot state",
		);
	});
});

describe("serversApi.refreshCapabilities", () => {
	test("refreshes the complete server catalog through one POST request", async () => {
		let request: { url: string; init?: RequestInit } | undefined;
		globalThis.fetch = async (input, init) => {
			request = { url: String(input), init };
			return new Response(
				JSON.stringify({
					success: true,
					data: {
						server_id: "server-everything",
						catalog_revision: 17,
						catalog_changed: false,
					},
				}),
				{ headers: { "content-type": "application/json" } },
			);
		};

		const result = await serversApi.refreshCapabilities("server-everything");

		expect(request?.url).toEndWith(
			"/api/mcp/servers/capabilities/refresh",
		);
		expect(request?.init?.method).toBe("POST");
		expect(JSON.parse(String(request?.init?.body))).toEqual({
			id: "server-everything",
		});
		expect(result).toEqual({
			server_id: "server-everything",
			catalog_revision: 17,
			catalog_changed: false,
		});
	});
});

describe("serversApi capability lists", () => {
	test("loads all capability kinds through the batch lists endpoint", async () => {
		let requestUrl: string | undefined;
		globalThis.fetch = async (input) => {
			requestUrl = String(input);
			return new Response(
				JSON.stringify({
					success: true,
					data: {
						tools: {
							items: [
								{
									id: "STOOLlegacy",
									ref_id: "cref_sha256:stable",
									name: "sequential_thinking_sequentialthinking",
									tool_name: "sequentialthinking",
									description: "Think through a problem step by step.",
									inputSchema: {
										type: "object",
										properties: {
											thought: { type: "string" },
										},
									},
								},
							],
						},
						resources: { items: [] },
						prompts: { items: [] },
						resource_templates: { items: [] },
					},
				}),
				{ headers: { "content-type": "application/json" } },
			);
		};

		const result = await serversApi.listAllCapabilities("server-sequential-thinking");

		expect(requestUrl).toContain("/api/mcp/servers/capabilities/lists");
		expect(requestUrl).toContain("id=server-sequential-thinking");
		expect(result.tools.items).toEqual([
			{
				id: "cref_sha256:stable",
				ref_id: "cref_sha256:stable",
				name: "sequential_thinking_sequentialthinking",
				tool_name: "sequentialthinking",
				description: "Think through a problem step by step.",
				inputSchema: {
					type: "object",
					properties: {
						thought: { type: "string" },
					},
				},
			},
		]);
		expect(result.resources.items).toEqual([]);
		expect(result.prompts.items).toEqual([]);
		expect(result.templates.items).toEqual([]);
	});

	test("returns transport_error for all kinds when batch list request fails", async () => {
		globalThis.fetch = async () => {
			throw new Error("network down");
		};

		const result = await serversApi.listAllCapabilities("server-sequential-thinking");

		expect(result.tools).toEqual({
			items: [],
			state: "transport_error",
			degraded_reason: "request_failed",
		});
		expect(result.resources).toEqual(result.tools);
		expect(result.prompts).toEqual(result.tools);
		expect(result.templates).toEqual(result.tools);
	});
});
