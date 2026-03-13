import { ArrowLeft, BookOpen, MessageSquare, Moon, Sun } from "lucide-react";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";
import { useAppStore } from "../../lib/store";
import { NotificationCenter } from "../notification-center";

const FEEDBACK_EMAIL = "MCPMate Team <info@mcpmate.io>";
const FEEDBACK_SUBJECT = encodeURIComponent("MCPMate preview feedback");
const FEEDBACK_BODY = encodeURIComponent(
	"Hi MCPMate team,\n\nDescribe your feedback here:\n\n\n\n— Sent from MCPMate preview\n",
);
const FEEDBACK_MAILTO = `mailto:${FEEDBACK_EMAIL}?subject=${FEEDBACK_SUBJECT}&body=${FEEDBACK_BODY}`;

const ROUTE_KEYS: Record<string, keyof typeof ROUTE_TRANSLATIONS> = {
	"/": "dashboard",
	"/profiles": "profiles",
	"/clients": "clients",
	"/market": "market",
	"/servers": "servers",
	"/runtime": "runtime",
	"/system": "system",
	"/settings": "settings",
};

const ROUTE_TRANSLATIONS = {
	dashboard: "header.routes.dashboard",
	profiles: "header.routes.profiles",
	clients: "header.routes.clients",
	market: "header.routes.market",
	servers: "header.routes.servers",
	runtime: "header.routes.runtime",
	system: "header.routes.system",
	settings: "header.routes.settings",
} as const;

const ROUTE_FALLBACKS: Record<keyof typeof ROUTE_TRANSLATIONS, string> = {
	dashboard: "Dashboard",
	profiles: "Profiles",
	clients: "Clients",
	market: "Market",
	servers: "Servers",
	runtime: "Runtime",
	system: "System",
	settings: "Settings",
};

const MAIN_ROUTES = Object.keys(ROUTE_KEYS);

export function Header() {
	const location = useLocation();
	const navigate = useNavigate();
	const { theme, setTheme, sidebarOpen } = useAppStore();
	const { t, i18n } = useTranslation();

	const toggleTheme = () => {
		setTheme(theme === "dark" ? "light" : "dark");
	};

	const handleFeedbackClick = useCallback(async () => {
		try {
			if (typeof window !== "undefined" && "__TAURI__" in window) {
				const opener = await import("@tauri-apps/plugin-opener");
				await opener.openUrl(FEEDBACK_MAILTO);
				return;
			}
		} catch (error) {
			console.error("Failed to open feedback email", error);
		}
		if (typeof window !== "undefined") {
			window.open(FEEDBACK_MAILTO, "_blank", "noopener,noreferrer");
		}
	}, []);

	const handleDocsClick = useCallback(() => {
		const lang = i18n.language?.startsWith("zh") ? "zh" : "en";
		const targetUrl = `https://docs.mcpmate.io/i18n/${lang}`;
		if (typeof window !== "undefined") {
			window.open(targetUrl, "_blank", "noopener,noreferrer");
		}
	}, [i18n.language]);

	const isMainRoute = MAIN_ROUTES.includes(location.pathname);
	const routeKey = ROUTE_KEYS[location.pathname];
	const pageTitle = routeKey
		? t(ROUTE_TRANSLATIONS[routeKey], {
				defaultValue: ROUTE_FALLBACKS[routeKey] ?? location.pathname,
			})
		: location.pathname;

	const handleBack = () => {
		navigate(-1);
	};

	return (
		<header
			className={`fixed top-0 right-0 z-30 flex h-16 items-center border-b border-slate-200 bg-white px-4 dark:border-slate-800 dark:bg-slate-950 ${
				sidebarOpen ? "left-64" : "left-16"
			} transition-all duration-300 ease-in-out`}
		>
			<div className="flex w-full items-center justify-between">
				{/* Left side: Sidebar toggle + Page title/Back button */}
				<div className="flex items-center gap-3">
					{/* Page title or Back button */}
					{isMainRoute ? (
						<h1 className="text-xl font-semibold text-slate-900 dark:text-slate-50">
							{pageTitle}
						</h1>
					) : (
						<button
							type="button"
							onClick={handleBack}
							className="flex items-center gap-2 text-slate-600 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100 transition-colors"
						>
							<ArrowLeft className="h-4 w-4" />
							{t("header.back", { defaultValue: "Back" })}
						</button>
					)}
				</div>

				{/* Right side: Theme toggle + Notification center */}
				<div className="flex items-center space-x-4">
					<button
						type="button"
						onClick={handleFeedbackClick}
						className="p-2 text-slate-600 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100 transition-colors"
						aria-label={t("header.sendFeedback", {
							defaultValue: "Send feedback via email",
						})}
					>
						<MessageSquare size={20} />
					</button>
					<button
						type="button"
						onClick={handleDocsClick}
						className="p-2 text-slate-600 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100 transition-colors"
						aria-label={t("header.openDocs", {
							defaultValue: "Open documentation",
						})}
					>
						<BookOpen size={20} />
					</button>
					{/* Theme toggle button */}
					<button
						type="button"
						onClick={toggleTheme}
						aria-label={t("header.toggleTheme", {
							defaultValue: "Toggle theme",
						})}
						className="p-2 text-slate-600 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100 transition-colors"
					>
						{theme === "dark" ? <Sun size={20} /> : <Moon size={20} />}
					</button>

					{/* Notification center */}
					<NotificationCenter />
				</div>
			</div>
		</header>
	);
}
