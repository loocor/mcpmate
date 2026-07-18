import { describe, expect, test } from "bun:test";

import { appVersionFromConfiguredVersion, classifyReleaseTag } from "./release-tag";

describe("classifyReleaseTag", () => {
  test("classifies a stable release", () => {
    expect(classifyReleaseTag("v1.2.3")).toEqual({
      tag: "v1.2.3",
      version: "1.2.3",
      releaseChannel: "stable",
      appVersion: "1.2.3",
    });
  });

  test.each(["v1.2.3-beta", "v1.2.3-beta.7"])("classifies a supported beta release: %s", (tag) => {
    expect(classifyReleaseTag(tag)).toEqual({
      tag,
      version: tag.slice(1),
      releaseChannel: "beta",
      appVersion: "1.2.3",
    });
  });

  test.each([
    "v1.2.3-alpha",
    "v1.2.3-rc.1",
    "v1.2.3-dev",
    "v1.2.3-debug",
    "v1.2.3-nightly",
    "v1.2.3-preview.1",
    "v1.2.3-beta.preview",
    "v1.2.3-beta.01",
    "v1.2.3+build.1",
    "v1.2.3-beta+build.1",
    "v01.2.3",
    "v1.02.3",
    "v1.2.03",
    "1.2.3",
    "v1.2",
  ])("rejects an unsupported release tag: %s", (tag) => {
    expect(() => classifyReleaseTag(tag)).toThrow(`Unsupported release tag: ${tag}`);
  });
});

describe("appVersionFromConfiguredVersion", () => {
  test("maps the configured development prerelease to its numeric Tauri version", () => {
    expect(appVersionFromConfiguredVersion("0.0.0-dev")).toBe("0.0.0");
  });

  test.each(["1.2", "01.2.3-dev", "1.2.3+build.1", "not-a-version"])(
    "rejects an invalid configured app version: %s",
    (version) => {
      expect(() => appVersionFromConfiguredVersion(version)).toThrow(
        `Unsupported configured app version: ${version}`,
      );
    },
  );
});
