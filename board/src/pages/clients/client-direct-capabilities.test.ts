import { expect, test } from "bun:test";

import {
	getDirectAuthenticationNotice,
	projectDirectCapabilities,
} from "./client-direct-capabilities";

test("preserves successful capability kinds when another kind fails", () => {
	const projection = projectDirectCapabilities(
		{
			tools: {
				items: [
					{
						ref_id: "cref_sha256:tool",
						name: "docs_search",
						unique_name: "docs__search",
					},
				],
				state: "ok",
			},
			resources: {
				items: [],
				state: "failed",
				degraded_reason: "resource inventory failed",
			},
			prompts: { items: [], state: "ok" },
			templates: { items: [], state: "ok" },
		},
		{ tool_refs: ["cref_sha256:tool"] },
		"server-docs",
		"Docs",
	);

	expect(projection.tools).toEqual([
		expect.objectContaining({
			id: "cref_sha256:tool",
			server_id: "server-docs",
			server_name: "Docs",
			tool_name: "docs_search",
			unique_name: "docs__search",
			enabled: true,
		}),
	]);
	expect(projection.failures).toEqual([
		{ kind: "resources", reason: "resource inventory failed" },
	]);
	expect(projection.failureState).toBe("partial");
});

test("classifies a batch where every capability kind fails as complete", () => {
	const failedKind = {
		items: [],
		state: "transport_error",
		degraded_reason: "capability request failed",
	};
	const projection = projectDirectCapabilities(
		{
			tools: failedKind,
			resources: failedKind,
			prompts: failedKind,
			templates: failedKind,
		},
		{},
		"server-offline",
		"Offline",
	);

	expect(projection.failureState).toBe("complete");
});

test("keeps a mixed batch partial when successful kinds are empty", () => {
	const projection = projectDirectCapabilities(
		{
			tools: { items: [], state: "ok" },
			resources: {
				items: [],
				state: "failed",
				degraded_reason: "resource inventory failed",
			},
			prompts: { items: [], state: "ok" },
			templates: { items: [], state: "ok" },
		},
		{},
		"server-empty",
		"Empty",
	);

	expect(projection.failureState).toBe("partial");
});

test("keeps insufficient scope distinct from ordinary authentication", () => {
	expect(
		getDirectAuthenticationNotice({
			code: "insufficient_scope",
			reason: "missing scope",
		})?.titleKey,
	).toBe(
		"servers:detail.capabilityList.authentication.insufficientScope.title",
	);
	expect(
		getDirectAuthenticationNotice({
			code: "auth_required",
			reason: "missing credential",
		})?.titleKey,
	).toBe("servers:detail.capabilityList.authentication.required.title");
});
