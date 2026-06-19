import { describe, expect, test } from "bun:test";
import { normalizeIngestPayload, parseJsonDrafts } from "./install-normalizer";

function serverJson(name: string, command: string): string {
	return JSON.stringify({
		mcpServers: {
			[name]: {
				command,
				args: [`${name}-server`],
			},
		},
	});
}

function summarizeDrafts(
	drafts: Awaited<ReturnType<typeof normalizeIngestPayload>>,
): Array<{ name: string; command?: string; args: string[] }> {
	return drafts.map((draft) => ({
		name: draft.name,
		command: draft.command,
		args: draft.args,
	}));
}

describe("install normalizer", () => {
	test("normalizes prefixed MCP server config text", async () => {
		const drafts = await normalizeIngestPayload({
			text: `Server Config

{
  "mcpServers": {
    "playwright": {
      "command": "npx",
      "args": [
        "@playwright/mcp@latest"
      ]
    }
  }
}`,
		});

		expect(summarizeDrafts(drafts)).toEqual([
			{
				name: "playwright",
				command: "npx",
				args: ["@playwright/mcp@latest"],
			},
		]);
	});

	test("normalizes loose mcpServers property text", async () => {
		const drafts = await normalizeIngestPayload({
			text: `"mcpServers": {
  "playwright": {
    "command": "npx",
    "args": [
      "@playwright/mcp@latest"
    ]
  }
}`,
		});

		expect(summarizeDrafts(drafts)).toEqual([
			{
				name: "playwright",
				command: "npx",
				args: ["@playwright/mcp@latest"],
			},
		]);
	});

	test("normalizes complete server objects from malformed surrounding JSON", async () => {
		const samples = [
			`{
  "mcpServers": {
    "playwright": {
      "command": "npx",
      "args": [
        "@playwright/mcp@latest"
      ]
    }`,
			`{
  "playwright": {
    "command": "npx",
    "args": [
      "@playwright/mcp@latest"
    ]
  }
}
}`,
		];

		for (const text of samples) {
			const drafts = await normalizeIngestPayload({ text });

			expect(summarizeDrafts(drafts)).toEqual([
				{
					name: "playwright",
					command: "npx",
					args: ["@playwright/mcp@latest"],
				},
			]);
		}
	});

	test("normalizes complete loose server maps", async () => {
		const drafts = await normalizeIngestPayload({
			text: `{
  "playwright": {
    "command": "npx",
    "args": [
      "@playwright/mcp@latest"
    ]
  }
}`,
		});

		expect(summarizeDrafts(drafts)).toEqual([
			{
				name: "playwright",
				command: "npx",
				args: ["@playwright/mcp@latest"],
			},
		]);
	});

	test("normalizes complete single server objects", async () => {
		const drafts = await normalizeIngestPayload({
			text: `{
  "name": "playwright",
  "command": "npx",
  "args": [
    "@playwright/mcp@latest"
  ]
}`,
		});

		expect(summarizeDrafts(drafts)).toEqual([
			{
				name: "playwright",
				command: "npx",
				args: ["@playwright/mcp@latest"],
			},
		]);
	});

	test("ignores nested command objects outside server containers", async () => {
		const drafts = await normalizeIngestPayload({
			text: `Broken payload
{
  "workflow": {
    "step": {
      "command": "npx",
      "args": [
        "@playwright/mcp@latest"
      ]
    }
  }`,
		});

		expect(summarizeDrafts(drafts)).toEqual([]);
	});

	test("ignores complete unrelated JSON objects", async () => {
		const samples = [
			`{}`,
			`{
  "workflow": {
    "step": {
      "command": "npx",
      "args": [
        "@playwright/mcp@latest"
      ]
    }
  }
}`,
		];

		for (const text of samples) {
			const drafts = await normalizeIngestPayload({ text });
			expect(summarizeDrafts(drafts)).toEqual([]);
		}
	});

	test("keeps JSON edit parsing strict", () => {
		expect(() =>
			parseJsonDrafts(`Server Config

{
  "mcpServers": {
    "playwright": {
      "command": "npx"
    }
  }
}`),
		).toThrow();
	});

	test("ignores legacy registry_server_id field", async () => {
		const drafts = await normalizeIngestPayload({
			text: JSON.stringify({
				mcpServers: {
					"google-ads": {
						command: "npx",
						args: ["-y", "@google-ads/mcp"],
						registry_server_id: "google-ads",
					},
				},
			}),
		});

		expect(drafts).toHaveLength(1);
		expect(drafts[0]?.source).toEqual({ type: "other" });
	});

	test("splits merged stdio command when args are missing", async () => {
		const drafts = await normalizeIngestPayload({
			text: JSON.stringify({
				mcpServers: {
					"mcp-mermaid": {
						command: "npx -y mcp-mermaid",
					},
				},
			}),
		});

		expect(summarizeDrafts(drafts)).toEqual([
			{
				name: "mcp-mermaid",
				command: "npx",
				args: ["-y", "mcp-mermaid"],
			},
		]);
	});

	test("preserves separate command and args from cursor.directory", async () => {
		const drafts = await normalizeIngestPayload({
			text: JSON.stringify({
				mcpServers: {
					"phantom-mcp-server": {
						command: "npx",
						args: ["-y", "@phantom/mcp-server"],
						env: { PHANTOM_APP_ID: "your-phantom-app-id" },
					},
				},
			}),
		});

		expect(summarizeDrafts(drafts)).toEqual([
			{
				name: "phantom-mcp-server",
				command: "npx",
				args: ["-y", "@phantom/mcp-server"],
			},
		]);
	});

	test("splits merged command when args is an empty array", async () => {
		const drafts = await normalizeIngestPayload({
			text: JSON.stringify({
				mcpServers: {
					"mcp-mermaid": {
						command: "npx -y mcp-mermaid",
						args: [],
					},
				},
			}),
		});

		expect(summarizeDrafts(drafts)).toEqual([
			{
				name: "mcp-mermaid",
				command: "npx",
				args: ["-y", "mcp-mermaid"],
			},
		]);
	});

	test("normalizes multi-file ingest payloads into drafts", async () => {
		const drafts = await normalizeIngestPayload({
			payloads: [
				{ text: serverJson("one", "uvx"), fileName: "one.json" },
				{ text: serverJson("two", "node"), fileName: "two.json" },
			],
		});

		expect(summarizeDrafts(drafts)).toEqual([
			{ name: "one", command: "uvx", args: ["one-server"] },
			{ name: "two", command: "node", args: ["two-server"] },
		]);
	});
});
