import { QueryClient } from "@tanstack/react-query";
import { describe, expect, test } from "bun:test";

import { syncAuthenticatedServerCapabilities } from "./server-auth-sync";

describe("syncAuthenticatedServerCapabilities", () => {
  test("refreshes discovery and invalidates every active server view", async () => {
    const serverId = "server-sentry";
    const queryClient = new QueryClient();
    const queryKeys = [
      ["server", serverId],
      ["servers"],
      ["server-oauth", serverId],
      ["server-cap", "all", serverId],
    ] as const;
    for (const queryKey of queryKeys) {
      queryClient.setQueryData(queryKey, { stale: true });
    }

    const refreshedServerIds: string[] = [];
    await syncAuthenticatedServerCapabilities({
      serverId,
      queryClient,
      refreshCapabilities: async (id) => {
        refreshedServerIds.push(id);
      },
    });

    expect(refreshedServerIds).toEqual([serverId]);
    for (const queryKey of queryKeys) {
      expect(queryClient.getQueryState(queryKey)?.isInvalidated).toBe(true);
    }
  });

  test("invalidates stale views when authenticated discovery fails", async () => {
    const serverId = "server-sentry";
    const queryClient = new QueryClient();
    const capabilityQueryKey = ["server-cap", "all", serverId] as const;
    queryClient.setQueryData(capabilityQueryKey, { authentication: "required" });

    await expect(
      syncAuthenticatedServerCapabilities({
        serverId,
        queryClient,
        refreshCapabilities: async () => {
          throw new Error("discovery failed");
        },
      }),
    ).rejects.toThrow("discovery failed");

    expect(queryClient.getQueryState(capabilityQueryKey)?.isInvalidated).toBe(
      true,
    );
  });
});
