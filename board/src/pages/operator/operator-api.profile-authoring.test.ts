import { afterEach, expect, test } from "bun:test";

import { listOperatorProfiles } from "./operator-api";

const originalFetch = globalThis.fetch;

afterEach(() => {
	globalThis.fetch = originalFetch;
});

test("preserves Profile authoring generations in the operator list", async () => {
	globalThis.fetch = async () =>
		Response.json({
			success: true,
			data: {
				profile: [
					{
						id: "profile-a",
						name: "Profile A",
						description: null,
						profile_type: "shared",
						priority: 50,
						is_active: true,
						is_default: false,
						role: "user",
						authoring_generation: 9,
						allowed_operations: ["deactivate", "update", "delete"],
					},
				],
				total: 1,
				timestamp: "2026-08-07T00:00:00Z",
			},
		});

	const result = await listOperatorProfiles();

	expect(result).toEqual({
		suits: [
			{
				id: "profile-a",
				name: "Profile A",
				description: undefined,
				suit_type: "shared",
				priority: 50,
				is_active: true,
				is_default: false,
				role: "user",
				authoring_generation: 9,
				allowed_operations: ["deactivate", "update", "delete"],
			},
		],
	});
});
