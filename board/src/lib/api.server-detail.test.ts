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
