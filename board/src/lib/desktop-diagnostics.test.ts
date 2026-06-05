import { describe, expect, test } from "bun:test";
import { recordDesktopDiagnosticEvent } from "./desktop-diagnostics";

describe("desktop diagnostics bridge", () => {
	test("records runtime diagnostics through the Tauri shell command", async () => {
		const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];

		const recorded = await recordDesktopDiagnosticEvent(
			{
				level: "info",
				source: "backend-readiness",
				message: "waiting for backend readiness",
				data: { attempt: 2 },
			},
			{
				isTauri: true,
				invoke: async (command, args) => {
					calls.push({ command, args });
				},
			},
		);

		expect(recorded).toBe(true);
		expect(calls).toEqual([
			{
				command: "mcp_shell_record_diagnostic_event",
				args: {
					payload: {
						level: "info",
						source: "backend-readiness",
						message: "waiting for backend readiness",
						data: { attempt: 2 },
					},
				},
			},
		]);
	});

	test("does not call desktop commands outside Tauri", async () => {
		let called = false;

		const recorded = await recordDesktopDiagnosticEvent(
			{
				level: "info",
				source: "backend-readiness",
				message: "waiting for backend readiness",
			},
			{
				isTauri: false,
				invoke: async () => {
					called = true;
				},
			},
		);

		expect(recorded).toBe(false);
		expect(called).toBe(false);
	});

});
