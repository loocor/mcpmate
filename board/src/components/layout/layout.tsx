import { useQueryClient } from "@tanstack/react-query";
import React, { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { Outlet, useNavigate } from "react-router-dom";
import { openFeedbackEmail } from "../../lib/feedback-email";
import {
	auditApi,
	configSuitsApi,
	notificationsService,
	serversApi,
	setApiBaseUrl,
	systemApi,
} from "../../lib/api";
import { isTauriEnvironmentSync } from "../../lib/platform";
import { useAppStore } from "../../lib/store";
import { websiteLangParam } from "../../lib/website-lang";
import { Header } from "./header";
import { Sidebar } from "./sidebar";

function applyDesktopApiBaseUrl(apiBaseUrl: string): void {
	setApiBaseUrl(apiBaseUrl);
	notificationsService.reconnectAfterApiBaseChanged();
}

function handleImportServerPayload(navigate: ReturnType<typeof useNavigate>, raw: unknown): void {
	if (!raw || typeof raw !== "object") {
		return;
	}
	const payload = raw as Record<string, unknown>;
	const text = typeof payload.text === "string" ? payload.text : "";
	if (!text.trim()) {
		return;
	}

	const format = typeof payload.format === "string" ? payload.format : undefined;
	const source = typeof payload.source === "string" ? payload.source : undefined;
	useAppStore.getState().setPendingServerDeepLinkImport({
		text,
		format,
		source,
	});
	navigate("/servers");
}

export function Layout() {
	const queryClient = useQueryClient();
	const { sidebarOpen, theme, setSidebarOpen } = useAppStore();
	const navigate = useNavigate();
	const { t, i18n } = useTranslation();
	const [desktopSourceReady, setDesktopSourceReady] = useState(
		() => !isTauriEnvironmentSync(),
	);

	React.useEffect(() => {
		if (!desktopSourceReady) {
			return;
		}
		const staleMs = 30 * 1000;
		void queryClient.prefetchQuery({
			queryKey: ["systemMetrics"],
			queryFn: systemApi.getMetrics,
			staleTime: staleMs,
			retry: false,
		});
		void queryClient.prefetchQuery({
			queryKey: ["configSuits", "dashboard"],
			queryFn: configSuitsApi.getAll,
			staleTime: staleMs,
			retry: false,
		});
		void queryClient.prefetchQuery({
			queryKey: ["servers"],
			queryFn: serversApi.getAll,
			staleTime: staleMs,
			retry: false,
		});
		void queryClient.prefetchQuery({
			queryKey: ["audit", "mcp-calls-stats"],
			queryFn: () => auditApi.list({ limit: 1000 }),
			staleTime: staleMs,
			retry: false,
		});
	}, [queryClient, desktopSourceReady]);

	// Apply theme and react to changes (system/manual)
	React.useEffect(() => {
		const apply = () => {
			const isDark =
				theme === "dark" ||
				(theme === "system" &&
					window.matchMedia("(prefers-color-scheme: dark)").matches);
			document.documentElement.classList.toggle("dark", isDark);
		};

		apply();

		let mediaQuery: MediaQueryList | null = null;
		const onChange = (e: MediaQueryListEvent) => {
			if (theme === "system") {
				document.documentElement.classList.toggle("dark", e.matches);
			}
		};
		if (theme === "system") {
			mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
			mediaQuery.addEventListener("change", onChange);
		}

		return () => {
			if (mediaQuery) mediaQuery.removeEventListener("change", onChange);
		};
	}, [theme]);

	React.useEffect(() => {
		let cancelled = false;

		const syncDesktopCoreSource = async () => {
			if (!isTauriEnvironmentSync()) {
				setDesktopSourceReady(true);
				return;
			}

			try {
				const { invoke } = await import("@tauri-apps/api/core");
				const source = (await invoke("mcp_shell_read_core_source")) as {
					apiBaseUrl?: string;
				};
				if (!cancelled && typeof source.apiBaseUrl === "string") {
					applyDesktopApiBaseUrl(source.apiBaseUrl);
				}
			} catch (error) {
				if (import.meta.env.DEV) {
					console.warn("[Layout] Failed to resolve desktop core source", error);
				}
			} finally {
				if (!cancelled) {
					setDesktopSourceReady(true);
				}
			}
		};

		void syncDesktopCoreSource();

		return () => {
			cancelled = true;
		};
	}, []);

	React.useEffect(() => {
		let unlistenSettings: (() => void) | undefined;
		let unlistenImportServer: (() => void) | undefined;
		let unlistenCoreState: (() => void) | undefined;
		let cancelled = false;

		const bind = async () => {
			if (!isTauriEnvironmentSync()) {
				return; // Skip binding in web dev/runtime
			}
			try {
				const { listen } = await import("@tauri-apps/api/event");
				const { invoke } = await import("@tauri-apps/api/core");
				if (cancelled) {
					return;
				}
				unlistenSettings = await listen(
					"mcpmate://open-settings",
					(event) => {
						const payload =
							event.payload && typeof event.payload === "object"
								? (event.payload as { tab?: string })
								: undefined;
						const target = payload?.tab ? `/settings?tab=${payload.tab}` : "/settings";
						navigate(target);
					},
				);
				unlistenImportServer = await listen("mcp-import/server", (event) => {
					handleImportServerPayload(navigate, event.payload);
				});
				unlistenCoreState = await listen("mcpmate://core/status-changed", (event) => {
					const payload =
						event.payload && typeof event.payload === "object"
							? (event.payload as { apiBaseUrl?: string })
							: undefined;
					if (typeof payload?.apiBaseUrl === "string") {
						applyDesktopApiBaseUrl(payload.apiBaseUrl);
					}
				});

				const pending = await invoke<unknown>(
					"mcp_deep_link_take_pending_server_import",
				);
				handleImportServerPayload(navigate, pending);
			} catch (error) {
				if (import.meta.env.DEV) {
					console.warn("[Layout] Failed to bind desktop shell events", error);
				}
			}
		};

		void bind();
		return () => {
			cancelled = true;
			if (unlistenSettings) {
				void unlistenSettings();
			}
			if (unlistenImportServer) {
				void unlistenImportServer();
			}
			if (unlistenCoreState) {
				void unlistenCoreState();
			}
		};
	}, [navigate]);

	const sidebarOpenRef = React.useRef(sidebarOpen);
	React.useEffect(() => {
		sidebarOpenRef.current = sidebarOpen;
	}, [sidebarOpen]);

	// Responsive sidebar: auto-collapse once on entering narrow widths, and allow
	// manual expand while narrow. Re-open on wide screens only if last close was auto.
	React.useEffect(() => {
		const autoKey = "mcp_sidebar_auto";
		const BREAKPOINT = 1200;
		const handler = () => {
			try {
				const w = window.innerWidth;
				if (w < BREAKPOINT) {
					if (sidebarOpenRef.current) {
						setSidebarOpen(false);
						try {
							localStorage.setItem(autoKey, "1");
						} catch {
							/* noop */
						}
					}
					return;
				}
				// Wide screens: only auto-open if last close was auto
				let wasAuto = false;
				try {
					wasAuto = localStorage.getItem(autoKey) === "1";
				} catch {
					/* noop */
				}
				if (w >= BREAKPOINT && wasAuto && !sidebarOpenRef.current) {
					setSidebarOpen(true);
					try {
						localStorage.removeItem(autoKey);
					} catch {
						/* noop */
					}
				}
			} catch {
				/* noop */
			}
		};
		handler();
		window.addEventListener("resize", handler);
		return () => window.removeEventListener("resize", handler);
	}, [setSidebarOpen]);

	const termsLabel = t("layout.terms", { defaultValue: "Terms" });
	const privacyLabel = t("layout.privacy", { defaultValue: "Privacy" });
	const langParam = websiteLangParam(i18n.language);
	const termsHref = `https://mcp.umate.ai/terms?lang=${langParam}`;
	const privacyHref = `https://mcp.umate.ai/privacy?lang=${langParam}`;

	const handleFooterFeedbackClick = useCallback(() => {
		void openFeedbackEmail();
	}, []);

	if (!desktopSourceReady) {
		return <div className="min-h-screen bg-background" />;
	}

	return (
		<div className="h-screen flex flex-col overflow-hidden">
			<Sidebar />
			<Header />
			<main
				className={`flex-1 min-h-0 min-w-0 pt-16 transition-all duration-300 ease-in-out ${sidebarOpen ? "ml-64" : "ml-16"
					}`}
			>
				{/* Viewport-height column: outlet fills space above footer; pages can use h-full + inner scroll */}
				<div className="box-border flex h-full w-full min-w-0 flex-col overflow-hidden p-4">
					<div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-y-auto">
						<Outlet />
					</div>
					<footer className="mt-6 shrink-0 text-[11px] text-slate-500 border-t border-slate-200 dark:border-slate-700 pt-2 pb-1 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2">
						<div className="flex items-center gap-4 flex-wrap">
							<a
								className="hover:underline"
								href="https://mcp.umate.ai"
								target="_blank"
								rel="noreferrer"
							>
								{t("layout.copyright", {
									defaultValue: "© 2026 MCPMate",
								})}
							</a>
							<div className="flex items-center gap-3">
								<a
									className="hover:underline"
									href={termsHref}
									target="_blank"
									rel="noreferrer"
								>
									{termsLabel}
								</a>
								<span className="text-slate-300">•</span>
								<a
									className="hover:underline"
									href={privacyHref}
									target="_blank"
									rel="noreferrer"
								>
									{privacyLabel}
								</a>
							</div>
						</div>
						<div className="flex items-center gap-2">
							<button
								type="button"
								className="inline-flex items-center gap-1 hover:underline text-inherit p-0 border-0 bg-transparent cursor-pointer"
								onClick={handleFooterFeedbackClick}
								aria-label={t("header.sendFeedback", {
									defaultValue: "Send feedback via email",
								})}
								title={t("header.sendFeedback", {
									defaultValue: "Send feedback via email",
								})}
							>
								{/* Fallback emoji icon to avoid extra imports */}
								<span role="img" aria-hidden="true">
									💬
								</span>
								<span>{t("layout.feedback", { defaultValue: "Feedback" })}</span>
							</button>
						</div>
					</footer>
				</div>
			</main>
		</div>
	);
}
