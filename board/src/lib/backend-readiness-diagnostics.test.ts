import { describe, expect, test } from "bun:test";
import {
	backendReadinessStatusKey,
	shouldReportBackendReadinessAttempt,
} from "./backend-readiness-diagnostics";

describe("backend readiness diagnostics", () => {
	test("reports the first attempt and suppresses repeated waits before the throttle window", () => {
		const first = shouldReportBackendReadinessAttempt({
			attempt: 1,
			lastReportedAtMs: null,
			lastStatusKey: null,
			nowMs: 1_000,
			statusKey: "network_error",
		});
		expect(first).toBe(true);

		const repeated = shouldReportBackendReadinessAttempt({
			attempt: 2,
			lastReportedAtMs: 1_000,
			lastStatusKey: "network_error",
			nowMs: 1_500,
			statusKey: "network_error",
		});
		expect(repeated).toBe(false);
	});

	test("reports status changes and throttled summaries", () => {
		const changed = shouldReportBackendReadinessAttempt({
			attempt: 3,
			lastReportedAtMs: 1_000,
			lastStatusKey: "network_error",
			nowMs: 2_000,
			statusKey: "not_ready",
		});
		expect(changed).toBe(true);

		const throttled = shouldReportBackendReadinessAttempt({
			attempt: 40,
			lastReportedAtMs: 1_000,
			lastStatusKey: "not_ready",
			nowMs: 31_000,
			statusKey: "not_ready",
		});
		expect(throttled).toBe(true);
	});

	test("normalizes readiness payloads and errors into stable status keys", () => {
		expect(backendReadinessStatusKey({ type: "ready", status: "ok" }, null)).toBe(
			"ready:ok",
		);
		expect(
			backendReadinessStatusKey({ type: "starting", status: "pending" }, null),
		).toBe("starting:pending");
		expect(backendReadinessStatusKey(null, new TypeError("fetch failed"))).toBe(
			"network_error",
		);
		expect(backendReadinessStatusKey(null, new Error("boom"))).toBe("error");
	});
});
