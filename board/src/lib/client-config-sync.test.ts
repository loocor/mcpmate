import { afterEach, describe, expect, test } from "bun:test";

import {
	attachCreatedClientIfEligible,
	reapplyClientConfigAfterSettingsUpdate,
	resolveClientConfigModeWithDefault,
} from "./client-config-sync";

const originalFetch = globalThis.fetch;

afterEach(() => {
	globalThis.fetch = originalFetch;
});

describe("client config sync", () => {
	test("attaches a newly created client when authoritative config details are eligible", async () => {
		const requests: Array<{ input: string; init?: RequestInit }> = [];
		globalThis.fetch = (async (input, init): Promise<Response> => {
			requests.push({ input: String(input), init });
			if (String(input).includes("/api/client/config/details?")) {
				return new Response(
					JSON.stringify({
						success: true,
						data: {
							writable_config: true,
							attachment_state: "detached",
							approval_status: "approved",
						},
					}),
					{ headers: { "content-type": "application/json" } },
				);
			}
			return new Response(
				JSON.stringify({
					success: true,
					data: {
						identifier: "zed",
						attachment_state: "attached",
						changed: true,
						warnings: [],
					},
				}),
				{ headers: { "content-type": "application/json" } },
			);
		}) as typeof fetch;

		const attached = await attachCreatedClientIfEligible({
			identifier: "zed",
			created: true,
		});

		expect(attached).toBe(true);
		expect(requests).toHaveLength(2);
		expect(requests[0]?.input).toContain("/api/client/config/details?");
		expect(requests[1]?.input).toEndWith("/api/client/attach");
		expect(requests[1]?.init?.method).toBe("POST");
		expect(JSON.parse(String(requests[1]?.init?.body))).toEqual({
			identifier: "zed",
		});
	});

	test("skips config inspection and attach when the update did not create a client", async () => {
		let requestCount = 0;
		globalThis.fetch = (async (input, init): Promise<Response> => {
			void input;
			void init;
			requestCount += 1;
			throw new Error("unexpected request");
		}) as typeof fetch;

		const attached = await attachCreatedClientIfEligible({
			identifier: "manual.client",
			created: false,
		});

		expect(attached).toBe(false);
		expect(requestCount).toBe(0);
	});

	for (const details of [
		{ writable_config: false, attachment_state: "detached", approval_status: "approved" },
		{ writable_config: true, attachment_state: "not_applicable", approval_status: "approved" },
		{ writable_config: true, attachment_state: "detached", approval_status: "pending" },
	]) {
		test(`skips attach for ineligible authoritative details ${JSON.stringify(details)}`, async () => {
			const requests: string[] = [];
			globalThis.fetch = (async (input): Promise<Response> => {
				requests.push(String(input));
				return new Response(JSON.stringify({ success: true, data: details }), {
					headers: { "content-type": "application/json" },
				});
			}) as typeof fetch;

			const attached = await attachCreatedClientIfEligible({
				identifier: "ineligible.client",
				created: true,
			});

			expect(attached).toBe(false);
			expect(requests).toHaveLength(1);
			expect(requests[0]).toContain("/api/client/config/details?");
		});
	}

	test("uses the dashboard default when the client mode is not persisted", () => {
		expect(resolveClientConfigModeWithDefault(null, "unify")).toBe("unify");
	});

	test("re-applies inherited default modes after client settings save", async () => {
		const requests: Array<{ input: string; init?: RequestInit }> = [];
		globalThis.fetch = (async (input, init): Promise<Response> => {
			requests.push({ input: String(input), init });
			if (String(input).includes("/api/client/capability-config?")) {
				return new Response(
					JSON.stringify({
						success: true,
						data: {
							identifier: "zed",
							capability_source: "activated",
							selected_profile_ids: [],
							source_revision_set: {},
						},
					}),
					{ headers: { "content-type": "application/json" } },
				);
			}
			if (String(input).endsWith("/api/client/config/apply")) {
				return new Response(
					JSON.stringify({ success: true, data: { applied: true } }),
					{ headers: { "content-type": "application/json" } },
				);
			}
			throw new Error(`unexpected request: ${String(input)}`);
		}) as typeof fetch;

		for (const defaultMode of ["unify", "hosted", "transparent"] as const) {
			await reapplyClientConfigAfterSettingsUpdate({
				identifier: "zed",
				configMode: null,
				defaultMode,
				writableConfig: true,
				approvalStatus: "approved",
			});
		}

		const applyPayloads = requests
			.filter(({ input }) => input.endsWith("/api/client/config/apply"))
			.map(({ init }) => JSON.parse(String(init?.body)));
		expect(applyPayloads).toEqual([
			{ identifier: "zed", mode: "unify", selected_config: "default", preview: false },
			{ identifier: "zed", mode: "hosted", selected_config: "default", preview: false },
			{ identifier: "zed", mode: "transparent", selected_config: "default", preview: false },
		]);
	});

	test("does not re-apply an invalid persisted mode", async () => {
		let requestCount = 0;
		globalThis.fetch = (async (input, init): Promise<Response> => {
			void input;
			void init;
			requestCount += 1;
			throw new Error("unexpected request");
		}) as typeof fetch;

		const applied = await reapplyClientConfigAfterSettingsUpdate({
			identifier: "zed",
			configMode: "invalid",
			defaultMode: "unify",
			writableConfig: true,
			approvalStatus: "approved",
		});

		expect(applied).toBe(false);
		expect(requestCount).toBe(0);
	});
});
