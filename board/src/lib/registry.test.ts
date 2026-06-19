import { describe, expect, test } from "bun:test";
import { matchesInstalledRegistryServer } from "./registry";
import type { RegistryServerEntry, ServerSource, ServerSummary } from "./types";

function registryEntry(name: string): RegistryServerEntry {
	return {
		name,
		version: "1.0.0",
	};
}

function installedServer(source: ServerSource | null): ServerSummary {
	return {
		id: "srv-1",
		name: "google-ads-server",
		status: "running",
		source: source ?? undefined,
	};
}

describe("registry server matching", () => {
	test("matches installed servers by registry source", () => {
		expect(
			matchesInstalledRegistryServer(
				registryEntry("google-ads"),
				installedServer({ type: "registry", ref: "google-ads" }),
			),
		).toBe(true);
	});

	test("does not treat non-registry sources as registry ids", () => {
		expect(
			matchesInstalledRegistryServer(
				registryEntry("google-ads"),
				installedServer({ type: "catalog", ref: "google-ads" }),
			),
		).toBe(false);
	});

	test("does not treat registry source without ref as match", () => {
		expect(
			matchesInstalledRegistryServer(
				registryEntry("google-ads"),
				installedServer({ type: "registry" }),
			),
		).toBe(false);
	});
});
