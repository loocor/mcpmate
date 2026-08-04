import { describe, expect, test } from "bun:test";

import { getServerListRefetchInterval } from "./server-list-polling";

describe("getServerListRefetchInterval", () => {
	test("polls quickly when an instance is transitioning without a server status", () => {
		expect(
			getServerListRefetchInterval([
				{
					status: undefined,
					instances: [{ status: "Initializing" }],
				},
			]),
		).toBe(5000);
	});

	test("polls quickly when the server status is transitioning", () => {
		expect(
			getServerListRefetchInterval([
				{ status: "starting", instances: [{ status: "Idle" }] },
			]),
		).toBe(5000);
	});

	test("uses the normal interval when every status is stable", () => {
		expect(
			getServerListRefetchInterval([
				{ status: undefined, instances: [{ status: "Ready" }] },
				{ status: "idle", instances: [] },
			]),
		).toBe(30000);
	});
});
