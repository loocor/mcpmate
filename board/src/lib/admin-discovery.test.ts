import { describe, expect, test } from "bun:test";
import {
	ADMIN_DISCOVERY_BASE_URL,
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
				{ surface: "onboarding", random: 20, limit: 50, offset: 10, platform: "macos" },
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
			`${ADMIN_DISCOVERY_BASE_URL}/discovery/clients?surface=onboarding&random=6`,
		);
		expect(requests[0].init?.credentials).toBe("omit");
	});

	test("maps Admin v2 client metadata into backend-recognized update payloads only", () => {
		const payload = adminDiscoveryClientToUpdatePayload(
			{
				identifier: "cursor-desktop",
				displayName: "Cursor",
				description: "AI code editor",
				links: {
					homepage: "https://cursor.com",
					docs: "https://docs.cursor.com",
					support: "https://support.cursor.com",
				},
				icon: {
					url: "https://example.com/cursor.png",
				},
				config: {
					kind: "none",
				},
				unrecognized: true,
			},
		);

		expect(payload).toEqual({
			identifier: "cursor-desktop",
			display_name: "Cursor",
			config_file_state: "without_config_file",
			description: "AI code editor",
			homepage_url: "https://cursor.com",
			docs_url: "https://docs.cursor.com",
			support_url: "https://support.cursor.com",
			logo_url: "https://example.com/cursor.png",
			clear_config_file_parse: true,
			clear_transports: true,
		});
		expect(payload).not.toHaveProperty("unrecognized");
	});

	test("filters Admin clients that cannot form backend client records", () => {
		expect(adminDiscoveryClientToCandidate({ displayName: "Missing ID" })).toBeNull();
		expect(
			adminDiscoveryClientToCandidate({
				identifier: "no-local-path",
				displayName: "No Local Path",
				config: {
					kind: "file",
					file: { format: "json", container: { keys: ["mcpServers"] } },
					transports: { stdio: { command_field: "command" } },
				},
			}),
		).toMatchObject({
			identifier: "no-local-path",
			configFileChoice: "without_config_file",
			configPath: "",
		});
	});

	test("uses v2 config file paths only for the explicit current platform", () => {
		const rawClient = {
			identifier: "cursor-desktop",
			displayName: "Cursor",
			config_path: "~/Library/Application Support/Cursor/legacy-client.json",
			config: {
				kind: "file",
				file: {
					format: "json",
					path: "~/Library/Application Support/Cursor/legacy-file.json",
					config_path: "~/Library/Application Support/Cursor/legacy-file-config.json",
					paths: {
						macos: "~/Library/Application Support/Cursor/User/globalStorage/mcp.json",
						windows: "%APPDATA%\\Cursor\\User\\globalStorage\\mcp.json",
					},
					container: {
						type: "standard",
						keys: ["mcpServers"],
					},
				},
				transports: {
					stdio: {
						command_field: "command",
						args_field: "args",
						env_field: "env",
					},
				},
			},
		};

		expect(adminDiscoveryClientToCandidate(rawClient)).toMatchObject({
			configFileChoice: "without_config_file",
			configPath: "",
		});
		expect(adminDiscoveryClientToCandidate(rawClient, { platform: "linux" })).toMatchObject({
			configFileChoice: "without_config_file",
			configPath: "",
		});
		expect(adminDiscoveryClientToCandidate(rawClient, { platform: "macos" })).toMatchObject({
			configFileChoice: "with_config_file",
			configPath: "~/Library/Application Support/Cursor/User/globalStorage/mcp.json",
			configFileParseFormat: "json",
			configFileParseContainerType: "standard",
			configFileParseContainerKeysText: "mcpServers",
			supportedTransports: ["stdio"],
		});
	});

	test("ignores legacy config paths when the current platform path is missing", () => {
		const rawClient = {
			identifier: "legacy-paths",
			displayName: "Legacy Paths",
			config_path: "~/Library/Application Support/Legacy/client-level.json",
			config: {
				kind: "file",
				file: {
					format: "json",
					path: "~/Library/Application Support/Legacy/file-path.json",
					config_path: "~/Library/Application Support/Legacy/file-config-path.json",
					paths: {
						windows: "%APPDATA%\\Legacy\\mcp.json",
						linux: "~/.config/legacy/mcp.json",
					},
					container: {
						type: "standard",
						keys: ["mcpServers"],
					},
				},
			},
		};

		expect(adminDiscoveryClientToCandidate(rawClient, { platform: "macos" })).toMatchObject({
			configFileChoice: "without_config_file",
			configPath: "",
		});

		expect(
			adminDiscoveryClientToCandidate(
				{
					...rawClient,
					config: {
						...rawClient.config,
						file: {
							...rawClient.config.file,
							paths: undefined,
						},
					},
				},
				{ platform: "macos" },
			),
		).toMatchObject({
			configFileChoice: "without_config_file",
			configPath: "",
		});
	});

	test("does not treat legacy has_config_file kind as writable config", () => {
		const rawClient = {
			identifier: "legacy-kind",
			displayName: "Legacy Kind",
			config: {
				kind: "has_config_file",
				file: {
					format: "json",
					paths: {
						macos: "~/Library/Application Support/Legacy Kind/mcp.json",
					},
					container: {
						type: "standard",
						keys: ["mcpServers"],
					},
				},
			},
		};

		expect(adminDiscoveryClientToCandidate(rawClient, { platform: "macos" })).toMatchObject({
			configFileChoice: "without_config_file",
			configPath: "",
		});
	});

	test("maps a v2 Admin client candidate into a config-aware update payload", () => {
		const candidate = adminDiscoveryClientToCandidate(
			{
				identifier: "cursor-desktop",
				displayName: "Cursor",
				config: {
					kind: "file",
					file: {
						format: "json",
						paths: {
							macos: "~/Library/Application Support/Cursor/mcp.json",
						},
						container: {
							type: "standard",
							keys: ["mcpServers"],
						},
					},
					transports: {
						stdio: {
							command_field: "command",
							args_field: "args",
							env_field: "env",
						},
					},
				},
			},
			{ platform: "macos" },
		);

		expect(candidate).not.toBeNull();
		expect(adminDiscoveryClientToUpdatePayload(candidate)).toMatchObject({
			identifier: "cursor-desktop",
			display_name: "Cursor",
			config_file_state: "with_config_file",
			config_path: "~/Library/Application Support/Cursor/mcp.json",
			config_file_parse: {
				format: "json",
				container_type: "standard",
				container_keys: ["mcpServers"],
			},
			clear_config_file_parse: false,
			transports: {
				stdio: {
					command_field: "command",
					args_field: "args",
					env_field: "env",
				},
			},
			clear_transports: false,
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
