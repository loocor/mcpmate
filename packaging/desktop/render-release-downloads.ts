const ASSET_SPECS = [
  ["macos-arm64-dmg", "macOS (Apple silicon, DMG)"],
  ["macos-x64-dmg", "macOS (Intel, DMG)"],
  ["windows-arm64-msi", "Windows (ARM64, MSI)"],
  ["windows-x64-msi", "Windows (x64, MSI)"],
  ["linux-arm64-appimage", "Linux (ARM64, AppImage)"],
  ["linux-x64-appimage", "Linux (x64, AppImage)"],
  ["linux-arm64-deb", "Linux (ARM64, DEB)"],
  ["linux-x64-deb", "Linux (x64, DEB)"],
] as const;

const SHA256_PATTERN = /^[a-fA-F0-9]{64}$/;

interface ReleaseAsset {
  githubReleaseUrl: string;
  sha256: string;
}

interface ReleaseManifestV2 {
  schemaVersion: 2;
  tag: string;
  version: string;
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

function isExactReleaseTag(tag: string): boolean {
  const match = tag.match(/^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z.-]+))?$/);
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

function validateTrackedUrl(value: unknown, tag: string, assetKey: string): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`Missing githubReleaseUrl for asset: ${assetKey}`);
  }

  let url: URL;
  try {
    url = new URL(value);
  } catch {
    throw new Error(`Invalid githubReleaseUrl for asset: ${assetKey}`);
  }

  const expectedPath = `/downloads/releases/${tag}/${assetKey}`;
  if (url.protocol !== "https:" || url.pathname !== expectedPath || url.search || url.hash) {
    throw new Error(`Invalid githubReleaseUrl for asset: ${assetKey}`);
  }

  return value;
}

export function validateReleaseManifest(manifest: unknown, expectedTag: string): ReleaseManifestV2 {
  if (!isExactReleaseTag(expectedTag)) {
    throw new Error(`Invalid expected release tag: ${expectedTag}`);
  }
  if (!isRecord(manifest)) {
    throw new Error("Release manifest must be an object");
  }
  if (manifest.schemaVersion !== 2) {
    throw new Error("Expected manifest schemaVersion 2");
  }
  if (typeof manifest.tag !== "string" || !isExactReleaseTag(manifest.tag)) {
    throw new Error("Manifest tag is invalid");
  }
  if (manifest.tag !== expectedTag) {
    throw new Error(`Manifest tag ${manifest.tag} does not match expected tag ${expectedTag}`);
  }
  if (manifest.version !== manifest.tag.slice(1)) {
    throw new Error(`Manifest version does not match tag: ${manifest.tag}`);
  }
  if (!isRecord(manifest.assets)) {
    throw new Error("Manifest assets must be an object");
  }

  for (const [assetKey] of ASSET_SPECS) {
    const asset = manifest.assets[assetKey];
    if (!isRecord(asset)) {
      throw new Error(`Missing required asset: ${assetKey}`);
    }
    if (typeof asset.sha256 !== "string" || !SHA256_PATTERN.test(asset.sha256)) {
      throw new Error(`Invalid SHA-256 for asset: ${assetKey}`);
    }
    validateTrackedUrl(asset.githubReleaseUrl, manifest.tag, assetKey);
  }

  return manifest as unknown as ReleaseManifestV2;
}

export function renderDownloadsSection(manifest: ReleaseManifestV2): string {
  const downloads = ASSET_SPECS.map(([assetKey, label]) => {
    const url = manifest.assets[assetKey].githubReleaseUrl;
    return `- ${label}: [Download](${url})`;
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
  const manifest = validateReleaseManifest(manifestInput, expectedTag);
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
