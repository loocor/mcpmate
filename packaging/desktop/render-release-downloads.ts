import { classifyReleaseTag, type ReleaseChannel } from "./release-tag";

const ASSET_SPECS = [
  {
    key: "macos-arm64-dmg",
    label: "macOS (Apple silicon, DMG)",
    platform: "macos",
    arch: "arm64",
    format: "dmg",
    nameSuffix: "macos_aarch64.dmg",
  },
  {
    key: "macos-x64-dmg",
    label: "macOS (Intel, DMG)",
    platform: "macos",
    arch: "x64",
    format: "dmg",
    nameSuffix: "macos_x86_64.dmg",
  },
  {
    key: "windows-arm64-msi",
    label: "Windows (ARM64, MSI)",
    platform: "windows",
    arch: "arm64",
    format: "msi",
    nameSuffix: "windows_arm64.msi",
  },
  {
    key: "windows-x64-msi",
    label: "Windows (x64, MSI)",
    platform: "windows",
    arch: "x64",
    format: "msi",
    nameSuffix: "windows_x64.msi",
  },
  {
    key: "linux-arm64-appimage",
    label: "Linux (ARM64, AppImage)",
    platform: "linux",
    arch: "arm64",
    format: "appimage",
    nameSuffix: "linux_arm64.AppImage",
  },
  {
    key: "linux-x64-appimage",
    label: "Linux (x64, AppImage)",
    platform: "linux",
    arch: "x64",
    format: "appimage",
    nameSuffix: "linux_x64.AppImage",
  },
  {
    key: "linux-arm64-deb",
    label: "Linux (ARM64, DEB)",
    platform: "linux",
    arch: "arm64",
    format: "deb",
    nameSuffix: "linux_arm64.deb",
  },
  {
    key: "linux-x64-deb",
    label: "Linux (x64, DEB)",
    platform: "linux",
    arch: "x64",
    format: "deb",
    nameSuffix: "linux_x64.deb",
  },
] as const;

const SHA256_PATTERN = /^[a-f0-9]{64}$/;
const PUBLIC_RELEASE_ORIGIN = "https://public.mcp.umate.ai";

interface ReleaseAsset {
  key: string;
  platform: string;
  arch: string;
  format: string;
  name: string;
  githubUrl: string;
  githubReleaseUrl: string;
  homebrewUrl: string;
  sha256: string;
}

interface ReleaseManifestV2 {
  schemaVersion: 2;
  tag: string;
  version: string;
  releaseChannel: ReleaseChannel;
  assets: Record<string, ReleaseAsset>;
}

interface CliArguments {
  manifestPath: string;
  notesPath: string;
  outputPath: string;
  tag: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function validatePublicAssetUrl(
  value: unknown,
  field: "githubReleaseUrl" | "homebrewUrl",
  route: "releases" | "homebrew",
  expectedOrigin: string,
  tag: string,
  assetKey: string,
): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`Invalid ${field} for asset: ${assetKey}`);
  }

  let url: URL;
  try {
    url = new URL(value);
  } catch {
    throw new Error(`Invalid ${field} for asset: ${assetKey}`);
  }

  const expectedPath = `/downloads/${route}/${tag}/${assetKey}`;
  if (
    url.origin !== expectedOrigin ||
    url.username ||
    url.password ||
    url.pathname !== expectedPath ||
    url.search ||
    url.hash
  ) {
    throw new Error(`Invalid ${field} for asset: ${assetKey}`);
  }

  return value;
}

function validateGitHubUrl(value: unknown, tag: string, assetKey: string, assetName: string): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`Manifest githubUrl is invalid: ${assetKey}`);
  }

  let url: URL;
  try {
    url = new URL(value);
  } catch {
    throw new Error(`Manifest githubUrl is invalid: ${assetKey}`);
  }

  const expectedPath = `/loocor/mcpmate/releases/download/${tag}/${assetName}`;
  if (
    url.origin !== "https://github.com" ||
    url.username ||
    url.password ||
    url.pathname !== expectedPath ||
    url.search ||
    url.hash
  ) {
    throw new Error(`Manifest githubUrl is invalid: ${assetKey}`);
  }

  return value;
}

export function validateReleaseManifest(
  manifest: unknown,
  expectedTag: string,
  expectedPublicOrigin: string,
): ReleaseManifestV2 {
  let expectedRelease;
  try {
    expectedRelease = classifyReleaseTag(expectedTag);
  } catch {
    throw new Error(`Invalid expected release tag: ${expectedTag}`);
  }
  if (!isRecord(manifest)) {
    throw new Error("Release manifest must be an object");
  }
  if (manifest.schemaVersion !== 2) {
    throw new Error("Expected manifest schemaVersion 2");
  }
  if (typeof manifest.tag !== "string") {
    throw new Error("Manifest tag is invalid");
  }
  let manifestRelease;
  try {
    manifestRelease = classifyReleaseTag(manifest.tag);
  } catch {
    throw new Error("Manifest tag is invalid");
  }
  if (manifest.tag !== expectedTag) {
    throw new Error(`Manifest tag ${manifest.tag} does not match expected tag ${expectedTag}`);
  }
  if (manifest.version !== manifestRelease.version) {
    throw new Error(`Manifest version does not match tag: ${manifest.tag}`);
  }
  if (manifest.releaseChannel !== expectedRelease.releaseChannel) {
    throw new Error("Manifest releaseChannel does not match tag");
  }
  if (!isRecord(manifest.assets)) {
    throw new Error("Manifest assets must be an object");
  }

  for (const spec of ASSET_SPECS) {
    const asset = manifest.assets[spec.key];
    if (!isRecord(asset)) {
      throw new Error(`Missing required asset: ${spec.key}`);
    }
    const expectedName = `MCPMate_${expectedRelease.appVersion}_${spec.nameSuffix}`;
    if (
      asset.key !== spec.key ||
      asset.platform !== spec.platform ||
      asset.arch !== spec.arch ||
      asset.format !== spec.format ||
      asset.name !== expectedName
    ) {
      throw new Error(`Manifest asset metadata is invalid: ${spec.key}`);
    }
    if (typeof asset.sha256 !== "string" || !SHA256_PATTERN.test(asset.sha256)) {
      throw new Error(`Invalid SHA-256 for asset: ${spec.key}`);
    }
    validatePublicAssetUrl(
      asset.githubReleaseUrl,
      "githubReleaseUrl",
      "releases",
      expectedPublicOrigin,
      manifest.tag,
      spec.key,
    );
    validatePublicAssetUrl(
      asset.homebrewUrl,
      "homebrewUrl",
      "homebrew",
      expectedPublicOrigin,
      manifest.tag,
      spec.key,
    );
    validateGitHubUrl(asset.githubUrl, manifest.tag, spec.key, expectedName);
  }

  return manifest as unknown as ReleaseManifestV2;
}

export function renderDownloadsSection(manifest: ReleaseManifestV2): string {
  const downloads = ASSET_SPECS.map((spec) => {
    const url = manifest.assets[spec.key].githubReleaseUrl;
    return `- ${spec.label}: [Download](${url})`;
  });

  return ["## Downloads", "", ...downloads].join("\n");
}

function findExistingDownloadsRanges(notes: string): Array<{ start: number; end: number }> {
  const sectionPattern = /^## (.+?)[ \t]*$/gm;
  const sections: Array<{ title: string; start: number }> = [];
  let section: RegExpExecArray | null;

  while ((section = sectionPattern.exec(notes)) !== null) {
    sections.push({ title: section[1].trim(), start: section.index });
  }

  return sections.flatMap((current, index) => {
    if (current.title !== "Downloads") {
      return [];
    }

    const nextSectionStart = sections[index + 1]?.start ?? notes.length;
    const remainder = notes.slice(current.start, nextSectionStart);
    const fullChangelog = /^\*\*Full Changelog\*\*.*$/m.exec(remainder);
    const end = fullChangelog?.index === undefined ? nextSectionStart : current.start + fullChangelog.index;
    return [{ start: current.start, end }];
  });
}

function joinMarkdownParts(parts: string[]): string {
  return `${parts.map((part) => part.trim()).filter(Boolean).join("\n\n")}\n`;
}

export function renderReleaseNotes(notes: string, manifestInput: unknown, expectedTag: string): string {
  const manifest = validateReleaseManifest(manifestInput, expectedTag, PUBLIC_RELEASE_ORIGIN);
  const downloads = renderDownloadsSection(manifest);
  const existingDownloads = findExistingDownloadsRanges(notes);

  if (existingDownloads.length === 1) {
    const [existing] = existingDownloads;
    return joinMarkdownParts([
      notes.slice(0, existing.start),
      downloads,
      notes.slice(existing.end),
    ]);
  }

  const notesWithoutDownloads = [...existingDownloads]
    .reverse()
    .reduce((currentNotes, range) => `${currentNotes.slice(0, range.start)}${currentNotes.slice(range.end)}`, notes);

  const fullChangelog = /^\*\*Full Changelog\*\*.*$/m.exec(notesWithoutDownloads);
  if (fullChangelog && fullChangelog.index !== undefined) {
    return joinMarkdownParts([
      notesWithoutDownloads.slice(0, fullChangelog.index),
      downloads,
      notesWithoutDownloads.slice(fullChangelog.index),
    ]);
  }

  return joinMarkdownParts([notesWithoutDownloads, downloads]);
}

function parseCliArguments(argumentsList: string[]): CliArguments {
  const values = new Map<string, string>();
  const supportedArguments = new Set(["--manifest", "--notes", "--output", "--tag"]);

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
    manifestPath: values.get("--manifest")!,
    notesPath: values.get("--notes")!,
    outputPath: values.get("--output")!,
    tag: values.get("--tag")!,
  };
}

export async function runCli(argumentsList: string[]): Promise<void> {
  const options = parseCliArguments(argumentsList);
  const manifestText = await Bun.file(options.manifestPath).text();
  const notes = await Bun.file(options.notesPath).text();

  let manifest: unknown;
  try {
    manifest = JSON.parse(manifestText);
  } catch {
    throw new Error(`Manifest is not valid JSON: ${options.manifestPath}`);
  }

  const rendered = renderReleaseNotes(notes, manifest, options.tag);
  await Bun.write(options.outputPath, rendered);
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
