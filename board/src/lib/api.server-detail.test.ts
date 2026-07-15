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
});
