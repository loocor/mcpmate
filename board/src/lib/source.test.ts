import { describe, expect, test } from "bun:test";
import { isRegistrySource, registryRef } from "./source";

describe("source helpers", () => {
	test("isRegistrySource detects registry sources with a ref", () => {
		expect(isRegistrySource({ type: "registry", ref: "google-ads" })).toBe(true);
		expect(isRegistrySource({ type: "registry" })).toBe(false);
		expect(isRegistrySource({ type: "catalog", ref: "github" })).toBe(false);
		expect(isRegistrySource(undefined)).toBe(false);
		expect(isRegistrySource(null)).toBe(false);
	});

	test("registryRef extracts the ref from registry sources", () => {
		expect(registryRef({ type: "registry", ref: "google-ads" })).toBe("google-ads");
		expect(registryRef({ type: "registry" })).toBeNull();
		expect(registryRef({ type: "catalog", ref: "github" })).toBeNull();
		expect(registryRef(undefined)).toBeNull();
	});
});
