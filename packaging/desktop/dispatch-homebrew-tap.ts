import { classifyReleaseTag } from "./release-tag";

interface DispatchOptions {
  tag: string;
  manifestUrl: string;
}

interface DispatchDependencies {
  env?: Record<string, string | undefined>;
  fetch?: (input: string | URL | Request, init?: RequestInit) => Promise<Response>;
}

const DISPATCH_URL = "https://api.github.com/repos/loocor/homebrew-tap/dispatches";
const MANIFEST_ORIGIN = "https://public.mcp.umate.ai";

export async function dispatchHomebrewTap(
  options: DispatchOptions,
  dependencies: DispatchDependencies = {},
): Promise<void> {
  const environment = dependencies.env ?? process.env;
  const token = environment.HOMEBREW_TAP_TOKEN;
  if (!token) {
    throw new Error("Missing HOMEBREW_TAP_TOKEN");
  }

  classifyReleaseTag(options.tag);
  const expectedManifestUrl = `${MANIFEST_ORIGIN}/downloads/releases/${options.tag}`;
  if (options.manifestUrl !== expectedManifestUrl) {
    throw new Error("Manifest URL must match the exact release tag");
  }

  const fetcher = dependencies.fetch ?? fetch;
  const response = await fetcher(DISPATCH_URL, {
    method: "POST",
    headers: {
      accept: "application/vnd.github+json",
      authorization: `Bearer ${token}`,
      "content-type": "application/json",
      "x-github-api-version": "2022-11-28",
    },
    body: JSON.stringify({
      event_type: "mcpmate_release",
      client_payload: {
        tag: options.tag,
        manifest_url: options.manifestUrl,
      },
    }),
  });
  if (!response.ok) {
    throw new Error(`Homebrew Tap dispatch failed with HTTP ${response.status}`);
  }
}

function parseCliArguments(argumentsList: string[]): DispatchOptions {
  const values = new Map<string, string>();
  const supportedArguments = new Set(["--tag", "--manifest-url"]);
  for (let index = 0; index < argumentsList.length; index += 2) {
    const name = argumentsList[index];
    const value = argumentsList[index + 1];
    if (!supportedArguments.has(name)) {
      throw new Error(`Unknown argument: ${name}`);
    }
    if (values.has(name)) {
      throw new Error(`Duplicate argument: ${name}`);
    }
    if (!value || value.startsWith("--")) {
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
    manifestUrl: values.get("--manifest-url")!,
  };
}

if (import.meta.main) {
  try {
    await dispatchHomebrewTap(parseCliArguments(process.argv.slice(2)));
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(message);
    process.exit(1);
  }
}
