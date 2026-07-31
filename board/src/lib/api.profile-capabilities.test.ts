import { afterEach, describe, expect, test } from "bun:test";

import { configSuitsApi } from "./api";

const originalFetch = globalThis.fetch;

afterEach(() => {
  globalThis.fetch = originalFetch;
});

describe("configSuitsApi capability lists", () => {
  test("maps stable CapabilityRef identities to profile capability item ids", async () => {
    globalThis.fetch = async () =>
      new Response(
        JSON.stringify({
          success: true,
          data: {
            profile_id: "profile-a",
            profile_name: "Profile A",
            tools: [
              {
                ref_id: "capref-tool-a",
                server_id: "server-a",
                server_name: "Server A",
                tool_name: "analyze",
                unique_name: "server_a__analyze",
                description: "Analyze a payload",
                enabled: true,
                state: "active",
                state_generation: 1,
                allowed_operations: ["enable", "disable"],
              },
            ],
            source_revision_set: {
              "server-a": 4,
            },
          },
        }),
        {
          headers: { "content-type": "application/json" },
        },
      );

    const response = await configSuitsApi.getTools("profile-a");

    expect(response.tools).toEqual([
      {
        id: "capref-tool-a",
        server_id: "server-a",
        server_name: "Server A",
        tool_name: "analyze",
        unique_name: "server_a__analyze",
        description: "Analyze a payload",
        enabled: true,
        state: "active",
        state_generation: 1,
        allowed_operations: ["enable", "disable"],
      },
    ]);
  });
});
