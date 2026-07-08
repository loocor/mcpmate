import { describe, expect, it } from "vitest";

import {
	capabilityFamilyListMethod,
	capabilityRecordToListItem,
} from "./inspector-capability-list-api";

describe("inspector capability list api", () => {
	it("maps tasks to the MCP tasks/list method", () => {
		expect(capabilityFamilyListMethod("tasks")).toBe("tasks/list");
	});

	it("normalizes task records using task identifiers", () => {
		expect(
			capabilityRecordToListItem(
				{
					taskId: "task-1",
					status: "working",
					statusMessage: "Indexing repository",
				},
				"tasks",
			),
		).toMatchObject({
			key: "task-1",
			title: "task-1",
			metadata: {
				taskId: "task-1",
				status: "working",
				statusMessage: "Indexing repository",
			},
		});
	});

	it("builds prompt input schema from advertised arguments", () => {
		expect(
			capabilityRecordToListItem(
				{
					name: "args-prompt",
					description: "A prompt with arguments",
					arguments: [
						{
							name: "required_arg",
							type: "string",
							description: "Required input",
							required: true,
						},
						{
							name: "optional_arg",
							type: "number",
							description: "Optional input",
							default: 3,
						},
					],
				},
				"prompts",
			),
		).toMatchObject({
			key: "args-prompt",
			inputSchema: {
				type: "object",
				properties: {
					required_arg: {
						type: "string",
						description: "Required input",
					},
					optional_arg: {
						type: "number",
						description: "Optional input",
						default: 3,
					},
				},
				required: ["required_arg"],
			},
		});
	});
});
