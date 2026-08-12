import { expect, test } from "bun:test";

import {
	exactDownloadsReleaseAssetUrl,
	releaseFromDownloadManifest,
	type PublicDownloadManifestV2,
} from "./githubRelease";

const tag = "v1.2.3-beta";

function platformForKey(key: string): "macos" | "windows" | "linux" {
	if (key.startsWith("macos")) {
		return "macos";
	}
	if (key.startsWith("windows")) {
		return "windows";
	}
	return "linux";
}

function formatForKey(key: string): "dmg" | "msi" | "deb" | "appimage" {
	if (key.endsWith("dmg")) {
		return "dmg";
	}
	if (key.endsWith("msi")) {
		return "msi";
	}
	if (key.endsWith("deb")) {
		return "deb";
	}
	return "appimage";
}

function installerAsset(key: string, name: string) {
	return {
		key,
		platform: platformForKey(key),
		arch: key.includes("arm64") ? "arm64" : "x64",
		format: formatForKey(key),
		name,
		githubUrl: `https://github.com/loocor/mcpmate/releases/download/${tag}/${name}`,
		githubReleaseUrl: exactDownloadsReleaseAssetUrl(tag, key),
		sha256: "a".repeat(64),
		size: 1,
	};
}

function manifestWithSupportAssets(): PublicDownloadManifestV2 {
	return {
		schemaVersion: 2,
		tag,
		version: "1.2.3-beta",
		releaseChannel: "beta",
		releaseUrl: `https://github.com/loocor/mcpmate/releases/tag/${tag}`,
		assets: {
			"macos-arm64-dmg": installerAsset("macos-arm64-dmg", "MCPMate_1.2.3_macos_aarch64.dmg"),
			"macos-x64-dmg": installerAsset("macos-x64-dmg", "MCPMate_1.2.3_macos_x86_64.dmg"),
			"windows-arm64-msi": installerAsset("windows-arm64-msi", "MCPMate_1.2.3_windows_arm64.msi"),
			"windows-x64-msi": installerAsset("windows-x64-msi", "MCPMate_1.2.3_windows_x64.msi"),
			"linux-arm64-appimage": installerAsset("linux-arm64-appimage", "MCPMate_1.2.3_linux_arm64.AppImage"),
			"linux-x64-appimage": installerAsset("linux-x64-appimage", "MCPMate_1.2.3_linux_x64.AppImage"),
			"linux-arm64-deb": installerAsset("linux-arm64-deb", "MCPMate_1.2.3_linux_arm64.deb"),
			"linux-x64-deb": installerAsset("linux-x64-deb", "MCPMate_1.2.3_linux_x64.deb"),
			signatures: {
				name: "signatures.zip",
				githubUrl: `https://github.com/loocor/mcpmate/releases/download/${tag}/signatures.zip`,
			},
		},
	};
}

test("keeps installer download routes when v2 includes support assets", () => {
	const release = releaseFromDownloadManifest(manifestWithSupportAssets());

	expect(release).not.toBeNull();
	expect(release?.assets).toHaveLength(6);
	expect(release?.assets).toContainEqual({
		name: "MCPMate_1.2.3_macos_aarch64.dmg",
		browser_download_url: exactDownloadsReleaseAssetUrl(tag, "macos-arm64-dmg"),
		download_count: undefined,
	});
});

test("rejects an installer without its deterministic public route", () => {
	const manifest = manifestWithSupportAssets();
	manifest.assets["windows-x64-msi"].githubReleaseUrl = "https://downloads.example.test/windows-x64-msi";

	expect(releaseFromDownloadManifest(manifest)).toBeNull();
});

test("rejects an installer whose key does not match its display row", () => {
	const manifest = manifestWithSupportAssets();
	manifest.assets["windows-x64-msi"].key = "windows-arm64-msi";

	expect(releaseFromDownloadManifest(manifest)).toBeNull();
});

test("rejects an installer with an invalid provided download count", () => {
	const manifest = manifestWithSupportAssets();
	(manifest.assets["windows-x64-msi"] as { githubDownloadCount?: unknown }).githubDownloadCount = -1;

	expect(releaseFromDownloadManifest(manifest)).toBeNull();
});

test("rejects a manifest with non-string release metadata", () => {
	const manifest = manifestWithSupportAssets();
	(manifest as { releaseUrl: unknown }).releaseUrl = 1;

	expect(releaseFromDownloadManifest(manifest)).toBeNull();
});
