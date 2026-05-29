import { describe, expect, test } from "bun:test";

import { groupSelectedDiscoveryServerConfigs } from "./onboarding-server-selection";

describe("onboarding server selection", () => {
	test("rejects duplicate preset server names instead of overwriting configs", () => {
		expect(() =>
			groupSelectedDiscoveryServerConfigs(
				[
					{
						key: "server:a",
						name: "shared-name",
						kind: "stdio",
						command: "node",
						args: ["a.js"],
						env: {},
						source_clients: ["MCPMate"],
						source_client_ids: [],
						import_config: { command: "node", args: ["a.js"] },
					},
					{
						key: "server:b",
						name: "shared-name",
						kind: "stdio",
						command: "node",
						args: ["b.js"],
						env: {},
						source_clients: ["MCPMate"],
						source_client_ids: [],
						import_config: { command: "node", args: ["b.js"] },
					},
				],
				new Set(["server:a", "server:b"]),
			),
		).toThrow("Duplicate preset server name: shared-name");
	});
});
