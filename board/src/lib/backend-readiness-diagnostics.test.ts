import { describe, expect, test } from "bun:test";
import {
	backendReadinessStatusKey,
	describeCoreStartupIssue,
	describeBackendReadinessIssue,
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
		expect(backendReadinessStatusKey(null, new Error("Backend is starting"))).toBe(
			"backend_starting",
		);
		expect(backendReadinessStatusKey(null, new Error("boom"))).toBe("error");
	});

	test("describes network readiness failures with the current api base", () => {
		expect(
			describeBackendReadinessIssue(
				null,
				new TypeError("fetch failed"),
				"http://127.0.0.1:8080",
			),
		).toMatchObject({
			kind: "network_error",
			statusKey: "network_error",
			detail: "Backend API is not reachable at http://127.0.0.1:8080: fetch failed",
			messageKey: "networkError",
			messageParams: {
				error: "fetch failed",
				target: " at http://127.0.0.1:8080",
			},
		});
	});

	test("describes backend-starting proxy responses as startup state", () => {
		expect(
			describeBackendReadinessIssue(
				null,
				new Error("Backend is starting"),
				"http://127.0.0.1:8081",
			),
		).toMatchObject({
			kind: "backend_starting",
			statusKey: "backend_starting",
			detail: "Backend process is starting at http://127.0.0.1:8081",
			messageKey: "backendStarting",
			messageParams: {
				target: " at http://127.0.0.1:8081",
			},
		});
	});

	test("describes non-ready payloads without treating them as fatal", () => {
		expect(
			describeBackendReadinessIssue(
				{ type: "starting", status: "pending", reason: "database_setup_failed" },
				null,
			),
		).toMatchObject({
			kind: "not_ready",
			statusKey: "starting:pending",
			detail: "Backend readiness is starting:pending (database_setup_failed)",
			messageKey: "notReady",
			messageParams: {
				reason: " (database_setup_failed)",
				statusKey: "starting:pending",
			},
		});
	});

	test("omits issues for ready payloads", () => {
		expect(
			describeBackendReadinessIssue(
				{ type: "ready", status: "ok" },
				null,
				"http://127.0.0.1:8080",
			),
		).toBeNull();
	});

	test("describes stopped desktop core source state", () => {
		expect(
			describeCoreStartupIssue({
				localService: {
					status: "stopped",
					label: "Stopped",
					detail: "The localhost core is currently stopped.",
					running: false,
				},
			}),
		).toMatchObject({
			kind: "core_stopped",
			statusKey: "core:stopped",
			detail: "Stopped: The localhost core is currently stopped.",
			messageKey: "coreService",
			messageParams: {
				detail: "The localhost core is currently stopped.",
				label: "Stopped: ",
			},
		});
	});

	test("describes unhealthy desktop core source state", () => {
		expect(
			describeCoreStartupIssue({
				localService: {
					status: "running_unhealthy",
					label: "Running (Unhealthy)",
					detail: "The API health check is failing.",
					running: true,
				},
			}),
		).toMatchObject({
			kind: "core_unhealthy",
			statusKey: "core:running_unhealthy",
			detail: "Running (Unhealthy): The API health check is failing.",
			messageKey: "coreService",
			messageParams: {
				detail: "The API health check is failing.",
				label: "Running (Unhealthy): ",
			},
		});
	});

	test("omits running desktop core source state", () => {
		expect(
			describeCoreStartupIssue({
				localService: {
					status: "running",
					label: "Running",
					detail: "The API health check is passing.",
					running: true,
				},
			}),
		).toBeNull();
	});
});
