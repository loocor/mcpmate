import { describe, expect, it } from "vitest";
import type { SecretOrigin } from "./types";
import {
	inferSecretDefaultsFromOrigin,
	inferSecretKindFromOrigin,
	inferSecretLabelFromOrigin,
	isUserCreatableSecretKind,
	kindOptionsForEditor,
	USER_CREATABLE_SECRET_KINDS,
} from "./secret-origin-hints";

describe("inferSecretKindFromOrigin", () => {
	it("maps url field to url_credential", () => {
		const origin: SecretOrigin = {
			server_name: "context7",
			field_group: "streamable_http",
			field_key: "url",
			field_path: "url",
		};
		expect(inferSecretKindFromOrigin(origin)).toBe("url_credential");
	});

	it("maps http headers to header_value", () => {
		const origin: SecretOrigin = {
			server_name: "remote-api",
			field_group: "headers",
			field_key: "Authorization",
		};
		expect(inferSecretKindFromOrigin(origin)).toBe("header_value");
	});

	it("maps env API_KEY to api_key", () => {
		const origin: SecretOrigin = {
			server_name: "context7",
			field_group: "env",
			field_key: "API_KEY",
		};
		expect(inferSecretKindFromOrigin(origin)).toBe("api_key");
	});

	it("maps url param token to token", () => {
		const origin: SecretOrigin = {
			server_name: "context7",
			field_group: "url_params",
			field_key: "token",
		};
		expect(inferSecretKindFromOrigin(origin)).toBe("token");
	});

	it("maps env password key to password", () => {
		const origin: SecretOrigin = {
			server_name: "db",
			field_group: "env",
			field_key: "DB_PASSWORD",
		};
		expect(inferSecretKindFromOrigin(origin)).toBe("password");
	});
});

describe("inferSecretLabelFromOrigin", () => {
	it("builds a readable label from context", () => {
		const origin: SecretOrigin = {
			server_name: "context7",
			field_group: "url_params",
			field_key: "token",
		};
		expect(inferSecretLabelFromOrigin(origin)).toBe(
			"context7 · URL parameter · token",
		);
	});

	it("uses k-index when key is missing", () => {
		const origin: SecretOrigin = {
			server_name: "context7",
			field_group: "url_params",
			field_index: 1,
		};
		expect(inferSecretLabelFromOrigin(origin)).toBe(
			"context7 · URL parameter · k2",
		);
	});

	it("uses translated field group labels when provided", () => {
		const origin: SecretOrigin = {
			server_name: "context7",
			field_group: "url_params",
			field_key: "token",
		};
		expect(
			inferSecretLabelFromOrigin(origin, (key) =>
				key === "originLabel.urlParameter" ? "URL 参数" : key,
			),
		).toBe("context7 · URL 参数 · token");
	});
});

describe("inferSecretDefaultsFromOrigin", () => {
	it("returns kind and label together", () => {
		const origin: SecretOrigin = {
			server_name: "context7",
			field_group: "env",
			field_key: "TOKEN",
		};
		expect(inferSecretDefaultsFromOrigin(origin)).toEqual({
			kind: "token",
			label: "context7 · Environment variable · TOKEN",
		});
	});
});

describe("kindOptionsForEditor", () => {
	const allOptions = [
		{ value: "token" as const, label: "Token" },
		{ value: "oauth_access_token" as const, label: "OAuth access" },
	];

	it("hides oauth kinds on create", () => {
		expect(
			kindOptionsForEditor(allOptions, { mode: "create", kind: "token" }),
		).toEqual([{ value: "token", label: "Token" }]);
	});

	it("keeps oauth kind visible when editing an oauth secret", () => {
		expect(
			kindOptionsForEditor(allOptions, {
				mode: "edit",
				kind: "oauth_access_token",
			}),
		).toEqual(allOptions);
	});
});

describe("USER_CREATABLE_SECRET_KINDS", () => {
	it("excludes oauth kinds", () => {
		expect(USER_CREATABLE_SECRET_KINDS).not.toContain("oauth_access_token");
		expect(USER_CREATABLE_SECRET_KINDS).not.toContain("oauth_refresh_token");
		expect(isUserCreatableSecretKind("token")).toBe(true);
		expect(isUserCreatableSecretKind("oauth_access_token")).toBe(false);
	});
});
