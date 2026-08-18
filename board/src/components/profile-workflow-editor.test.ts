import { describe, expect, test } from "bun:test";

import {
		createEmptyWorkflowStep,
		isUnsavedWorkflowStep,
		serializeWorkflowSteps,
		withSingleWorkflowCapabilityBinding,
		workflowDraftFromSpecification,
	} from "../lib/profile-workflow-specification";

	describe("Workflow Profile editor", () => {
		test("treats client-only step ids as unsaved drafts until they exist on the server", () => {
			const persistedStepIds = new Set(["step-1"]);
			expect(
				isUnsavedWorkflowStep(
					{
						title: "Untitled step",
						description: "",
						bindings: [],
					},
					persistedStepIds,
				),
			).toBe(true);
			expect(
				isUnsavedWorkflowStep(
					{
						step_id: "draft-step",
						title: "Untitled step",
						description: "",
						bindings: [],
					},
					persistedStepIds,
				),
			).toBe(true);
			expect(
				isUnsavedWorkflowStep(
					{
						step_id: "step-1",
						title: "Saved step",
						description: "",
						bindings: [],
					},
					persistedStepIds,
				),
			).toBe(false);
			expect(isUnsavedWorkflowStep(undefined, persistedStepIds)).toBe(true);
			expect(createEmptyWorkflowStep().step_id).toEqual(expect.any(String));
		});

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

expect(steps).toHaveLength(2);
			expect(steps[0]?.step_id).toEqual(expect.any(String));
			expect(steps[1]?.step_id).toEqual(expect.any(String));
			expect(steps[0]).toMatchObject({
				title: "Gather context",
				description: null,
				bindings: [
					{ ref_id: "capability-a", binding_policy: "meta_on_demand" },
					{ ref_id: "capability-b", binding_policy: "direct" },
				],
			});
			expect(steps[1]).toMatchObject({
				title: "Respond",
				description: "Use the gathered context.",
				bindings: [],
			});
		});

	test("hydrates nullable descriptions without changing binding order", () => {
		const drafts = workflowDraftFromSpecification({
			profile_id: "profile-workflow",
			specification_revision: 3,
			validation_notes: null,
			avoid_rules: null,
			tool_binding_count: 1,
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
