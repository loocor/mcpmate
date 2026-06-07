import { describe, expect, it } from "vitest";
import type { SecretOrigin } from "./types";
import {
	generateSecretAliasFromOrigin,
	resolveUniqueSecretAlias,
	suggestSecretAliasFromOrigin,
} from "./secret-alias";

describe("generateSecretAliasFromOrigin", () => {
	it("uses object type, server name, field group, and env key", () => {
		const origin: SecretOrigin = {
			server_name: "context7",
			source: "server_edit",
			field_group: "env",
			field_key: "TOKEN",
			field_path: "env.0.value",
		};
		expect(generateSecretAliasFromOrigin(origin)).toBe("server-context7-env-token");
	});

	it("uses k-index for args without key", () => {
		const origin: SecretOrigin = {
			server_name: "my-server",
			source: "server_install",
			field_group: "args",
			field_index: 0,
			field_path: "args.0.value",
		};
		expect(generateSecretAliasFromOrigin(origin)).toBe("server-my-server-args-k1");
	});

	it("uses command segment for stdio command", () => {
		const origin: SecretOrigin = {
			server_name: "context7",
			source: "server_edit",
			field_group: "stdio",
			field_key: "command",
			field_path: "command",
		};
		expect(generateSecretAliasFromOrigin(origin)).toBe("server-context7-command");
	});

	it("uses url segment for streamable http url", () => {
		const origin: SecretOrigin = {
			server_name: "remote-api",
			source: "server_edit",
			field_group: "streamable_http",
			field_key: "url",
			field_path: "url",
		};
		expect(generateSecretAliasFromOrigin(origin)).toBe("server-remote-api-url");
	});

	it("uses header key for http headers", () => {
		const origin: SecretOrigin = {
			server_name: "remote-api",
			source: "server_edit",
			field_group: "headers",
			field_key: "Authorization",
			field_path: "headers.0.value",
		};
		expect(generateSecretAliasFromOrigin(origin)).toBe(
			"server-remote-api-headers-authorization",
		);
	});

	it("uses url-parameters group and key for url params", () => {
		const origin: SecretOrigin = {
			server_name: "context7",
			source: "server_edit",
			field_group: "url_params",
			field_key: "token",
			field_index: 0,
			field_path: "urlParams.0.value",
		};
		expect(generateSecretAliasFromOrigin(origin)).toBe(
			"server-context7-url-parameters-token",
		);
	});

	it("uses k-index for url params without key", () => {
		const origin: SecretOrigin = {
			server_name: "context7",
			source: "server_edit",
			field_group: "url_params",
			field_index: 1,
			field_path: "urlParams.1.value",
		};
		expect(generateSecretAliasFromOrigin(origin)).toBe(
			"server-context7-url-parameters-k2",
		);
	});
});

describe("resolveUniqueSecretAlias", () => {
	it("returns base alias when unused", () => {
		expect(
			resolveUniqueSecretAlias("server-context7-env-token", ["other-secret"]),
		).toBe("server-context7-env-token");
	});

	it("appends numeric suffix on collision", () => {
		expect(
			resolveUniqueSecretAlias("server-context7-env-token", [
				"server-context7-env-token",
				"server-context7-env-token-2",
			]),
		).toBe("server-context7-env-token-3");
	});
});

describe("suggestSecretAliasFromOrigin", () => {
	it("combines generation and collision resolution", () => {
		const origin: SecretOrigin = {
			server_name: "context7",
			source: "server_edit",
			field_group: "env",
			field_key: "TOKEN",
		};
		expect(
			suggestSecretAliasFromOrigin(origin, ["server-context7-env-token"]),
		).toBe("server-context7-env-token-2");
	});
});
