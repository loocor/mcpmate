import { describe, expect, it, vi } from "vitest";
import {
	buildSecretInsertNextValue,
	resolveSecretFieldHeaderKey,
} from "./use-secret-field-insert";
import type { ManualServerFormValues } from "../types";

describe("use-secret-field-insert", () => {
	it("resolves header keys from headers, env, and urlParams value fields", () => {
		const getValues = vi.fn((path: keyof ManualServerFormValues) => {
			if (path === "env.2.key") return "Authorization";
			if (path === "headers.0.key") return "X-Api-Key";
			if (path === "urlParams.1.key") return "token";
			return undefined;
		});

		expect(resolveSecretFieldHeaderKey("env.2.value", getValues)).toBe(
			"Authorization",
		);
		expect(resolveSecretFieldHeaderKey("headers.0.value", getValues)).toBe(
			"X-Api-Key",
		);
		expect(resolveSecretFieldHeaderKey("urlParams.1.value", getValues)).toBe(
			"token",
		);
		expect(resolveSecretFieldHeaderKey("command", getValues)).toBeNull();
	});

	it("appends bearer prefix for empty authorization env on inline create", () => {
		const getValues = vi.fn((path: keyof ManualServerFormValues) => {
			if (path === "env.0.key") return "Authorization";
			if (path === "env.0.value") return "";
			return undefined;
		});

		expect(
			buildSecretInsertNextValue(
				"env.0.value",
				"[[secret:token]]",
				getValues,
			),
		).toBe("Bearer [[secret:token]]");
	});
});
