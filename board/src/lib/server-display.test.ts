import { describe, expect, test } from "bun:test";

import {
  formatServerNamespaceTitle,
  getServerDisplayName,
} from "./server-display";
import type { ServerSummary } from "./types";

function server(overrides: Partial<ServerSummary> = {}): ServerSummary {
	return {
		id: "server-a",
    name: "managed_namespace_v2",
		status: "connected",
		...overrides,
	};
}

describe("getServerDisplayName", () => {
	test("uses the upstream title when present", () => {
		expect(
			getServerDisplayName(
				server({
					server_info: {
						name: "upstream-server",
						title: "Upstream Title",
						version: "1.0.0",
					},
				}),
			),
		).toBe("Upstream Title");
	});

  test("falls back to a title-cased MCPMate namespace", () => {
		expect(
			getServerDisplayName(
				server({ server_info: { name: "upstream-server", version: "1.0.0" } }),
			),
    ).toBe("Managed Namespace V2");
  });
});

describe("formatServerNamespaceTitle", () => {
  test("formats canonical and legacy separators consistently", () => {
    expect(formatServerNamespaceTitle("sequential_thinking_v2")).toBe(
      "Sequential Thinking V2",
    );
    expect(formatServerNamespaceTitle("legacy-server name")).toBe(
      "Legacy Server Name",
    );
	});
});
