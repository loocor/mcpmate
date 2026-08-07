import { describe, expect, test } from "bun:test";

import { ApiRequestError } from "../../lib/api";
import { handleProfileCapabilityMutationError } from "../../lib/profile-capability-conflict";

describe("Profile capability conflict handling", () => {
	test("invalidates only related capability kinds for catalog dependency conflicts", async () => {
		const queries = [
			["configSuitTools", "profile-a", { source_revision_set: { "server-a": 4 } }],
			["configSuitResources", "profile-a", { source_revision_set: { "server-a": 4 } }],
			["configSuitPrompts", "profile-a", { source_revision_set: { "server-a": 4 } }],
			["configSuitResourceTemplates", "profile-a", { source_revision_set: { "server-a": 4 } }],
			["configSuitTools", "profile-a", { source_revision_set: { "server-z": 9 } }],
			["configSuitTools", "profile-b", { source_revision_set: { "server-a": 4 } }],
		] as const;
		const invalidated: string[] = [];
		const handled = await handleProfileCapabilityMutationError({
			error: new ApiRequestError(
				"Catalog changed",
				409,
				"catalog_dependency_changed",
				{ dependencyServerIds: ["server-a"] },
			),
			profileId: "profile-a",
			invalidateQueries: async (predicate) => {
				for (const [kind, profileId, data] of queries) {
					if (predicate([kind, profileId], data)) {
						invalidated.push(`${kind}:${profileId}`);
					}
				}
			},
		});

		expect(handled).toBeTrue();
		expect(invalidated).toEqual([
			"configSuitTools:profile-a",
			"configSuitResources:profile-a",
			"configSuitPrompts:profile-a",
			"configSuitResourceTemplates:profile-a",
		]);
	});

	test("ignores unrelated errors", async () => {
		let invalidated = false;
		const handled = await handleProfileCapabilityMutationError({
			error: new Error("Request failed"),
			profileId: "profile-a",
			invalidateQueries: async () => {
				invalidated = true;
			},
		});

		expect(handled).toBeFalse();
		expect(invalidated).toBeFalse();
	});
});
