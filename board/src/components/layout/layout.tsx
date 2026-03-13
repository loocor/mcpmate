import React from "react";
import { useTranslation } from "react-i18next";
import { Outlet, useNavigate } from "react-router-dom";
import { isTauriEnvironmentSync } from "../../lib/platform";
import { useAppStore } from "../../lib/store";
import { Header } from "./header";
import { Sidebar } from "./sidebar";

export function Layout() {
	const { sidebarOpen, theme, setSidebarOpen } = useAppStore();
	const navigate = useNavigate();
	const { t } = useTranslation();

	// Apply theme and react to changes (system/manual)
	React.useEffect(() => {
		const apply = () => {
			const isDark =
				theme === "dark" ||
				(theme === "system" &&
					window.matchMedia("(prefers-color-scheme: dark)").matches);
			document.documentElement.classList.toggle("dark", isDark);

			// Set background color based on theme
			document.body.style.backgroundColor = isDark ? "#0f0f0f" : "#f2f2f2";
		};

		apply();

		let mediaQuery: MediaQueryList | null = null;
		const onChange = (e: MediaQueryListEvent) => {
			if (theme === "system") {
				document.documentElement.classList.toggle("dark", e.matches);
				// Update background color when system theme changes
				document.body.style.backgroundColor = e.matches ? "#0f0f0f" : "#f2f2f2";
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
		let unlistenMain: (() => void) | undefined;
		let unlistenSettings: (() => void) | undefined;
		let cancelled = false;

		const bind = async () => {
			if (!isTauriEnvironmentSync()) {
				return; // Skip binding in web dev/runtime
			}
			try {
				const { listen } = await import("@tauri-apps/api/event");
				if (cancelled) {
					return;
				}
				unlistenMain = await listen("mcpmate://open-main", () => {
					navigate("/");
				});
				unlistenSettings = await listen("mcpmate://open-settings", () => {
					navigate("/settings");
				});
			} catch (error) {
				if (import.meta.env.DEV) {
					console.warn("[Layout] Failed to bind desktop shell events", error);
				}
			}
		};

		void bind();
		return () => {
			cancelled = true;
			if (unlistenMain) {
				void unlistenMain();
			}
			if (unlistenSettings) {
				void unlistenSettings();
			}
		};
	}, [navigate]);

	// Responsive sidebar: auto-collapse on small screens; if it was auto-collapsed,
	// re-open when returning to large screens. Manual toggles remain respected.
	React.useEffect(() => {
		const autoKey = "mcp_sidebar_auto";
		const handler = () => {
			try {
				const w = window.innerWidth;
				if (w < 1200) {
					if (sidebarOpen) setSidebarOpen(false);
					try {
						localStorage.setItem(autoKey, "1");
					} catch {
						/* noop */
					}
					return;
				}
				// Large screens: only auto-open if last close was auto
				let wasAuto = false;
				try {
					wasAuto = localStorage.getItem(autoKey) === "1";
				} catch {
					/* noop */
				}
				if (w >= 1200 && wasAuto && !sidebarOpen) {
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
	}, [sidebarOpen, setSidebarOpen]);

	// Footer labels (Terms / Privacy) localized inline
	const { i18n } = useTranslation();
	const lang = (i18n.language || "").toLowerCase();
	const termsLabel = t("layout.terms", { defaultValue: "Terms" });
	const privacyLabel = t("layout.privacy", { defaultValue: "Privacy" });
	const langParam = lang.startsWith("zh")
		? "zh"
		: lang.startsWith("ja")
			? "ja"
			: "en";
	const termsHref = `https://mcpmate.io/terms?lang=${langParam}`;
	const privacyHref = `https://mcpmate.io/privacy?lang=${langParam}`;

	return (
		<div className="min-h-screen">
			<Sidebar />
			<Header />
			<main
				className={`pt-16 transition-all duration-300 ease-in-out ${
					sidebarOpen ? "ml-64" : "ml-16"
				}`}
			>
				{/* Make main area a flex column that occupies viewport height minus header, so footer can stick to bottom when content is short */}
				<div className="w-full p-4 min-h-[calc(100vh-4rem)] flex flex-col">
					<div className="flex-1">
						<Outlet />
					</div>
					<footer className="mt-6 text-[11px] text-slate-500 border-t border-slate-200 dark:border-slate-900 pt-2 pb-1 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2">
						<div className="flex items-center gap-4 flex-wrap">
							<a
								className="hover:underline"
								href="https://mcpmate.io"
								target="_blank"
								rel="noreferrer"
							>
								{t("layout.copyright", {
									defaultValue: "© 2025 MCPMate",
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
							<a
								className="inline-flex items-center gap-1 hover:underline"
								href="https://forms.gle/zbZxTEJVpoVhpRE58"
								target="_blank"
								rel="noreferrer"
								aria-label={t("layout.feedback", {
									defaultValue: "Feedback Survey",
								})}
								title={t("layout.feedback", {
									defaultValue: "Feedback Survey",
								})}
							>
								{/* Fallback emoji icon to avoid extra imports */}
								<span role="img" aria-hidden="true">
									💬
								</span>
								<span>
									{t("layout.feedback", { defaultValue: "Feedback Survey" })}
								</span>
							</a>
						</div>
					</footer>
				</div>
			</main>
		</div>
	);
}
