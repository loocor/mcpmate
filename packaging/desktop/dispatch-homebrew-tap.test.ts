import { describe, expect, test } from "bun:test";

import { dispatchHomebrewTap } from "./dispatch-homebrew-tap";

describe("dispatchHomebrewTap", () => {
  test("fails before network access when HOMEBREW_TAP_TOKEN is missing", async () => {
    let fetchCalls = 0;

    await expect(
      dispatchHomebrewTap(
        {
          tag: "v1.2.3-beta.7",
          manifestUrl: "https://public.mcp.umate.ai/downloads/releases/v1.2.3-beta.7",
        },
        {
          env: {},
          fetch: async () => {
            fetchCalls += 1;
            return new Response(null, { status: 204 });
          },
        },
      ),
    ).rejects.toThrow("Missing HOMEBREW_TAP_TOKEN");
    expect(fetchCalls).toBe(0);
  });

  test("sends the exact repository_dispatch payload", async () => {
    const requests: Array<{ url: string; init?: RequestInit }> = [];

    await dispatchHomebrewTap(
      {
        tag: "v1.2.3",
        manifestUrl: "https://public.mcp.umate.ai/downloads/releases/v1.2.3",
      },
      {
        env: { HOMEBREW_TAP_TOKEN: "test-token" },
        fetch: async (input, init) => {
          requests.push({ url: input.toString(), init });
          return new Response(null, { status: 204 });
        },
      },
    );

    expect(requests).toHaveLength(1);
    expect(requests[0].url).toBe("https://api.github.com/repos/loocor/homebrew-tap/dispatches");
    expect(requests[0].init).toEqual({
      method: "POST",
      headers: {
        accept: "application/vnd.github+json",
        authorization: "Bearer test-token",
        "content-type": "application/json",
        "x-github-api-version": "2022-11-28",
      },
      body: JSON.stringify({
        event_type: "mcpmate_release",
        client_payload: {
          tag: "v1.2.3",
          manifest_url: "https://public.mcp.umate.ai/downloads/releases/v1.2.3",
        },
      }),
    });
  });

  test("rejects unsupported tags and mismatched manifest URLs before network access", async () => {
    let fetchCalls = 0;
    const dependencies = {
      env: { HOMEBREW_TAP_TOKEN: "test-token" },
      fetch: async () => {
        fetchCalls += 1;
        return new Response(null, { status: 204 });
      },
    };

    await expect(
      dispatchHomebrewTap(
        {
          tag: "v1.2.3-nightly",
          manifestUrl: "https://public.mcp.umate.ai/downloads/releases/v1.2.3-nightly",
        },
        dependencies,
      ),
    ).rejects.toThrow("Unsupported release tag: v1.2.3-nightly");
    await expect(
      dispatchHomebrewTap(
        {
          tag: "v1.2.3-beta",
          manifestUrl: "https://public.mcp.umate.ai/downloads/releases/v1.2.3",
        },
        dependencies,
      ),
    ).rejects.toThrow("Manifest URL must match the exact release tag");
    expect(fetchCalls).toBe(0);
  });
});
