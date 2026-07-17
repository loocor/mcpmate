import { afterAll, describe, expect, test } from "bun:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { renderReleaseNotes } from "./render-release-downloads";

const fixtureUrl = new URL("./fixtures/release-manifest-v2.json", import.meta.url);
const rendererPath = fileURLToPath(new URL("./render-release-downloads.ts", import.meta.url));
const temporaryDirectories: string[] = [];
const generatedNotes = [
  "## What's Changed",
  "",
  "* Add Homebrew distribution by @loocor in #123",
  "",
  "**Full Changelog**: https://github.com/loocor/mcpmate/compare/v0.3.3...v0.3.4-beta",
  "",
].join("\n");

async function manifestFixture(): Promise<Record<string, unknown>> {
  return structuredClone(await Bun.file(fixtureUrl).json());
}

function countDownloadsSections(notes: string): number {
  return notes.match(/^## Downloads$/gm)?.length ?? 0;
}

async function createCliFiles(manifestContent?: string): Promise<{
  directory: string;
  manifestPath: string;
  notesPath: string;
  outputPath: string;
}> {
  const directory = await mkdtemp(join(tmpdir(), "mcpmate-release-renderer-test-"));
  const manifestPath = join(directory, "manifest.json");
  const notesPath = join(directory, "notes.md");
  const outputPath = join(directory, "output.md");
  temporaryDirectories.push(directory);
  await writeFile(manifestPath, manifestContent ?? (await Bun.file(fixtureUrl).text()));
  await writeFile(notesPath, generatedNotes);
  return { directory, manifestPath, notesPath, outputPath };
}

async function runCli(argumentsList: string[]): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const subprocess = Bun.spawn([process.execPath, rendererPath, ...argumentsList], {
    stdout: "pipe",
    stderr: "pipe",
  });
  const [exitCode, stdout, stderr] = await Promise.all([
    subprocess.exited,
    new Response(subprocess.stdout).text(),
    new Response(subprocess.stderr).text(),
  ]);
  return { exitCode, stdout, stderr };
}

afterAll(async () => {
  await Promise.all(temporaryDirectories.map((directory) => rm(directory, { recursive: true })));
});

describe("renderReleaseNotes", () => {
  test("inserts Downloads after generated notes and before Full Changelog", async () => {
    const rendered = renderReleaseNotes(generatedNotes, await manifestFixture(), "v0.3.4-beta");

    expect(rendered.indexOf("* Add Homebrew distribution")).toBeLessThan(rendered.indexOf("## Downloads"));
    expect(rendered.indexOf("## Downloads")).toBeLessThan(rendered.indexOf("**Full Changelog**"));
    expect(rendered).toContain("macOS (Apple silicon, DMG)");
    expect(rendered).toContain("Windows (x64, MSI)");
    expect(rendered).toContain("Linux (ARM64, AppImage)");
  });

  test("appends Downloads to fallback notes without a changelog marker", async () => {
    const fallback = "## What's Changed\n\n- Release notes fallback generated from tag comparison.\n";
    const rendered = renderReleaseNotes(fallback, await manifestFixture(), "v0.3.4-beta");

    expect(rendered).toStartWith(fallback.trimEnd());
    expect(rendered).toEndWith(
      "- Linux (x64, DEB): [Download](https://public.mcp.umate.ai/downloads/releases/v0.3.4-beta/linux-x64-deb)\n",
    );
  });

  test("replaces an existing Downloads section in place", async () => {
    const notes = [
      "## What's Changed",
      "",
      "- Existing change",
      "",
      "## Downloads",
      "",
      "- Old download: https://downloads.example.invalid/old",
      "",
      "**Full Changelog**: https://github.com/loocor/mcpmate/compare/old...new",
      "",
    ].join("\n");
    const rendered = renderReleaseNotes(notes, await manifestFixture(), "v0.3.4-beta");

    expect(countDownloadsSections(rendered)).toBe(1);
    expect(rendered).not.toContain("downloads.example.invalid");
    expect(rendered.indexOf("## Downloads")).toBeLessThan(rendered.indexOf("**Full Changelog**"));
  });

  test("removes every old Downloads range when other level-two sections separate duplicates", async () => {
    const notes = [
      "## What's Changed",
      "",
      "- Existing change",
      "",
      "## Downloads",
      "",
      "- Old first download: https://downloads.example.invalid/first",
      "",
      "## Security",
      "",
      "- Security note that must remain",
      "",
      "## Downloads",
      "",
      "- Old second download: https://downloads.example.invalid/second",
      "",
      "## Contributors",
      "",
      "- Contributor note that must remain",
      "",
      "**Full Changelog**: https://github.com/loocor/mcpmate/compare/old...new",
      "",
    ].join("\n");
    const rendered = renderReleaseNotes(notes, await manifestFixture(), "v0.3.4-beta");

    expect(countDownloadsSections(rendered)).toBe(1);
    expect(rendered).not.toContain("downloads.example.invalid");
    expect(rendered).toContain("## Security\n\n- Security note that must remain");
    expect(rendered).toContain("## Contributors\n\n- Contributor note that must remain");
    expect(rendered.indexOf("## Contributors")).toBeLessThan(rendered.indexOf("## Downloads"));
    expect(rendered.indexOf("## Downloads")).toBeLessThan(rendered.indexOf("**Full Changelog**"));
  });

  test("fails when a required asset is missing", async () => {
    const manifest = await manifestFixture();
    delete (manifest.assets as Record<string, unknown>)["linux-x64-deb"];

    expect(() => renderReleaseNotes(generatedNotes, manifest, "v0.3.4-beta")).toThrow(
      "Missing required asset: linux-x64-deb",
    );
  });

  test("fails when the manifest tag does not match the requested release", async () => {
    const manifest = await manifestFixture();

    expect(() => renderReleaseNotes(generatedNotes, manifest, "v0.3.5-beta")).toThrow(
      "Manifest tag v0.3.4-beta does not match expected tag v0.3.5-beta",
    );
  });

  test("rejects the same invalid prerelease forms as the Admin exact-tag contract", async () => {
    const manifest = await manifestFixture();

    for (const tag of ["v1.2.3-01", "v1.2.3-", "v1.2.3-alpha.", "v1.2.3-alpha-"]) {
      expect(() => renderReleaseNotes(generatedNotes, manifest, tag)).toThrow(`Invalid expected release tag: ${tag}`);
    }
  });

  test("accepts internal prerelease hyphens allowed by the Admin exact-tag contract", async () => {
    const manifest = await manifestFixture();
    manifest.tag = "v1.2.3-alpha--preview.1";
    manifest.version = "1.2.3-alpha--preview.1";
    const assets = manifest.assets as Record<string, Record<string, unknown>>;
    for (const [assetKey, asset] of Object.entries(assets)) {
      if (typeof asset.githubReleaseUrl === "string") {
        asset.githubReleaseUrl = `https://public.mcp.umate.ai/downloads/releases/v1.2.3-alpha--preview.1/${assetKey}`;
      }
    }

    expect(renderReleaseNotes(generatedNotes, manifest, "v1.2.3-alpha--preview.1")).toContain(
      "/downloads/releases/v1.2.3-alpha--preview.1/macos-arm64-dmg",
    );
  });

  test("fails when a required asset digest is invalid", async () => {
    const manifest = await manifestFixture();
    const assets = manifest.assets as Record<string, Record<string, unknown>>;
    assets["macos-arm64-dmg"].sha256 = "sha256:not-a-digest";

    expect(() => renderReleaseNotes(generatedNotes, manifest, "v0.3.4-beta")).toThrow(
      "Invalid SHA-256 for asset: macos-arm64-dmg",
    );
  });

  test("fails when githubReleaseUrl is a raw GitHub asset URL", async () => {
    const manifest = await manifestFixture();
    const assets = manifest.assets as Record<string, Record<string, unknown>>;
    assets["macos-arm64-dmg"].githubReleaseUrl = assets["macos-arm64-dmg"].githubUrl;

    expect(() => renderReleaseNotes(generatedNotes, manifest, "v0.3.4-beta")).toThrow(
      "Invalid githubReleaseUrl for asset: macos-arm64-dmg",
    );
  });

  test("fails when the manifest schema is not version 2", async () => {
    const manifest = await manifestFixture();
    manifest.schemaVersion = 1;

    expect(() => renderReleaseNotes(generatedNotes, manifest, "v0.3.4-beta")).toThrow(
      "Expected manifest schemaVersion 2",
    );
  });

  test("preserves a beta version in every tracked URL", async () => {
    const rendered = renderReleaseNotes(generatedNotes, await manifestFixture(), "v0.3.4-beta");

    expect(rendered.match(/v0\.3\.4-beta/g)?.length).toBe(9);
    expect(rendered).not.toContain("v0.3.4/");
  });

  test("is byte-identical and contains exactly one Downloads section when repeated", async () => {
    const manifest = await manifestFixture();
    const once = renderReleaseNotes(generatedNotes, manifest, "v0.3.4-beta");
    const twice = renderReleaseNotes(once, manifest, "v0.3.4-beta");

    expect(twice).toBe(once);
    expect(countDownloadsSections(twice)).toBe(1);
  });

  test("never renders raw GitHub asset URLs or support assets", async () => {
    const rendered = renderReleaseNotes(generatedNotes, await manifestFixture(), "v0.3.4-beta");

    expect(rendered).not.toContain("github.com/loocor/mcpmate/releases/download/");
    expect(rendered).not.toContain("signatures.zip");
    expect(rendered).not.toContain("update.json");
    expect(rendered).not.toContain("app.tar.gz");
  });
});

describe("release downloads CLI errors", () => {
  test("returns a non-zero exit for an unknown argument", async () => {
    const files = await createCliFiles();
    const result = await runCli([
      "--manifest",
      files.manifestPath,
      "--notes",
      files.notesPath,
      "--tag",
      "v0.3.4-beta",
      "--output",
      files.outputPath,
      "--unknown",
      "value",
    ]);

    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("Unknown argument: --unknown");
  });

  test("returns a non-zero exit for a duplicate argument", async () => {
    const files = await createCliFiles();
    const result = await runCli([
      "--manifest",
      files.manifestPath,
      "--manifest",
      files.manifestPath,
      "--notes",
      files.notesPath,
      "--tag",
      "v0.3.4-beta",
      "--output",
      files.outputPath,
    ]);

    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("Duplicate argument: --manifest");
  });

  test("returns a non-zero exit when an argument value is missing", async () => {
    const files = await createCliFiles();
    const result = await runCli([
      "--manifest",
      files.manifestPath,
      "--notes",
      files.notesPath,
      "--tag",
      "v0.3.4-beta",
      "--output",
    ]);

    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("Missing value for argument: --output");
  });

  test("returns a non-zero exit for invalid manifest JSON", async () => {
    const files = await createCliFiles("{not valid JSON");
    const result = await runCli([
      "--manifest",
      files.manifestPath,
      "--notes",
      files.notesPath,
      "--tag",
      "v0.3.4-beta",
      "--output",
      files.outputPath,
    ]);

    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain(`Manifest is not valid JSON: ${files.manifestPath}`);
  });

  test("returns a non-zero exit when a required input file is missing", async () => {
    const files = await createCliFiles();
    const missingManifestPath = join(files.directory, "missing-manifest.json");
    const result = await runCli([
      "--manifest",
      missingManifestPath,
      "--notes",
      files.notesPath,
      "--tag",
      "v0.3.4-beta",
      "--output",
      files.outputPath,
    ]);

    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain(missingManifestPath);
  });
});
