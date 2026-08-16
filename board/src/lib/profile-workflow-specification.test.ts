import { describe, expect, test } from "bun:test";

import { buildWorkflowCapabilityOptions } from "./profile-workflow-specification";

describe("buildWorkflowCapabilityOptions", () => {
	test("flattens tools, resources, prompts, and templates into options", () => {
		const options = buildWorkflowCapabilityOptions([
			{
				server: { id: "server-a", name: "Server A" },
				lists: {
					tools: {
						items: [
							{
								ref_id: "tool:server-a:lookup",
								tool_name: "lookup",
								unique_name: "server_a__lookup",
								description: "Look up a record by identifier.",
							},
						],
					},
					resources: {
						items: [
							{
								ref_id: "resource:server-a:doc",
								resource_uri: "docs/readme.md",
								unique_uri: "server-a://docs/readme.md",
							},
						],
					},
					prompts: {
						items: [
							{
								ref_id: "prompt:server-a:summarize",
								prompt_name: "summarize",
								unique_name: "server_a__summarize",
							},
						],
					},
					templates: {
						items: [
							{
								ref_id: "template:server-a:issues",
								uri_template: "issues/{id}",
								unique_uri_template: "server-a://issues/{id}",
							},
						],
					},
				},
			},
		]);

		expect(options).toEqual([
			{
				ref_id: "tool:server-a:lookup",
				label: "server_a__lookup",
				description: "Look up a record by identifier.",
			},
			{
				ref_id: "resource:server-a:doc",
				label: "server-a://docs/readme.md",
			},
			{
				ref_id: "prompt:server-a:summarize",
				label: "server_a__summarize",
			},
			{
				ref_id: "template:server-a:issues",
				label: "server-a://issues/{id}",
			},
		]);
	});

	test("keeps ref ids and falls back to upstream labels when needed", () => {
		const options = buildWorkflowCapabilityOptions([
			{
				server: { id: "server-b", name: "Server B" },
				lists: {
					tools: {
						items: [
							{ tool_name: "no-ref" },
							{
								ref_id: "tool:server-b:fetch",
								id: "legacy-tool-id",
								tool_name: "fetch-page",
							},
						],
					},
					resources: { items: [] },
					prompts: { items: [] },
					templates: { items: [] },
				},
			},
		]);

		expect(options).toEqual([
			{
				ref_id: "tool:server-b:fetch",
				label: "fetch-page",
			},
		]);
	});
});
