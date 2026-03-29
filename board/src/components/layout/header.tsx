import { ArrowLeft, BookOpen, MessageSquare, Moon, Sun } from "lucide-react";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";
import { openFeedbackEmail } from "../../lib/feedback-email";
import { useAppStore } from "../../lib/store";
import { websiteDocsLocale } from "../../lib/website-lang";
import { NotificationCenter } from "../notification-center";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "../ui/tooltip";

/** Public marketing site; doc paths mirror `website/src/docs/nav.ts` (`/docs/:locale/:page`). */
const WEBSITE_DOCS_ORIGIN = "https://mcp.umate.ai";

/**
 * Map Board routes to website guide slugs (see `website/src/docs/nav.ts` Guides section).
 */
export function boardPathToDocsPage(pathname: string, search = ""): string {
  const params = new URLSearchParams(search);
  const tab = params.get("tab") ?? "overview";
  const view = params.get("view") ?? "browse";

  if (pathname === "/" || pathname === "") return "dashboard";

  if (pathname.startsWith("/profiles/presets/")) {
    return "profile-presets";
  }

  if (pathname.startsWith("/profiles/") || pathname.startsWith("/config/")) {
    if (tab === "overview") return "profile-detail-overview";
    if (
      ["servers", "tools", "prompts", "resources", "templates"].includes(tab)
    ) {
      return `profile-capabilities#${tab}`;
    }
    return "profile-detail-overview";
  }

  if (pathname.startsWith("/profiles") || pathname.startsWith("/config")) {
    return "profile";
  }

  if (pathname.startsWith("/clients/")) {
    if (tab === "configuration") return "client-configuration";
    if (tab === "backups") return "client-backups";
    return "client-detail-overview";
  }

  if (pathname.startsWith("/clients")) return "clients";
  if (pathname.startsWith("/market")) return "market";

  if (pathname.startsWith("/servers/") && pathname.includes("/instances/")) {
    return "server-instances";
  }

  if (pathname.startsWith("/servers/")) {
    if (view === "debug") {
      if (["tools", "prompts", "resources", "templates"].includes(tab)) {
        return `server-inspector#${tab}`;
      }
      return "server-inspector";
    }
    if (["tools", "prompts", "resources", "templates"].includes(tab)) {
      return `server-capabilities#${tab}`;
    }
    return "server-detail-overview";
  }

  if (pathname.startsWith("/servers")) return "servers";
  if (pathname.startsWith("/runtime") || pathname.startsWith("/system"))
    return "runtime";
  if (pathname.startsWith("/audit")) return "logs";
  if (pathname.startsWith("/api-docs")) return "api-docs";
  if (pathname.startsWith("/settings")) return "settings";
  if (pathname.startsWith("/account")) return "quickstart";
  if (pathname.startsWith("/404")) return "guides-overview";
  return "guides-overview";
}

const ROUTE_TRANSLATIONS = {
  dashboard: "header.routes.dashboard",
  profiles: "header.routes.profiles",
  clients: "header.routes.clients",
  market: "header.routes.market",
  servers: "header.routes.servers",
  runtime: "header.routes.runtime",
  audit: "header.routes.audit",
  apiDocs: "header.routes.apiDocs",
  system: "header.routes.system",
  settings: "header.routes.settings",
} as const;

const ROUTE_KEYS: Record<string, keyof typeof ROUTE_TRANSLATIONS> = {
  "/": "dashboard",
  "/profiles": "profiles",
  "/clients": "clients",
  "/market": "market",
  "/servers": "servers",
  "/runtime": "runtime",
  "/audit": "audit",
  "/api-docs": "apiDocs",
  "/system": "system",
  "/settings": "settings",
};

const ROUTE_FALLBACKS: Record<keyof typeof ROUTE_TRANSLATIONS, string> = {
  dashboard: "Dashboard",
  profiles: "Profiles",
  clients: "Clients",
  market: "Market",
  servers: "Servers",
  runtime: "Runtime",
  audit: "Audit",
  apiDocs: "API Docs",
  system: "System",
  settings: "Settings",
};

const MAIN_ROUTES = Object.keys(ROUTE_KEYS);

export function Header() {
  const location = useLocation();
  const navigate = useNavigate();
  const { theme, setTheme, sidebarOpen } = useAppStore();
  const { t, i18n } = useTranslation();

  const toggleTheme = useCallback(() => {
    setTheme(theme === "dark" ? "light" : "dark");
  }, [theme, setTheme]);

  const handleFeedbackClick = useCallback(() => openFeedbackEmail(), []);

  const handleDocsClick = useCallback(() => {
    const locale = websiteDocsLocale(i18n.language);
    const page = boardPathToDocsPage(location.pathname, location.search);
    const targetUrl = `${WEBSITE_DOCS_ORIGIN}/docs/${locale}/${page}`;
    if (typeof window !== "undefined") {
      window.open(targetUrl, "_blank", "noopener,noreferrer");
    }
  }, [i18n.language, location.pathname, location.search]);

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

  const feedbackLabel = t("header.sendFeedback", {
    defaultValue: "Send feedback via email",
  });
  const docsLabel = t("header.openDocs", {
    defaultValue: "Open documentation",
  });
  const themeLabel = t("header.toggleTheme", {
    defaultValue: "Toggle theme",
  });

  return (
    <header
      className={`fixed top-0 right-0 z-30 flex h-16 items-center border-b border-border bg-card px-4 ${
        sidebarOpen ? "left-64" : "left-16"
      } transition-all duration-300 ease-in-out`}
    >
      <div className="flex w-full items-center justify-between">
        {/* Left side: Sidebar toggle + Page title/Back button */}
        <div className="flex items-center gap-3">
          {/* Page title or Back button */}
          {isMainRoute ? (
            <h1 className="text-xl font-semibold text-foreground">
              {pageTitle}
            </h1>
          ) : (
            <button
              type="button"
              onClick={handleBack}
              className="flex items-center gap-2 text-muted-foreground hover:text-foreground transition-colors"
            >
              <ArrowLeft className="h-4 w-4" />
              {t("header.back", { defaultValue: "Back" })}
            </button>
          )}
        </div>

        {/* Right side: Theme toggle + Notification center */}
        <TooltipProvider delayDuration={400}>
          <div className="flex items-center space-x-4">
            <Tooltip>
              <TooltipTrigger asChild>
                <button
                  type="button"
                  onClick={handleFeedbackClick}
                  className="p-2 text-muted-foreground hover:text-foreground transition-colors"
                  aria-label={feedbackLabel}
                >
                  <MessageSquare size={20} />
                </button>
              </TooltipTrigger>
              <TooltipContent side="bottom">{feedbackLabel}</TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger asChild>
                <button
                  type="button"
                  onClick={handleDocsClick}
                  className="p-2 text-muted-foreground hover:text-foreground transition-colors"
                  aria-label={docsLabel}
                >
                  <BookOpen size={20} />
                </button>
              </TooltipTrigger>
              <TooltipContent side="bottom">{docsLabel}</TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger asChild>
                <button
                  type="button"
                  onClick={toggleTheme}
                  aria-label={themeLabel}
                  className="p-2 text-muted-foreground hover:text-foreground transition-colors"
                >
                  {theme === "dark" ? <Sun size={20} /> : <Moon size={20} />}
                </button>
              </TooltipTrigger>
              <TooltipContent side="bottom">{themeLabel}</TooltipContent>
            </Tooltip>

            <NotificationCenter />
          </div>
        </TooltipProvider>
      </div>
    </header>
  );
}
