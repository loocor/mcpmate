export type ReleaseChannel = "stable" | "beta";

export interface ReleaseTagClassification {
  tag: string;
  version: string;
  releaseChannel: ReleaseChannel;
  appVersion: string;
}

const STABLE_TAG_PATTERN = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const BETA_TAG_PATTERN = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)-beta(?:\.(0|[1-9]\d*))?$/;
const CONFIGURED_VERSION_PATTERN =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/;

export function classifyReleaseTag(tag: string): ReleaseTagClassification {
  const stableMatch = tag.match(STABLE_TAG_PATTERN);
  const betaMatch = tag.match(BETA_TAG_PATTERN);
  const match = stableMatch ?? betaMatch;
  if (!match) {
    throw new Error(`Unsupported release tag: ${tag}`);
  }

  const appVersion = `${match[1]}.${match[2]}.${match[3]}`;
  return {
    tag,
    version: tag.slice(1),
    releaseChannel: stableMatch ? "stable" : "beta",
    appVersion,
  };
}

export function appVersionFromConfiguredVersion(version: string): string {
  const match = version.match(CONFIGURED_VERSION_PATTERN);
  const prereleaseIdentifiers = match?.[4]?.split(".") ?? [];
  const hasInvalidNumericIdentifier = prereleaseIdentifiers.some(
    (identifier) => /^\d+$/.test(identifier) && !/^(0|[1-9]\d*)$/.test(identifier),
  );
  if (!match || hasInvalidNumericIdentifier) {
    throw new Error(`Unsupported configured app version: ${version}`);
  }
  return `${match[1]}.${match[2]}.${match[3]}`;
}

function parseCliArguments(argumentsList: string[]): { mode: "tag" | "configured-version"; value: string } {
  if (argumentsList.length !== 2 || !argumentsList[1]) {
    throw new Error("Usage: release-tag.ts (--tag <release-tag> | --configured-version <version>)");
  }
  if (argumentsList[0] === "--tag") {
    return { mode: "tag", value: argumentsList[1] };
  }
  if (argumentsList[0] === "--configured-version") {
    return { mode: "configured-version", value: argumentsList[1] };
  }
  throw new Error("Usage: release-tag.ts (--tag <release-tag> | --configured-version <version>)");
}

if (import.meta.main) {
  try {
    const options = parseCliArguments(process.argv.slice(2));
    const output =
      options.mode === "tag"
        ? JSON.stringify(classifyReleaseTag(options.value))
        : appVersionFromConfiguredVersion(options.value);
    process.stdout.write(`${output}\n`);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(message);
    process.exit(1);
  }
}
