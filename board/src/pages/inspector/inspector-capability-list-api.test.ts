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
		});
	});
});
