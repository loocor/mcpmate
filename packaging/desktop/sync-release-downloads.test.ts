import { afterAll, describe, expect, test } from "bun:test";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { runCli, synchronizeReleaseDownloads } from "./sync-release-downloads";

const fixtureUrl = new URL("./fixtures/release-manifest-v2.json", import.meta.url);
const temporaryDirectories: string[] = [];

async function manifestFixture(): Promise<Record<string, unknown>> {
  const manifest = structuredClone(await Bun.file(fixtureUrl).json()) as Record<string, unknown>;
  const assets = manifest.assets as Record<string, Record<string, unknown>>;
  for (const asset of Object.values(assets)) {
    for (const field of ["githubReleaseUrl", "homebrewUrl"]) {
      if (typeof asset[field] === "string") {
        const trackedUrl = new URL(asset[field]);
        trackedUrl.host = "public.example";
        asset[field] = trackedUrl.toString();
      }
    }
  }
  return manifest;
}

async function createOutputPath(): Promise<string> {
  const directory = await mkdtemp(join(tmpdir(), "mcpmate-release-sync-test-"));
  temporaryDirectories.push(directory);
  return join(directory, "release-manifest-v2.json");
}

function response(body: unknown, status = 200, headers?: HeadersInit): Response {
  return new Response(typeof body === "string" ? body : JSON.stringify(body), { status, headers });
}

function successfulFetch(manifest: Record<string, unknown>, requests: Array<{ url: string; init?: RequestInit }>) {
  return async (input: string | URL | Request, init?: RequestInit): Promise<Response> => {
    const url = input.toString();
    requests.push({ url, init });

    if (url.endsWith("/refresh")) {
      return response({ ok: true });
    }
    if (url === `https://public.example/downloads/releases/${manifest.tag}`) {
      return response(manifest);
    }

    const assets = manifest.assets as Record<string, Record<string, unknown>>;
    const asset = Object.values(assets).find(
      (candidate) => candidate.githubReleaseUrl === url || candidate.homebrewUrl === url,
    );
    if (!asset || typeof asset.githubUrl !== "string") {
      throw new Error(`Unexpected request: ${url}`);
    }
    return response("", 302, { location: asset.githubUrl });
  };
}

afterAll(async () => {
  await Promise.all(temporaryDirectories.map((directory) => rm(directory, { recursive: true })));
});

describe("synchronizeReleaseDownloads", () => {
  test("refreshes the exact tag, validates redirects, and writes the verified manifest", async () => {
    const manifest = await manifestFixture();
    const requests: Array<{ url: string; init?: RequestInit }> = [];
    const outputPath = await createOutputPath();

    await synchronizeReleaseDownloads(
      {
        tag: "v0.3.4-beta",
        authOrigin: "https://auth.example",
        publicOrigin: "https://public.example",
        outputPath,
      },
      {
        env: { DOWNLOADS_WORKFLOW_TOKEN: "test-workflow-token" },
        fetch: successfulFetch(manifest, requests),
      },
    );

    expect(requests[0]).toEqual({
      url: "https://auth.example/internal/downloads/releases/v0.3.4-beta/refresh",
      init: {
        method: "POST",
        headers: { authorization: "Bearer test-workflow-token" },
      },
    });
    expect(requests[1]).toEqual({
      url: "https://public.example/downloads/releases/v0.3.4-beta",
      init: undefined,
    });
    expect(requests.slice(2)).toHaveLength(16);
    expect(requests.slice(2).every((request) => request.init?.redirect === "manual")).toBe(true);
    expect(JSON.parse(await readFile(outputPath, "utf8"))).toEqual(manifest);
  });

  test("synchronizes a stable exact-tag manifest with a matching channel", async () => {
    const manifest = await manifestFixture();
    manifest.tag = "v1.2.3";
    manifest.version = "1.2.3";
    manifest.releaseChannel = "stable";
    const assets = manifest.assets as Record<string, Record<string, unknown>>;
    for (const asset of Object.values(assets)) {
      for (const field of ["name", "githubUrl", "githubReleaseUrl", "homebrewUrl"]) {
        if (typeof asset[field] === "string") {
          asset[field] = asset[field]
            .replaceAll("v0.3.4-beta", "v1.2.3")
            .replaceAll("0.3.4", "1.2.3");
        }
      }
    }
    const requests: Array<{ url: string; init?: RequestInit }> = [];

    await synchronizeReleaseDownloads(
      {
        tag: "v1.2.3",
        authOrigin: "https://auth.example",
        publicOrigin: "https://public.example",
        outputPath: await createOutputPath(),
      },
      {
        env: { DOWNLOADS_WORKFLOW_TOKEN: "test-token" },
        fetch: successfulFetch(manifest, requests),
      },
    );

    expect(requests[0].url).toBe("https://auth.example/internal/downloads/releases/v1.2.3/refresh");
    expect(requests[1].url).toBe("https://public.example/downloads/releases/v1.2.3");
  });

  test("rejects a manifest channel mismatch before validating redirects", async () => {
    const manifest = await manifestFixture();
    manifest.releaseChannel = "stable";
    const requests: Array<{ url: string; init?: RequestInit }> = [];

    await expect(
      synchronizeReleaseDownloads(
        {
          tag: "v0.3.4-beta",
          authOrigin: "https://auth.example",
          publicOrigin: "https://public.example",
          outputPath: await createOutputPath(),
        },
        {
          env: { DOWNLOADS_WORKFLOW_TOKEN: "test-token" },
          fetch: successfulFetch(manifest, requests),
        },
      ),
    ).rejects.toThrow("Manifest releaseChannel does not match tag");
    expect(requests).toHaveLength(2);
  });

  test("fails before network access when DOWNLOADS_WORKFLOW_TOKEN is missing", async () => {
    let fetchCalls = 0;

    await expect(
      synchronizeReleaseDownloads(
        {
          tag: "v0.3.4-beta",
          authOrigin: "https://auth.example",
          publicOrigin: "https://public.example",
          outputPath: "/tmp/unused-release-manifest.json",
        },
        {
          env: {},
          fetch: async () => {
            fetchCalls += 1;
            return response({});
          },
        },
      ),
    ).rejects.toThrow("Missing DOWNLOADS_WORKFLOW_TOKEN");
    expect(fetchCalls).toBe(0);
  });

  test("fails loudly when the exact-tag refresh fails", async () => {
    await expect(
      synchronizeReleaseDownloads(
        {
          tag: "v0.3.4-beta",
          authOrigin: "https://auth.example",
          publicOrigin: "https://public.example",
          outputPath: "/tmp/unused-release-manifest.json",
        },
        {
          env: { DOWNLOADS_WORKFLOW_TOKEN: "test-token" },
          fetch: async () => response("refresh unavailable", 503),
        },
      ),
    ).rejects.toThrow("Exact-tag refresh failed with HTTP 503");
  });

  test("fails loudly when the public exact-tag manifest request fails", async () => {
    let call = 0;
    await expect(
      synchronizeReleaseDownloads(
        {
          tag: "v0.3.4-beta",
          authOrigin: "https://auth.example",
          publicOrigin: "https://public.example",
          outputPath: "/tmp/unused-release-manifest.json",
        },
        {
          env: { DOWNLOADS_WORKFLOW_TOKEN: "test-token" },
          fetch: async () => (++call === 1 ? response({ ok: true }) : response("missing", 404)),
        },
      ),
    ).rejects.toThrow("Public exact-tag manifest request failed with HTTP 404");
  });

  test("rejects invalid JSON from the public exact-tag manifest", async () => {
    let call = 0;
    await expect(
      synchronizeReleaseDownloads(
        {
          tag: "v0.3.4-beta",
          authOrigin: "https://auth.example",
          publicOrigin: "https://public.example",
          outputPath: "/tmp/unused-release-manifest.json",
        },
        {
          env: { DOWNLOADS_WORKFLOW_TOKEN: "test-token" },
          fetch: async () => (++call === 1 ? response({ ok: true }) : response("{not-json")),
        },
      ),
    ).rejects.toThrow("Public exact-tag manifest request returned invalid JSON");
  });

  test("rejects a public manifest for a different tag", async () => {
    const manifest = await manifestFixture();
    manifest.tag = "v0.3.5-beta";
    manifest.version = "0.3.5-beta";
    let call = 0;

    await expect(
      synchronizeReleaseDownloads(
        {
          tag: "v0.3.4-beta",
          authOrigin: "https://auth.example",
          publicOrigin: "https://public.example",
          outputPath: "/tmp/unused-release-manifest.json",
        },
        {
          env: { DOWNLOADS_WORKFLOW_TOKEN: "test-token" },
          fetch: async () => (++call === 1 ? response({ ok: true }) : response(manifest)),
        },
      ),
    ).rejects.toThrow("Manifest tag v0.3.5-beta does not match expected tag v0.3.4-beta");
  });

  test("requires every tracked download to return HTTP 302", async () => {
    const manifest = await manifestFixture();
    const requests: Array<{ url: string; init?: RequestInit }> = [];
    const fetch = successfulFetch(manifest, requests);
    let call = 0;

    await expect(
      synchronizeReleaseDownloads(
        {
          tag: "v0.3.4-beta",
          authOrigin: "https://auth.example",
          publicOrigin: "https://public.example",
          outputPath: "/tmp/unused-release-manifest.json",
        },
        {
          env: { DOWNLOADS_WORKFLOW_TOKEN: "test-token" },
          fetch: async (input, init) => {
            call += 1;
            if (call === 3) {
              return response("", 301, {
                location: "https://github.com/loocor/mcpmate/releases/download/v0.3.4-beta/file",
              });
            }
            return fetch(input, init);
          },
        },
      ),
    ).rejects.toThrow("must return HTTP 302");
  });

  test("requires the redirect Location header", async () => {
    const manifest = await manifestFixture();
    const fetch = successfulFetch(manifest, []);
    let call = 0;

    await expect(
      synchronizeReleaseDownloads(
        {
          tag: "v0.3.4-beta",
          authOrigin: "https://auth.example",
          publicOrigin: "https://public.example",
          outputPath: "/tmp/unused-release-manifest.json",
        },
        {
          env: { DOWNLOADS_WORKFLOW_TOKEN: "test-token" },
          fetch: async (input, init) => (++call === 3 ? response("", 302) : fetch(input, init)),
        },
      ),
    ).rejects.toThrow("is missing the Location header");
  });

  test("requires Location to equal the manifest githubUrl", async () => {
    const manifest = await manifestFixture();
    const fetch = successfulFetch(manifest, []);
    let call = 0;

    await expect(
      synchronizeReleaseDownloads(
        {
          tag: "v0.3.4-beta",
          authOrigin: "https://auth.example",
          publicOrigin: "https://public.example",
          outputPath: "/tmp/unused-release-manifest.json",
        },
        {
          env: { DOWNLOADS_WORKFLOW_TOKEN: "test-token" },
          fetch: async (input, init) =>
            ++call === 3
              ? response("", 302, {
                  location:
                    "https://github.com/loocor/mcpmate/releases/download/v0.3.4-beta/MCPMate_wrong.dmg",
                })
              : fetch(input, init),
        },
      ),
    ).rejects.toThrow("does not match manifest githubUrl");
  });

  test("rejects a required githubReleaseUrl on a foreign HTTPS origin", async () => {
    const manifest = await manifestFixture();
    const assets = manifest.assets as Record<string, Record<string, unknown>>;
    assets["macos-arm64-dmg"].githubReleaseUrl =
      "https://downloads.example.invalid/downloads/releases/v0.3.4-beta/macos-arm64-dmg";

    await expect(
      synchronizeReleaseDownloads(
        {
          tag: "v0.3.4-beta",
          authOrigin: "https://auth.example",
          publicOrigin: "https://public.example",
          outputPath: await createOutputPath(),
        },
        {
          env: { DOWNLOADS_WORKFLOW_TOKEN: "test-token" },
          fetch: successfulFetch(manifest, []),
        },
      ),
    ).rejects.toThrow("Invalid githubReleaseUrl for asset: macos-arm64-dmg");
  });

  test("rejects userinfo in a required githubReleaseUrl", async () => {
    const manifest = await manifestFixture();
    const assets = manifest.assets as Record<string, Record<string, unknown>>;
    assets["macos-arm64-dmg"].githubReleaseUrl =
      "https://unexpected-user@public.example/downloads/releases/v0.3.4-beta/macos-arm64-dmg";

    await expect(
      synchronizeReleaseDownloads(
        {
          tag: "v0.3.4-beta",
          authOrigin: "https://auth.example",
          publicOrigin: "https://public.example",
          outputPath: await createOutputPath(),
        },
        {
          env: { DOWNLOADS_WORKFLOW_TOKEN: "test-token" },
          fetch: successfulFetch(manifest, []),
        },
      ),
    ).rejects.toThrow("Invalid githubReleaseUrl for asset: macos-arm64-dmg");
  });

  test("rejects a required homebrewUrl on a foreign HTTPS origin", async () => {
    const manifest = await manifestFixture();
    const assets = manifest.assets as Record<string, Record<string, unknown>>;
    assets["macos-arm64-dmg"].homebrewUrl =
      "https://downloads.example.invalid/downloads/homebrew/v0.3.4-beta/macos-arm64-dmg";

    await expect(
      synchronizeReleaseDownloads(
        {
          tag: "v0.3.4-beta",
          authOrigin: "https://auth.example",
          publicOrigin: "https://public.example",
          outputPath: await createOutputPath(),
        },
        {
          env: { DOWNLOADS_WORKFLOW_TOKEN: "test-token" },
          fetch: successfulFetch(manifest, []),
        },
      ),
    ).rejects.toThrow("Invalid homebrewUrl for asset: macos-arm64-dmg");
  });

  test("requires each Homebrew redirect Location to equal the manifest githubUrl", async () => {
    const manifest = await manifestFixture();
    const fetch = successfulFetch(manifest, []);

    await expect(
      synchronizeReleaseDownloads(
        {
          tag: "v0.3.4-beta",
          authOrigin: "https://auth.example",
          publicOrigin: "https://public.example",
          outputPath: await createOutputPath(),
        },
        {
          env: { DOWNLOADS_WORKFLOW_TOKEN: "test-token" },
          fetch: async (input, init) =>
            input.toString().includes("/downloads/homebrew/")
              ? response("", 302, {
                  location:
                    "https://github.com/loocor/mcpmate/releases/download/v0.3.4-beta/MCPMate_wrong.dmg",
                })
              : fetch(input, init),
        },
      ),
    ).rejects.toThrow("does not match manifest githubUrl");
  });

  test("rejects githubUrl targets outside the MCPMate repository and exact tag", async () => {
    for (const githubUrl of [
      "https://github.com/attacker/mcpmate/releases/download/v0.3.4-beta/MCPMate.dmg",
      "https://github.com/loocor/mcpmate/releases/download/v0.3.5-beta/MCPMate.dmg",
    ]) {
      const manifest = await manifestFixture();
      const assets = manifest.assets as Record<string, Record<string, unknown>>;
      assets["macos-arm64-dmg"].githubUrl = githubUrl;
      const fetch = successfulFetch(manifest, []);

      await expect(
        synchronizeReleaseDownloads(
          {
            tag: "v0.3.4-beta",
            authOrigin: "https://auth.example",
            publicOrigin: "https://public.example",
            outputPath: "/tmp/unused-release-manifest.json",
          },
          { env: { DOWNLOADS_WORKFLOW_TOKEN: "test-token" }, fetch },
        ),
      ).rejects.toThrow("Manifest githubUrl is invalid: macos-arm64-dmg");
    }
  });

  test("runCli rejects unknown and missing arguments without network access", async () => {
    let fetchCalls = 0;
    const dependencies = {
      env: { DOWNLOADS_WORKFLOW_TOKEN: "test-token" },
      fetch: async () => {
        fetchCalls += 1;
        return response({});
      },
    };

    await expect(runCli(["--unknown", "value"], dependencies)).rejects.toThrow("Unknown argument: --unknown");
    await expect(
      runCli(
        [
          "--tag",
          "v0.3.4-beta",
          "--auth-origin",
          "https://auth.example",
          "--public-origin",
          "https://public.example",
        ],
        dependencies,
      ),
    ).rejects.toThrow("Missing required argument: --output");
    expect(fetchCalls).toBe(0);
  });

  test("runCli completes a fully mocked local synchronization", async () => {
    const manifest = await manifestFixture();
    const outputPath = await createOutputPath();
    const requests: Array<{ url: string; init?: RequestInit }> = [];

    await runCli(
      [
        "--tag",
        "v0.3.4-beta",
        "--auth-origin",
        "https://auth.example",
        "--public-origin",
        "https://public.example",
        "--output",
        outputPath,
      ],
      {
        env: { DOWNLOADS_WORKFLOW_TOKEN: "test-token" },
        fetch: successfulFetch(manifest, requests),
      },
    );

    expect(requests).toHaveLength(18);
    expect(JSON.parse(await readFile(outputPath, "utf8"))).toEqual(manifest);
  });
});
