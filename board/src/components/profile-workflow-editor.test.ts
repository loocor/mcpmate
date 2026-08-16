import { describe, expect, test } from "bun:test";

import {
	serializeWorkflowSteps,
	withSingleWorkflowCapabilityBinding,
	workflowDraftFromSpecification,
} from "../lib/profile-workflow-specification";

describe("Workflow Profile editor", () => {
	test("serializes an empty workflow so the last step can be removed", () => {
		expect(serializeWorkflowSteps([])).toEqual([]);
	});

	test("replaces a step capability binding with exactly one binding", () => {
		const step = {
			title: "Gather context",
			description: "",
			bindings: [
				{ ref_id: "capability-a", binding_policy: "direct" as const },
				{ ref_id: "capability-b", binding_policy: "meta_on_demand" as const },
			],
		};

		expect(withSingleWorkflowCapabilityBinding(step, "capability-b")).toEqual({
			...step,
			bindings: [
				{ ref_id: "capability-b", binding_policy: "meta_on_demand" },
			],
		});
		expect(withSingleWorkflowCapabilityBinding(step, null).bindings).toEqual([]);
	});

	test("serializes ordered steps and explicit binding policies", () => {
		const steps = serializeWorkflowSteps([
			{
				title: "  Gather context ",
				description: "  ",
				bindings: [
					{ ref_id: "capability-a", binding_policy: "meta_on_demand" },
					{ ref_id: "capability-b", binding_policy: "direct" },
				],
			},
			{
				title: "Respond",
				description: "Use the gathered context.",
				bindings: [],
			},
		]);

		expect(steps).toEqual([
			{
				title: "Gather context",
				description: null,
				bindings: [
					{ ref_id: "capability-a", binding_policy: "meta_on_demand" },
					{ ref_id: "capability-b", binding_policy: "direct" },
				],
			},
			{
				title: "Respond",
				description: "Use the gathered context.",
				bindings: [],
			},
		]);
	});

	test("hydrates nullable descriptions without changing binding order", () => {
		const drafts = workflowDraftFromSpecification({
			profile_id: "profile-workflow",
			specification_revision: 3,
			validation_notes: null,
			avoid_rules: null,
			steps: [
				{
					title: "First",
					description: null,
					bindings: [
						{ ref_id: "capability-b", binding_policy: "direct" },
						{ ref_id: "capability-a", binding_policy: "meta_on_demand" },
					],
				},
			],
		});

		expect(drafts).toEqual([
			{
				title: "First",
				description: "",
				bindings: [
					{ ref_id: "capability-b", binding_policy: "direct" },
					{ ref_id: "capability-a", binding_policy: "meta_on_demand" },
				],
			},
		]);
	});
});
