import { afterEach, describe, expect, test } from "bun:test";

import {
	reapplyClientConfigAfterSettingsUpdate,
	resolveClientConfigModeWithDefault,
} from "./client-config-sync";

const originalFetch = globalThis.fetch;

afterEach(() => {
	globalThis.fetch = originalFetch;
});

describe("client config sync", () => {
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
