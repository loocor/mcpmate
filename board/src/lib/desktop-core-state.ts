import { useCallback, useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { isTauriEnvironmentSync } from "./platform";

export interface DesktopCoreSourceResponse {
	selectedSource: "localhost" | "remote";
	localhostRuntimeMode: "service" | "desktop_managed";
	localhostApiPort: number;
	localhostMcpPort: number;
	remoteBaseUrl: string;
	apiBaseUrl: string;
	localService: {
		status: "not_installed" | "stopped" | "running" | "running_unhealthy";
		label: string;
		detail: string;
		level: string;
		installed: boolean;
		running: boolean;
	};
	remoteAvailable: boolean;
}

type Action = "start" | "stop" | "status" | "install" | "uninstall";

export function useDesktopCoreState() {
	const queryClient = useQueryClient();
	const isTauriShell = isTauriEnvironmentSync();
	const [coreView, setCoreView] = useState<DesktopCoreSourceResponse | null>(null);
	const [busyAction, setBusyAction] = useState<Action | null>(null);

	const invalidateSystemStatus = useCallback(async () => {
		await queryClient.invalidateQueries({ queryKey: ["systemStatus"] });
	}, [queryClient]);

	const refreshCoreView = useCallback(async () => {
		if (!isTauriShell) return null;
		const { invoke } = await import("@tauri-apps/api/core");
		const resp = (await invoke("mcp_shell_read_core_source")) as DesktopCoreSourceResponse;
		setCoreView(resp);
		return resp;
	}, [isTauriShell]);

	const manageLocalCore = useCallback(
		async (action: Action) => {
			if (!isTauriShell) return null;
			try {
				setBusyAction(action);
				const { invoke } = await import("@tauri-apps/api/core");
				const resp = (await invoke("mcp_shell_manage_local_core_service", {
					action,
				})) as DesktopCoreSourceResponse;
				setCoreView(resp);
				await invalidateSystemStatus();
				return resp;
			} finally {
				setBusyAction(null);
			}
		},
		[invalidateSystemStatus, isTauriShell],
	);

	useEffect(() => {
		void refreshCoreView();
	}, [refreshCoreView]);

	useEffect(() => {
		let unlisten: (() => void) | undefined;
		let cancelled = false;

		const bind = async () => {
			if (!isTauriShell) return;
			try {
				const { listen } = await import("@tauri-apps/api/event");
				unlisten = await listen("mcpmate://core/status-changed", (event) => {
					if (cancelled) return;
					setCoreView(event.payload as DesktopCoreSourceResponse);
					void invalidateSystemStatus();
				});
			} catch (error) {
				if (import.meta.env.DEV) {
					console.warn("[DesktopCoreState] Failed to bind core-state listener", error);
				}
			}
		};

		void bind();
		return () => {
			cancelled = true;
			if (unlisten) void unlisten();
		};
	}, [invalidateSystemStatus, isTauriShell]);

	return {
		isTauriShell,
		coreView,
		setCoreView,
		busyAction,
		refreshCoreView,
		manageLocalCore,
	};
}
