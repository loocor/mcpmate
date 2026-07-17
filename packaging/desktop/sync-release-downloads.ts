import { validateReleaseManifest } from "./render-release-downloads";

interface SyncOptions {
  tag: string;
  authOrigin: string;
  publicOrigin: string;
  outputPath: string;
}

interface SyncDependencies {
  env?: Record<string, string | undefined>;
  fetch?: (input: string | URL | Request, init?: RequestInit) => Promise<Response>;
  writeFile?: (path: string, data: string) => Promise<number | void>;
}

interface TrackedReleaseAsset {
  githubReleaseUrl: string;
  githubUrl: string;
}

type CliArguments = SyncOptions;

const RELEASE_TAG_PATTERN = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z.-]+))?$/;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isExactReleaseTag(tag: string): boolean {
  const match = tag.match(RELEASE_TAG_PATTERN);
  if (!match) {
    return false;
  }

  const prerelease = match[4];
  if (!prerelease) {
    return true;
  }

  return prerelease.split(".").every((identifier) => {
    if (!/^[0-9A-Za-z](?:[0-9A-Za-z-]*[0-9A-Za-z])?$/.test(identifier)) {
      return false;
    }
    return !/^\d+$/.test(identifier) || /^(0|[1-9]\d*)$/.test(identifier);
  });
}

function normalizeOrigin(value: string, name: string): string {
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    throw new Error(`Invalid ${name}: ${value}`);
  }

  if ((url.protocol !== "https:" && url.protocol !== "http:") || url.pathname !== "/" || url.search || url.hash) {
    throw new Error(`Invalid ${name}: ${value}`);
  }

  return url.origin;
}

function endpoint(origin: string, pathname: string): string {
  return new URL(pathname, `${origin}/`).toString();
}

function requiredTrackedAssets(manifest: unknown): Array<[string, TrackedReleaseAsset]> {
  if (!isRecord(manifest) || !isRecord(manifest.assets)) {
    throw new Error("Manifest assets must be an object");
  }

  return Object.entries(manifest.assets).flatMap(([assetKey, value]) => {
    if (!isRecord(value) || typeof value.githubReleaseUrl !== "string") {
      return [];
    }
    if (typeof value.githubUrl !== "string" || value.githubUrl.length === 0) {
      throw new Error(`Missing githubUrl for asset: ${assetKey}`);
    }
    return [[assetKey, { githubReleaseUrl: value.githubReleaseUrl, githubUrl: value.githubUrl }]];
  });
}

function validateGitHubAssetUrl(value: string, tag: string, assetKey: string): void {
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    throw new Error(`githubUrl for ${assetKey} is not an exact-tag MCPMate GitHub asset URL`);
  }

  const expectedPrefix = `/loocor/mcpmate/releases/download/${encodeURIComponent(tag)}/`;
  const assetName = url.pathname.slice(expectedPrefix.length);
  if (
    url.origin !== "https://github.com" ||
    !url.pathname.startsWith(expectedPrefix) ||
    assetName.length === 0 ||
    assetName.includes("/") ||
    url.search ||
    url.hash
  ) {
    throw new Error(`githubUrl for ${assetKey} is not an exact-tag MCPMate GitHub asset URL`);
  }
}

function validateTrackedDownloadUrl(value: string, publicOrigin: string, tag: string, assetKey: string): void {
  let url: URL;
  try {
    url = new URL(value);
  } catch {
    throw new Error(`githubReleaseUrl for ${assetKey} must use the configured public origin and exact asset path`);
  }

  const expectedPath = `/downloads/releases/${encodeURIComponent(tag)}/${encodeURIComponent(assetKey)}`;
  if (
    url.origin !== publicOrigin ||
    url.username ||
    url.password ||
    url.pathname !== expectedPath ||
    url.search ||
    url.hash
  ) {
    throw new Error(`githubReleaseUrl for ${assetKey} must use the configured public origin and exact asset path`);
  }
}

async function parseJsonResponse(response: Response, description: string): Promise<unknown> {
  const text = await response.text();
  try {
    return JSON.parse(text);
  } catch {
    throw new Error(`${description} returned invalid JSON`);
  }
}

export async function synchronizeReleaseDownloads(
  options: SyncOptions,
  dependencies: SyncDependencies = {},
): Promise<void> {
  if (!isExactReleaseTag(options.tag)) {
    throw new Error(`Invalid expected release tag: ${options.tag}`);
  }
  if (!options.outputPath) {
    throw new Error("Output path is required");
  }

  const authOrigin = normalizeOrigin(options.authOrigin, "auth origin");
  const publicOrigin = normalizeOrigin(options.publicOrigin, "public origin");
  const environment = dependencies.env ?? process.env;
  const workflowToken = environment.DOWNLOADS_WORKFLOW_TOKEN;
  if (!workflowToken) {
    throw new Error("Missing DOWNLOADS_WORKFLOW_TOKEN");
  }

  const fetcher = dependencies.fetch ?? fetch;
  const encodedTag = encodeURIComponent(options.tag);
  const refreshUrl = endpoint(authOrigin, `/internal/downloads/releases/${encodedTag}/refresh`);
  const refreshResponse = await fetcher(refreshUrl, {
    method: "POST",
    headers: { authorization: `Bearer ${workflowToken}` },
  });
  if (!refreshResponse.ok) {
    throw new Error(`Exact-tag refresh failed with HTTP ${refreshResponse.status}`);
  }

  const manifestUrl = endpoint(publicOrigin, `/downloads/releases/${encodedTag}`);
  const manifestResponse = await fetcher(manifestUrl);
  if (!manifestResponse.ok) {
    throw new Error(`Public exact-tag manifest request failed with HTTP ${manifestResponse.status}`);
  }

  const manifestInput = await parseJsonResponse(manifestResponse, "Public exact-tag manifest request");
  validateReleaseManifest(manifestInput, options.tag);

  for (const [assetKey, asset] of requiredTrackedAssets(manifestInput)) {
    validateTrackedDownloadUrl(asset.githubReleaseUrl, publicOrigin, options.tag, assetKey);
    validateGitHubAssetUrl(asset.githubUrl, options.tag, assetKey);
    const redirectResponse = await fetcher(asset.githubReleaseUrl, { redirect: "manual" });
    if (redirectResponse.status !== 302) {
      throw new Error(`Tracked download for ${assetKey} must return HTTP 302; received ${redirectResponse.status}`);
    }

    const location = redirectResponse.headers.get("location");
    if (!location) {
      throw new Error(`Tracked download for ${assetKey} is missing the Location header`);
    }
    if (location !== asset.githubUrl) {
      throw new Error(`Tracked download Location for ${assetKey} does not match manifest githubUrl`);
    }
  }

  const writeFile = dependencies.writeFile ?? Bun.write;
  await writeFile(options.outputPath, `${JSON.stringify(manifestInput, null, 2)}\n`);
}

function parseCliArguments(argumentsList: string[]): CliArguments {
  const values = new Map<string, string>();
  const supportedArguments = new Set(["--tag", "--auth-origin", "--public-origin", "--output"]);

  for (let index = 0; index < argumentsList.length; index += 2) {
    const name = argumentsList[index];
    const value = argumentsList[index + 1];
    if (!supportedArguments.has(name)) {
      throw new Error(`Unknown argument: ${name}`);
    }
    if (values.has(name)) {
      throw new Error(`Duplicate argument: ${name}`);
    }
    if (value === undefined || value.startsWith("--")) {
      throw new Error(`Missing value for argument: ${name}`);
    }
    values.set(name, value);
  }

  for (const name of supportedArguments) {
    if (!values.has(name)) {
      throw new Error(`Missing required argument: ${name}`);
    }
  }

  return {
    tag: values.get("--tag")!,
    authOrigin: values.get("--auth-origin")!,
    publicOrigin: values.get("--public-origin")!,
    outputPath: values.get("--output")!,
  };
}

export async function runCli(argumentsList: string[], dependencies: SyncDependencies = {}): Promise<void> {
  await synchronizeReleaseDownloads(parseCliArguments(argumentsList), dependencies);
}

if (import.meta.main) {
  try {
    await runCli(process.argv.slice(2));
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(message);
    process.exit(1);
  }
}
