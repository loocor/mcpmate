import { describe, expect, test } from "bun:test";
import {
	buildAdminDiscoveryUrl,
	adminDiscoveryClientToCandidate,
	adminDiscoveryClientToUpdatePayload,
	fetchAdminDiscoveryClients,
	adminDiscoveryServerToOnboardingCandidate,
} from "./admin-discovery";

describe("admin discovery adapter", () => {
	test("builds direct Admin discovery URLs with capped query values", () => {
		expect(
			buildAdminDiscoveryUrl(
				"/discovery/clients",
				{ surface: "onboarding", random: 20, limit: 50, offset: 10 },
				"https://admin.example.com/",
			),
		).toBe("https://admin.example.com/discovery/clients?surface=onboarding&random=12");

		expect(
			buildAdminDiscoveryUrl(
				"/discovery/clients",
				{ limit: 200, offset: -2 },
				"https://admin.example.com",
			),
		).toBe("https://admin.example.com/discovery/clients?limit=50&offset=0");
	});

	test("fetches Admin discovery directly without credentials", async () => {
		const originalFetch = globalThis.fetch;
		const requests: Array<{ input: string | URL | Request; init?: RequestInit }> = [];
		globalThis.fetch = ((input: string | URL | Request, init?: RequestInit) => {
			requests.push({ input, init });
			return Promise.resolve(
				new Response(
					JSON.stringify({
						schemaVersion: "test",
						generatedAt: new Date().toISOString(),
						clients: [{ identifier: "cursor-desktop", displayName: "Cursor" }],
						metadata: {},
					}),
					{ status: 200, headers: { "content-type": "application/json" } },
				),
			);
		}) as typeof fetch;

		try {
			await expect(fetchAdminDiscoveryClients({ surface: "onboarding", random: 6 })).resolves.toHaveLength(1);
		} finally {
			globalThis.fetch = originalFetch;
		}

		expect(requests).toHaveLength(1);
		expect(String(requests[0].input)).toBe(
			"https://public.mcp.umate.ai/discovery/clients?surface=onboarding&random=6",
		);
		expect(requests[0].init?.credentials).toBe("omit");
	});

	test("maps Admin clients into backend-recognized client update payloads only", () => {
		const payload = adminDiscoveryClientToUpdatePayload(
			{
				identifier: "cursor-desktop",
				displayName: "Cursor",
				description: "AI code editor",
				homepage_url: "https://cursor.com",
				logoUrl: "https://example.com/cursor.png",
				config: {
					kind: "has_config_file",
					file: {
						format: "json",
						containerType: "standard",
						containerKeys: ["mcpServers"],
					},
					transports: {
						stdio: {
							include_type: true,
							type_value: "stdio",
							command_field: "command",
							args_field: "args",
							env_field: "env",
						},
					},
				},
				backend_template: { should: "not pass through" },
				unrecognized: true,
			},
			{ forceWithoutConfigFile: true },
		);

		expect(payload).toEqual({
			identifier: "cursor-desktop",
			display_name: "Cursor",
			config_file_state: "without_config_file",
			description: "AI code editor",
			homepage_url: "https://cursor.com",
			logo_url: "https://example.com/cursor.png",
			clear_config_file_parse: true,
			clear_transports: true,
		});
		expect(payload).not.toHaveProperty("backend_template");
		expect(payload).not.toHaveProperty("unrecognized");
	});

	test("filters Admin clients that cannot form backend client records", () => {
		expect(adminDiscoveryClientToCandidate({ displayName: "Missing ID" })).toBeNull();
		expect(
			adminDiscoveryClientToCandidate({
				identifier: "no-local-path",
				displayName: "No Local Path",
				config: {
					kind: "has_config_file",
					file: { format: "json", containerKeys: ["mcpServers"] },
					transports: { stdio: { command_field: "command" } },
				},
			}),
		).toMatchObject({
			identifier: "no-local-path",
			configFileChoice: "without_config_file",
			configPath: "",
		});
	});

	test("maps Admin servers into onboarding import candidates", () => {
		const candidate = adminDiscoveryServerToOnboardingCandidate({
			id: "github",
			official: {
				title: "GitHub",
			},
			runtime: {
				install_config: {
					type: "stdio",
					command: "npx",
					args: ["-y", "@modelcontextprotocol/server-github"],
					env: {
						GITHUB_TOKEN: "${GITHUB_TOKEN}",
						IGNORED_NUMBER: 1,
					},
					extra: "ignored",
				},
			},
		});

		expect(candidate).toEqual({
			key: "admin:github",
			name: "GitHub",
			kind: "stdio",
			command: "npx",
			args: ["-y", "@modelcontextprotocol/server-github"],
			env: {
				GITHUB_TOKEN: "${GITHUB_TOKEN}",
			},
			url: undefined,
			source_clients: ["MCPMate Admin"],
			source_client_ids: [],
			import_config: {
				type: "stdio",
				registry_server_id: "github",
				command: "npx",
				args: ["-y", "@modelcontextprotocol/server-github"],
				env: {
					GITHUB_TOKEN: "${GITHUB_TOKEN}",
				},
			},
		});
	});

	test("filters Admin servers that cannot form backend import payloads", () => {
		expect(
			adminDiscoveryServerToOnboardingCandidate({
				id: "missing-command",
				runtime: { install_config: { type: "stdio" } },
			}),
		).toBeNull();
		expect(
			adminDiscoveryServerToOnboardingCandidate({
				id: "missing-url",
				runtime: { install_config: { type: "streamable_http" } },
			}),
		).toBeNull();
	});
});
