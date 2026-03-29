import {
  Activity,
  AppWindow,
  Bug,
  FileSearch,
  LayoutDashboard,
  Menu,
  Server,
  Settings,
  Sliders,
  Store,
} from "lucide-react";
import type React from "react";
import { useTranslation } from "react-i18next";
import { NavLink } from "react-router-dom";
import { useAppStore } from "../../lib/store";
import { cn } from "../../lib/utils";
import { Button } from "../ui/button";
import { AccountSessionDialog } from "./account-session-dialog";

interface SidebarLinkProps {
  to: string;
  icon: React.ReactNode;
  children: React.ReactNode;
}

function SidebarLink({ to, icon, children }: SidebarLinkProps) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        cn(
          "flex items-center px-3 py-2 text-sm font-medium rounded-md transition-colors",
          "hover:bg-slate-200 dark:hover:bg-slate-800",
          isActive
            ? "bg-slate-200 text-slate-900 dark:bg-slate-800 dark:text-slate-50"
            : "text-slate-700 dark:text-slate-400",
        )
      }
    >
      <span className="mr-3 h-5 w-5">{icon}</span>
      <span>{children}</span>
    </NavLink>
  );
}

export function Sidebar() {
  const sidebarOpen = useAppStore((state) => state.sidebarOpen);
  const toggleSidebar = useAppStore((state) => state.toggleSidebar);
  const showApiDocsMenu = useAppStore(
    (state) => state.dashboardSettings.showApiDocsMenu,
  );
  const { t } = useTranslation();

  return (
    <div
      className={cn(
        "fixed inset-y-0 left-0 z-40 flex flex-col transition-all duration-300 ease-in-out",
        "border-r border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-900",
        sidebarOpen ? "w-64" : "w-16",
      )}
    >
      <div className="flex h-16 items-center justify-between px-4">
        <div
          className={cn(
            "flex items-center gap-2 w-full",
            sidebarOpen ? "justify-between" : "justify-center",
          )}
        >
          {sidebarOpen ? (
            <>
              {/* Brand: show logo + title when expanded */}
				<img
					src="/logo.svg"
                alt="MCPMate"
                className={cn(
                  "h-6 w-6 object-contain transition",
                  "dark:invert dark:brightness-0",
                )}
              />
              <span className="font-bold text-xl dark:text-white">
                {t("layout.brand", { defaultValue: "MCPMate" })}{" "}
                <sup className="text-[9px] text-slate-500 dark:text-slate-400">
                  {t("layout.alpha", { defaultValue: "Beta" })}
                </sup>
              </span>
              <Button
                variant="ghost"
                size="icon"
                onClick={toggleSidebar}
                className="ml-auto -mr-2 h-9 w-9 shrink-0 rounded-md text-slate-600 hover:bg-slate-100 hover:text-slate-900 dark:text-slate-300 dark:hover:bg-slate-800 dark:hover:text-slate-50"
                aria-label={t("layout.collapseSidebar", {
                  defaultValue: "Collapse sidebar",
                })}
              >
                <Menu size={18} />
              </Button>
            </>
          ) : (
            <button
              type="button"
              onClick={toggleSidebar}
              className="flex h-9 w-9 items-center justify-center rounded-md transition-colors hover:bg-slate-100 dark:hover:bg-slate-800"
              aria-label={t("layout.expandSidebar", {
                defaultValue: "Expand sidebar",
              })}
            >
				<img
					src="/logo.svg"
                alt="MCPMate"
                className="h-6 w-6 object-contain dark:invert dark:brightness-0"
              />
            </button>
          )}
        </div>
      </div>

      <div className="flex flex-col flex-1 gap-1 px-2 py-4">
        <div className={cn("flex", !sidebarOpen && "justify-center")}>
          {sidebarOpen ? (
            <span className="px-3 text-xs font-semibold text-slate-500 dark:text-slate-400 mb-1">
              {t("nav.main", { defaultValue: "MAIN" })}
            </span>
          ) : null}
        </div>

        <SidebarLink to="/" icon={<LayoutDashboard size={20} />}>
          {sidebarOpen && t("nav.dashboard", { defaultValue: "Dashboard" })}
        </SidebarLink>

        <SidebarLink to="/profiles" icon={<Sliders size={20} />}>
          {sidebarOpen && t("nav.profiles", { defaultValue: "Profiles" })}
        </SidebarLink>

        <SidebarLink to="/clients" icon={<AppWindow size={20} />}>
          {sidebarOpen && t("nav.clients", { defaultValue: "Clients" })}
        </SidebarLink>

        <SidebarLink to="/servers" icon={<Server size={20} />}>
          {sidebarOpen && t("nav.servers", { defaultValue: "Servers" })}
        </SidebarLink>

        <SidebarLink to="/market" icon={<Store size={20} />}>
          {sidebarOpen && t("nav.market", { defaultValue: "Market" })}
        </SidebarLink>

        {/* Tools removed per feedback */}

        <div className={cn("flex mt-4", !sidebarOpen && "justify-center")}>
          {sidebarOpen ? (
            <span className="px-3 text-xs font-semibold text-slate-500 dark:text-slate-400 mb-1">
              {t("nav.developer", { defaultValue: "DEVELOPER" })}
            </span>
          ) : null}
        </div>

        <SidebarLink to="/audit" icon={<FileSearch size={20} />}>
          {sidebarOpen && t("nav.audit", { defaultValue: "Audit" })}
        </SidebarLink>

        <SidebarLink to="/runtime" icon={<Activity size={20} />}>
          {sidebarOpen && t("nav.runtime", { defaultValue: "Runtime" })}
        </SidebarLink>

        {showApiDocsMenu && (
          <SidebarLink to="/api-docs" icon={<Bug size={20} />}>
            {sidebarOpen && t("nav.apiDocs", { defaultValue: "API Docs" })}
          </SidebarLink>
        )}

        <div className="mt-auto flex flex-col gap-1">
          <AccountSessionDialog sidebarOpen={sidebarOpen} />

          <SidebarLink to="/settings" icon={<Settings size={20} />}>
            {sidebarOpen && t("nav.settings", { defaultValue: "Settings" })}
          </SidebarLink>
        </div>
      </div>
    </div>
  );
}
