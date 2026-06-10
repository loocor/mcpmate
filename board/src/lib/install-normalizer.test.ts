import { describe, expect, test } from "bun:test";
import { normalizeIngestPayload } from "./install-normalizer";

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

describe("install normalizer", () => {
	test("normalizes multi-file ingest payloads into drafts", async () => {
		const drafts = await normalizeIngestPayload({
			payloads: [
				{ text: serverJson("one", "uvx"), fileName: "one.json" },
				{ text: serverJson("two", "node"), fileName: "two.json" },
			],
		});

		expect(
			drafts.map((draft) => ({
				name: draft.name,
				command: draft.command,
				args: draft.args,
			})),
		).toEqual([
			{ name: "one", command: "uvx", args: ["one-server"] },
			{ name: "two", command: "node", args: ["two-server"] },
		]);
	});
});
