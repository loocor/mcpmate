import { afterEach, describe, expect, test } from "bun:test";

import * as apiModule from "./api";
import { configSuitsApi, serversApi } from "./api";

const originalFetch = globalThis.fetch;

afterEach(() => {
	globalThis.fetch = originalFetch;
});

describe("Profile authoring API", () => {
	test("preserves profile conflict status code and details", async () => {
		globalThis.fetch = async () =>
			new Response(
				JSON.stringify({
					error: {
						message: "Profile was changed by another author",
						status: 409,
						code: "profile_authoring_changed",
						details: { currentAuthoringGeneration: 13 },
					},
				}),
				{ status: 409, statusText: "Conflict" },
			);

		const error = await configSuitsApi
			.getAuthoringView("profile-a")
			.catch((value: unknown) => value);

		expect(error?.constructor?.name).toBe("ApiRequestError");
		expect(error).toMatchObject({
			status: 409,
			code: "profile_authoring_changed",
			details: { currentAuthoringGeneration: 13 },
			message: "Profile was changed by another author",
		});
	});

	test("preserves ordinary error status and message", async () => {
		globalThis.fetch = async () =>
			new Response(
				JSON.stringify({
					error: {
						message: "Profile not found",
						status: 404,
					},
				}),
				{ status: 404, statusText: "Not Found" },
			);

		const error = await configSuitsApi
			.getAuthoringView("missing")
			.catch((value: unknown) => value);

		expect(error?.constructor?.name).toBe("ApiRequestError");
		expect(error).toMatchObject({
			status: 404,
			message: "Profile not found",
		});
	});

	test("maps Workflow specification save and preview to their dedicated APIs", async () => {
		const requests: Array<{ url: string; init?: RequestInit }> = [];
		globalThis.fetch = async (input, init) => {
			requests.push({ url: String(input), init });
			if (String(input).includes("/preview")) {
				return Response.json({
					success: true,
					data: {
						preview: {
							profile_id: "profile-workflow",
							specification_revision: 1,
							validation_notes: null,
							avoid_rules: null,
							steps: [],
							valid: true,
						},
					},
				});
			}
			return Response.json({
				success: true,
				data: {
					specification: {
						profile_id: "profile-workflow",
						specification_revision: 1,
						validation_notes: null,
						avoid_rules: null,
						tool_binding_count: 1,
						steps: [],
					},
				},
			});
		};

		await configSuitsApi.saveWorkflowSpecification({
			profile_id: "profile-workflow",
			expected_specification_revision: null,
			validation_notes: "Confirm source coverage.",
			avoid_rules: null,
			steps: [
				{
					title: "Collect",
					description: null,
					bindings: [{ ref_id: "capability-a", binding_policy: "meta_on_demand" }],
				},
			],
		});
		const preview = await configSuitsApi.getWorkflowSpecificationPreview("profile-workflow");

		expect(new URL(requests[0]!.url, "http://localhost").pathname).toBe(
			"/api/mcp/profile/workflow/specification/save",
		);
		expect(JSON.parse(String(requests[0]!.init?.body))).toMatchObject({
			profile_id: "profile-workflow",
			expected_specification_revision: null,
			validation_notes: "Confirm source coverage.",
			avoid_rules: null,
			steps: [{ bindings: [{ ref_id: "capability-a", binding_policy: "meta_on_demand" }] }],
		});
		expect(new URL(requests[1]!.url, "http://localhost").pathname).toBe(
			"/api/mcp/profile/workflow/specification/preview",
		);
		expect(preview.valid).toBeTrue();
	});

	test("maps every Profile writer to its generation-aware request shape", async () => {
		const requests: Array<{ url: string; body: unknown }> = [];
		globalThis.fetch = async (input, init) => {
			requests.push({
				url: String(input),
				body: init?.body ? JSON.parse(String(init.body)) : undefined,
			});
			return Response.json({ success: true, data: {} });
		};

		await configSuitsApi.activateSuit("profile-a", 12);
		await configSuitsApi.deactivateSuit("profile-a", 13);
		await configSuitsApi.deleteSuit("profile-a", 14);
		await configSuitsApi.enableServer("profile-a", "server-a", 15);
		await configSuitsApi.enableTool(
			"profile-a",
			"tool-a",
			16,
			{ "server-a": 4 },
		);

		expect(
			requests.map(({ url }) => new URL(url, "http://localhost").pathname),
		).toEqual([
			"/api/mcp/profile/manage",
			"/api/mcp/profile/manage",
			"/api/mcp/profile/delete",
			"/api/mcp/profile/servers/manage",
			"/api/mcp/profile/tools/manage",
		]);
		expect(requests.map(({ body }) => body)).toEqual([
		{
				ids: ["profile-a"],
				action: "activate",
				expected_authoring_generations: { "profile-a": 12 },
		},
		{
				ids: ["profile-a"],
				action: "deactivate",
				expected_authoring_generations: { "profile-a": 13 },
		},
		{
			id: "profile-a",
			expected_authoring_generation: 14,
		},
		{
				profile_id: "profile-a",
				component_ids: ["server-a"],
				action: "enable",
				expected_authoring_generation: 15,
		},
		{
				profile_id: "profile-a",
				component_ids: ["tool-a"],
				action: "enable",
				expected_authoring_generation: 16,
				source_revision_set: { "server-a": 4 },
		},
		]);
	});

	test("associates discovered imported servers with one current Profile save", async () => {
		const requests: Array<{ url: string; body: unknown }> = [];
		globalThis.fetch = async (input, init) => {
			const url = String(input);
			requests.push({
				url,
				body: init?.body ? JSON.parse(String(init.body)) : undefined,
			});
			if (url.includes("/api/mcp/servers/list")) {
				return Response.json({
					success: true,
					data: {
						servers: [
							{ id: "server-a", name: "Imported A", enabled: true },
							{ id: "server-z", name: "Unrelated", enabled: true },
						],
					},
				});
			}
			if (url.includes("/api/mcp/profile/authoring/view")) {
				return Response.json({
					success: true,
					data: {
						profile: {
							id: "profile-a",
							name: "Profile A",
							description: null,
							profile_type: "shared",
							priority: 50,
							is_active: true,
							is_default: false,
							authoring_generation: 7,
							role: "user",
							allowed_operations: ["update"],
						},
						server_ids: ["server-existing"],
					},
				});
			}
			return Response.json({
				success: true,
				data: {
					profile: {
						id: "profile-a",
						name: "Profile A",
						description: null,
						profile_type: "shared",
						priority: 50,
						is_active: true,
						is_default: false,
						authoring_generation: 8,
						role: "user",
						allowed_operations: ["update"],
					},
				},
			});
		};

		await apiModule.associateImportedServersWithProfile(
			"profile-a",
			["Imported A"],
		);

		expect(
			requests.map(({ url }) => new URL(url, "http://localhost").pathname),
		).toEqual([
			"/api/mcp/servers/list",
			"/api/mcp/profile/authoring/view",
			"/api/mcp/profile/authoring/save",
		]);
		expect(requests[2]?.body).toEqual({
			id: "profile-a",
			expected_authoring_generation: 7,
			name: "Profile A",
			description: null,
			profile_type: "shared",
			priority: 50,
			is_active: true,
			is_default: false,
			server_ids: ["server-existing", "server-a"],
			clone_from_id: null,
		});
	});

	test("resolves an imported server from the second server-list page", async () => {
		const requestedOffsets: string[] = [];
		let saveBody: unknown;
		globalThis.fetch = async (input, init) => {
			const url = new URL(String(input), "http://localhost");
			if (url.pathname.endsWith("/api/mcp/servers/list")) {
				const offset = url.searchParams.get("offset") ?? "";
				requestedOffsets.push(offset);
				const servers =
					offset === "0"
						? Array.from({ length: 100 }, (_, index) => ({
								id: `filler-${index}`,
								name: `Filler ${index}`,
								enabled: true,
							}))
						: [{ id: "server-target", name: "Imported Target", enabled: true }];
				return Response.json({ success: true, data: { servers } });
			}
			if (url.pathname.endsWith("/api/mcp/profile/authoring/view")) {
				return Response.json({
					success: true,
					data: {
						profile: {
							id: "profile-a",
							name: "Profile A",
							profile_type: "shared",
							priority: 50,
							is_active: true,
							is_default: false,
							authoring_generation: 7,
							allowed_operations: [],
						},
						server_ids: [],
					},
				});
			}
			saveBody = init?.body ? JSON.parse(String(init.body)) : undefined;
			return Response.json({
				success: true,
				data: {
					profile: {
						id: "profile-a",
						name: "Profile A",
						profile_type: "shared",
						priority: 50,
						is_active: true,
						is_default: false,
						authoring_generation: 8,
						allowed_operations: [],
					},
				},
			});
		};

		await apiModule.associateImportedServersWithProfile(
			"profile-a",
			["Imported Target"],
		);

		expect(requestedOffsets).toEqual(["0", "100"]);
		expect(saveBody).toMatchObject({ server_ids: ["server-target"] });
	});

	test("returns a stable error when an imported server is missing", async () => {
		globalThis.fetch = async () =>
			Response.json({ success: true, data: { servers: [] } });

		const error = await apiModule
			.associateImportedServersWithProfile("profile-a", ["Missing"])
			.catch((value: unknown) => value);

		expect(error).toMatchObject({
			code: "imported_server_missing",
		});
	});

	test("returns a stable error when an imported server name is ambiguous", async () => {
		globalThis.fetch = async () =>
			Response.json({
				success: true,
				data: {
					servers: [
						{ id: "server-a", name: "Duplicate", enabled: true },
						{ id: "server-b", name: "Duplicate", enabled: true },
					],
				},
			});

		const error = await apiModule
			.associateImportedServersWithProfile("profile-a", ["Duplicate"])
			.catch((value: unknown) => value);

		expect(error).toMatchObject({
			code: "imported_server_ambiguous",
		});
	});

	test("associates committed imports before reporting import completion errors", async () => {
		const requestedPaths: string[] = [];
		globalThis.fetch = async (input) => {
			const url = new URL(String(input), "http://localhost");
			requestedPaths.push(url.pathname);
			if (url.pathname.endsWith("/api/mcp/servers/list")) {
				return Response.json({
					success: true,
					data: {
						servers: [{ id: "server-a", name: "Imported A", enabled: true }],
					},
				});
			}
			if (url.pathname.endsWith("/api/mcp/profile/authoring/view")) {
				return Response.json({
					success: true,
					data: {
						profile: {
							id: "profile-a",
							name: "Profile A",
							profile_type: "shared",
							priority: 50,
							is_active: true,
							is_default: false,
							authoring_generation: 7,
							allowed_operations: [],
						},
						server_ids: [],
					},
				});
			}
			return Response.json({
				success: true,
				data: {
					profile: {
						id: "profile-a",
						name: "Profile A",
						profile_type: "shared",
						priority: 50,
						is_active: true,
						is_default: false,
						authoring_generation: 8,
						allowed_operations: [],
					},
				},
			});
		};

		for (const failure of [
			{
				failedCount: 1,
				failedServers: ["Failed B"],
				runtimeSyncError: null,
				expectedMessage: "1 server import(s) failed",
			},
			{
				failedCount: 0,
				failedServers: [],
				runtimeSyncError: "pool update failed",
				expectedMessage:
					"Servers were imported, but runtime synchronization failed: pool update failed",
			},
		]) {
			requestedPaths.length = 0;
			const error = await apiModule
				.completeServerImportForProfile("profile-a", {
					importedCount: 1,
					importedServers: ["Imported A"],
					skippedCount: 0,
					skippedServers: [],
					skippedDetails: [],
					failedCount: failure.failedCount,
					failedServers: failure.failedServers,
					runtimeSyncError: failure.runtimeSyncError,
				})
				.catch((value: unknown) => value);

			expect(requestedPaths).toEqual([
				"/api/mcp/servers/list",
				"/api/mcp/profile/authoring/view",
				"/api/mcp/profile/authoring/save",
			]);
			expect(error).toBeInstanceOf(Error);
			expect(error).toMatchObject({ message: failure.expectedMessage });
		}
	});

	test("serializes stdio creates as tagged transports without Profile state", async () => {
		let requestBody: unknown;
		globalThis.fetch = async (_input, init) => {
			requestBody = init?.body ? JSON.parse(String(init.body)) : undefined;
			return Response.json({ success: true, data: {} });
		};

		await serversApi.createServer({
			name: "server-a",
			kind: "stdio",
			command: "server-a",
			args: ["--serve"],
			env: {
				API_KEY: "[[secret:server-token]]",
				LOG_LEVEL: "debug",
			},
			source: { type: "catalog", ref: "server-a" },
			pending_import: true,
			meta: { description: "A server" },
			enabled: true,
			profile_ids: ["profile-a"],
		});

		expect(requestBody).toEqual({
			name: "server-a",
			transport: {
				kind: "stdio",
				command: "server-a",
				args: ["--serve"],
				env: {
					API_KEY: { kind: "secret_ref", alias: "server-token" },
					LOG_LEVEL: { kind: "literal", value: "debug" },
				},
			},
			source: { type: "catalog", ref: "server-a" },
			pending_import: true,
			meta: { description: "A server" },
		});
	});

	test("serializes SSE updates with a tagged HTTP transport and endpoint fallback", async () => {
		let requestBody: unknown;
		globalThis.fetch = async (_input, init) => {
			requestBody = init?.body ? JSON.parse(String(init.body)) : undefined;
			return Response.json({ success: true, data: {} });
		};

		await serversApi.updateServer("server-a", {
			kind: "sse",
			command: "https://example.com/sse",
			headers: {
				Authorization: "[[secret:api-token]]",
				"X-Client": "board",
			},
			source: { type: "browser", ref: "example" },
			pending_import: false,
			meta: { version: "1.2.3" },
		});

		expect(requestBody).toEqual({
			id: "server-a",
			transport: {
				kind: "http",
				protocol: "sse",
				endpoint: "https://example.com/sse",
				headers: {
					Authorization: { kind: "secret_ref", alias: "api-token" },
					"X-Client": { kind: "literal", value: "board" },
				},
			},
			source: { type: "browser", ref: "example" },
			pending_import: false,
			meta: { version: "1.2.3" },
		});
	});

	test("serializes streamable HTTP updates as literal values unless the secret reference is exact", async () => {
		let requestBody: unknown;
		globalThis.fetch = async (_input, init) => {
			requestBody = init?.body ? JSON.parse(String(init.body)) : undefined;
			return Response.json({ success: true, data: {} });
		};

		await serversApi.updateServer("server-b", {
			kind: "streamable_http",
			url: "https://example.com/mcp",
			headers: {
				Authorization: "Bearer [[secret:api-token]]",
				"X-Token": "[[secret:stream-token]]",
			},
		});

		expect(requestBody).toEqual({
			id: "server-b",
			transport: {
				kind: "http",
				protocol: "streamable_http",
				endpoint: "https://example.com/mcp",
				headers: {
					Authorization: {
						kind: "literal",
						value: "Bearer [[secret:api-token]]",
					},
					"X-Token": { kind: "secret_ref", alias: "stream-token" },
				},
			},
		});
	});

	test("rejects partial updates without a transport kind", async () => {
		await expect(
			serversApi.updateServer("server-b", {
				command: "server-b",
			}),
		).rejects.toThrow("Server updates require a complete transport kind");
	});
});
