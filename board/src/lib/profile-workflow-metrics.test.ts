import { describe, expect, test } from "bun:test";

import { formatWorkflowToolBindingCount } from "./profile-workflow-metrics";

describe("formatWorkflowToolBindingCount", () => {
	test("reports persisted tool bindings as enabled and available", () => {
		expect(formatWorkflowToolBindingCount(3, false, false)).toBe("3/3");
	});

	test("keeps loading and failed workflow metrics distinct from an empty workflow", () => {
		expect(formatWorkflowToolBindingCount(undefined, true, false)).toBe("...");
		expect(formatWorkflowToolBindingCount(undefined, false, true)).toBe("—");
		expect(formatWorkflowToolBindingCount(0, false, false)).toBe("0/0");
	});
});
