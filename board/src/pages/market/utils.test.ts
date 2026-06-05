import { describe, expect, test } from "bun:test";
import type { RegistryServerEntry } from "../../lib/types";
import type { RemoteOption } from "./types";
import {
	buildDraftFromRemoteOption,
	hasUnsupportedRegistryPackageOption,
	hasPreviewableOption,
	isSupportedRegistryPackageType,
} from "./utils";

function packageOption(overrides: Partial<RemoteOption> = {}): RemoteOption {
	return {
		id: "context7-package",
		label: "Stdio - @upstash/context7-mcp",
		kind: "stdio",
		source: "package",
		url: null,
		headers: null,
		envVars: [{ name: "CONTEXT7_API_KEY", isRequired: true }],
		packageIdentifier: "@upstash/context7-mcp",
		packageMeta: {
			registryType: "npm",
			version: "1.0.31",
		},
		...overrides,
	};
}

describe("market registry install resolution", () => {
	test("resolves npm packages with explicit registry identifier and version", () => {
		const draft = buildDraftFromRemoteOption(packageOption(), "context7");

		expect(draft).toMatchObject({
			name: "context7",
			kind: "stdio",
			command: "npx",
			args: ["-y", "@upstash/context7-mcp@1.0.31"],
			env: {
				CONTEXT7_API_KEY: "",
			},
		});
	});

	test("resolves pypi packages through uvx", () => {
		const draft = buildDraftFromRemoteOption(
			packageOption({
				packageIdentifier: "mcp-server-fetch",
				packageMeta: {
					registryType: "pypi",
					version: "1.6.0",
					runtimeArguments: [{ name: "--python", type: "named", value: "3.12" }],
					packageArguments: [{ name: "--debug", type: "named" }],
				},
			}),
			"fetch",
		);

		expect(draft).toMatchObject({
			name: "fetch",
			kind: "stdio",
			command: "uvx",
			args: ["--python", "3.12", "mcp-server-fetch@1.6.0", "--debug"],
		});
	});

	test("rejects unsupported package registry types instead of falling back to npx", () => {
		expect(() =>
			buildDraftFromRemoteOption(
				packageOption({
					packageMeta: {
						registryType: "mcpb",
						version: "1.0.0",
					},
				}),
				"mcpb-server",
			),
		).toThrow("Unsupported package registry type");
	});

	test("does not mark unsupported package registry types as previewable", () => {
		expect(isSupportedRegistryPackageType("npm")).toBe(true);
		expect(isSupportedRegistryPackageType("pypi")).toBe(true);
		expect(isSupportedRegistryPackageType("mcpb")).toBe(false);
	});

	test("recognizes official package types that are not supported yet", () => {
		const server: RegistryServerEntry = {
			name: "mcpb-server",
			version: "1.0.0",
			packages: [
				{
					registryType: "mcpb",
					identifier: "https://example.com/server.mcpb",
					transport: {
						type: "stdio",
					},
				},
			],
		};

		expect(hasPreviewableOption(server)).toBe(false);
		expect(hasUnsupportedRegistryPackageOption(server)).toBe(true);
	});

	test("does not mark package entries with non-stdio transport as previewable", () => {
		const server: RegistryServerEntry = {
			name: "package-server",
			version: "1.0.0",
			packages: [
				{
					registryType: "npm",
					identifier: "@example/package-server",
					transport: {
						type: "streamable-http",
					},
				},
			],
		};

		expect(hasPreviewableOption(server)).toBe(false);
	});

	test("does not mark packages with unresolved required arguments as previewable", () => {
		const server: RegistryServerEntry = {
			name: "package-server",
			version: "1.0.0",
			packages: [
				{
					registryType: "npm",
					identifier: "@example/package-server",
					transport: {
						type: "stdio",
					},
					packageArguments: [
						{
							name: "--api-key",
							type: "named",
							isRequired: true,
						},
					],
				},
			],
		};

		expect(hasPreviewableOption(server)).toBe(false);
	});

	test("rejects package drafts with non-stdio transport", () => {
		expect(() =>
			buildDraftFromRemoteOption(
				packageOption({
					kind: "streamable_http",
				}),
				"package-server",
			),
		).toThrow("Package transport must be stdio");
	});

	test("rejects package drafts with unresolved required arguments", () => {
		expect(() =>
			buildDraftFromRemoteOption(
				packageOption({
					packageMeta: {
						registryType: "npm",
						version: "1.0.0",
						packageArguments: [
							{
								name: "--api-key",
								type: "named",
								isRequired: true,
							},
						],
					},
				}),
				"package-server",
			),
		).toThrow("Required package argument is missing a value");
	});

	test("uses remote headers instead of env values for remote drafts", () => {
		const draft = buildDraftFromRemoteOption(
			{
				id: "remote",
				label: "SSE - https://api.example.com/sse",
				kind: "sse",
				source: "remote",
				url: "https://api.example.com/sse",
				headers: [
					{
						name: "X-Region",
						default: "us-east-1",
					},
				],
				envVars: null,
				packageIdentifier: null,
				packageMeta: null,
			},
			"remote-api",
		);

		expect(draft).toMatchObject({
			name: "remote-api",
			kind: "sse",
			url: "https://api.example.com/sse",
			env: {},
			headers: {
				"X-Region": "us-east-1",
			},
		});
	});

	test("marks sse and streamable http remotes as previewable when no variables are required", () => {
		const server: RegistryServerEntry = {
			name: "remote-server",
			version: "1.0.0",
			remotes: [
				{
					type: "sse",
					url: "https://api.example.com/sse",
				},
			],
		};

		expect(hasPreviewableOption(server)).toBe(true);
	});

	test("does not mark remotes with unresolved url variables as previewable", () => {
		const server: RegistryServerEntry = {
			name: "remote-server",
			version: "1.0.0",
			remotes: [
				{
					type: "streamable-http",
					url: "https://api.example.com/{region}/mcp",
					variables: {
						region: {
							isRequired: true,
							default: "us-east-1",
						},
					},
				},
			],
		};

		expect(hasPreviewableOption(server)).toBe(false);
	});
});
