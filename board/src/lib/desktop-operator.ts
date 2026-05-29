import { isTauriEnvironmentSync } from "./platform";

async function invokeDesktopCommand<T>(
	command: string,
	args?: Record<string, unknown>,
): Promise<T> {
	if (!isTauriEnvironmentSync()) {
		throw new Error("MCPMate Desktop shell is required for this operator action.");
	}
	const { invoke } = await import("@tauri-apps/api/core");
	return invoke<T>(command, args);
}

export async function openFullBoardFromOperator(path?: string): Promise<void> {
	await invokeDesktopCommand<void>("mcp_shell_open_full_board", { path });
}

export async function closeOperatorPanel(): Promise<void> {
	await invokeDesktopCommand<void>("mcp_shell_close_operator_panel");
}

export async function showOperatorPanel(): Promise<void> {
	await invokeDesktopCommand<void>("mcp_shell_show_operator_panel");
}

export async function setOperatorPanelPinned(pinned: boolean): Promise<void> {
	await invokeDesktopCommand<void>("mcp_shell_set_operator_panel_pinned", {
		pinned,
	});
}
