import { isTauriEnvironmentSync } from "./platform";

type DesktopDiagnosticLevel = "debug" | "info" | "warn" | "error";

export interface DesktopDiagnosticEvent {
	data?: Record<string, unknown>;
	level: DesktopDiagnosticLevel;
	message: string;
	source: string;
}

type DesktopDiagnosticsInvoke = (
	command: string,
	args?: Record<string, unknown>,
) => Promise<unknown>;

interface DesktopDiagnosticsOptions {
	invoke?: DesktopDiagnosticsInvoke;
	isTauri?: boolean;
}

async function defaultInvoke(command: string, args?: Record<string, unknown>): Promise<unknown> {
	const { invoke } = await import("@tauri-apps/api/core");
	return invoke(command, args);
}

export async function recordDesktopDiagnosticEvent(
	event: DesktopDiagnosticEvent,
	options: DesktopDiagnosticsOptions = {},
): Promise<boolean> {
	if (!(options.isTauri ?? isTauriEnvironmentSync())) {
		return false;
	}
	const invoke = options.invoke ?? defaultInvoke;
	await invoke("mcp_shell_record_diagnostic_event", {
		payload: {
			level: event.level,
			source: event.source,
			message: event.message,
			data: event.data ?? {},
		},
	});
	return true;
}
