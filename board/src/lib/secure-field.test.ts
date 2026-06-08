import { describe, expect, it } from "vitest";
import {
	buildBearerSecretValue,
	classifyFieldValue,
	extractWholeSecretAlias,
	isRedactedMask,
	parseBearerSecretValue,
	resolveRecordUpdatePayload,
	resolveSecureFieldVariant,
	sanitizeRecordForSave,
	sanitizeStringForSave,
} from "./secure-field";

describe("secure-field", () => {
	it("detects full and partial redacted masks", () => {
		expect(isRedactedMask("***REDACTED***")).toBe(true);
		expect(isRedactedMask("Bearer***ue")).toBe(true);
		expect(isRedactedMask("abcdef***xy")).toBe(true);
		expect(isRedactedMask("Bearer [[secret:token]]")).toBe(false);
		expect(isRedactedMask("plain-value")).toBe(false);
	});

	it("classifies secret placeholders", () => {
		expect(classifyFieldValue("[[secret:github-token]]")).toBe("secret_ref");
		expect(classifyFieldValue("Bearer [[secret:token]]")).toBe("secret_ref");
		expect(extractWholeSecretAlias("[[secret:github-token]]")).toBe(
			"github-token",
		);
	});

	it("parses bearer secret structures", () => {
		expect(parseBearerSecretValue("Bearer [[secret:token]]")).toEqual({
			prefix: "Bearer ",
			secretAlias: "token",
			redacted: false,
		});
		expect(parseBearerSecretValue("Bearer ***REDACTED***")).toEqual({
			prefix: "Bearer ",
			secretAlias: null,
			redacted: true,
		});
	});

	it("resolves secure field variants", () => {
		expect(resolveSecureFieldVariant("[[secret:token]]")).toBe("whole-secret");
		expect(resolveSecureFieldVariant("Bearer [[secret:token]]")).toBe(
			"bearer-secret",
		);
		expect(resolveSecureFieldVariant("Bearer***ue")).toBe("bearer-redacted");
		expect(resolveSecureFieldVariant("***REDACTED***")).toBe("redacted");
	});

	it("strips redacted values on save", () => {
		expect(sanitizeStringForSave("***REDACTED***")).toBeUndefined();
		expect(sanitizeStringForSave("Bearer***ue")).toBeUndefined();
		expect(sanitizeStringForSave("[[secret:token]]")).toBe("[[secret:token]]");
		expect(
			sanitizeRecordForSave({
				Authorization: "Bearer***ue",
				"X-Custom": "visible",
			}),
		).toEqual({ "X-Custom": "visible" });
	});

	it("builds bearer secret values from picker placeholders", () => {
		expect(buildBearerSecretValue("[[secret:token]]")).toBe(
			"Bearer [[secret:token]]",
		);
	});

	it("resolves record update payloads for env and header clears", () => {
		const baseline = { API_KEY: "secret-value" };

		expect(resolveRecordUpdatePayload(undefined, baseline)).toEqual({});
		expect(resolveRecordUpdatePayload(baseline, baseline)).toBeUndefined();
		expect(resolveRecordUpdatePayload({ API_KEY: "new" }, baseline)).toEqual({
			API_KEY: "new",
		});
		expect(resolveRecordUpdatePayload({ API_KEY: "new" }, undefined)).toEqual({
			API_KEY: "new",
		});
		expect(resolveRecordUpdatePayload(undefined, undefined)).toBeUndefined();
	});
});
