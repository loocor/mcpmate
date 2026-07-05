import {
  Activity,
  AppWindow,
  Bug,
  FileSearch,
  KeyRound,
  LayoutDashboard,
  Menu,
  Microscope,
  Server,
  Settings,
  Sliders,
  Store,
} from "lucide-react";
import type React from "react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { NavLink, useLocation } from "react-router-dom";
import { resolveMarketListHref } from "../../pages/market/market-list-pagination-storage";
import {
  openInspectorWindow,
  shouldOpenInspectorInSameTab,
} from "../../lib/open-inspector-window";
import { useAppStore } from "../../lib/store";
import { cn } from "../../lib/utils";
import { Button } from "../ui/button";
import { AccountSessionDialog } from "./account-session-dialog";
import { SidebarBrandBadge } from "./sidebar-brand-badge";
import {
  SidebarNavIcon,
  SidebarSectionLabelSlot,
  sidebarBodyClassName,
  sidebarFooterClassName,
  sidebarHeaderBrandClassName,
  sidebarHeaderClassName,
  sidebarLogoToggleClassName,
  sidebarNavItemClassName,
} from "./sidebar-nav-item";

interface SidebarLinkProps {
  to: string;
  icon: React.ReactNode;
  children: React.ReactNode;
  sidebarOpen: boolean;
}

function SidebarLink({ to, icon, children, sidebarOpen }: SidebarLinkProps) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        sidebarNavItemClassName(sidebarOpen, { active: isActive })
      }
    >
      <SidebarNavIcon sidebarOpen={sidebarOpen}>{icon}</SidebarNavIcon>
      {sidebarOpen ? <span>{children}</span> : null}
    </NavLink>
  );
}

interface SidebarInspectorLinkProps {
  icon: React.ReactNode;
  children: React.ReactNode;
  sidebarOpen: boolean;
}

function SidebarInspectorLink({
  icon,
  children,
  sidebarOpen,
}: SidebarInspectorLinkProps) {
  const { t } = useTranslation();

  return (
    <a
      href="/inspector"
      className={sidebarNavItemClassName(sidebarOpen)}
      aria-label={t("nav.inspector", { defaultValue: "Inspector" })}
      onClick={(event) => {
        if (shouldOpenInspectorInSameTab(event)) {
          return;
        }
        event.preventDefault();
        openInspectorWindow();
      }}
    >
      <SidebarNavIcon sidebarOpen={sidebarOpen}>{icon}</SidebarNavIcon>
      {sidebarOpen ? <span>{children}</span> : null}
    </a>
  );
}

export function Sidebar() {
  const sidebarOpen = useAppStore((state) => state.sidebarOpen);
  const toggleSidebar = useAppStore((state) => state.toggleSidebar);
  const location = useLocation();
  const marketHref = useMemo(
    () => resolveMarketListHref(location.pathname, location.search, location.state),
    [location.pathname, location.search, location.state],
  );
  const showApiDocsMenu = useAppStore(
    (state) => state.dashboardSettings.showApiDocsMenu,
  );
  const { t } = useTranslation();

  return (
    <div
      className={cn(
        "fixed inset-y-0 left-0 z-40 flex flex-col overflow-x-hidden transition-[width] duration-300 ease-in-out",
        "border-r border-border bg-card",
        sidebarOpen ? "w-64" : "w-16",
      )}
    >
      <div className={sidebarHeaderClassName()}>
        <div className={sidebarHeaderBrandClassName(sidebarOpen)}>
          {sidebarOpen ? (
            <>
              {/* Brand: show logo + title when expanded; overflow-x-hidden on sidebar clips during width transition */}
              <img
                src="/logo.svg"
                alt="MCPMate"
                className={cn(
                  "h-6 w-6 shrink-0 object-contain transition-colors",
                  "dark:invert dark:brightness-0",
                )}
              />
              <span className="shrink-0 whitespace-nowrap font-bold text-xl text-foreground">
                {t("layout.brand", { defaultValue: "MCPMate" })}{" "}
                <SidebarBrandBadge />
              </span>
              <Button
                variant="ghost"
                size="icon"
                onClick={toggleSidebar}
                className="ml-auto -mr-2 h-9 w-9 shrink-0 rounded-md text-muted-foreground hover:bg-accent hover:text-accent-foreground"
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
              className={sidebarLogoToggleClassName()}
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

      <div className={sidebarBodyClassName(sidebarOpen)}>
        <SidebarSectionLabelSlot
          sidebarOpen={sidebarOpen}
          label={t("nav.main", { defaultValue: "MAIN" })}
        />

        <SidebarLink
          to="/"
          icon={<LayoutDashboard size={20} />}
          sidebarOpen={sidebarOpen}
        >
          {t("nav.dashboard", { defaultValue: "Dashboard" })}
        </SidebarLink>

        <SidebarLink
          to="/profiles"
          icon={<Sliders size={20} />}
          sidebarOpen={sidebarOpen}
        >
          {t("nav.profiles", { defaultValue: "Profiles" })}
        </SidebarLink>

        <SidebarLink
          to="/clients"
          icon={<AppWindow size={20} />}
          sidebarOpen={sidebarOpen}
        >
          {t("nav.clients", { defaultValue: "Clients" })}
        </SidebarLink>

        <SidebarLink
          to="/servers"
          icon={<Server size={20} />}
          sidebarOpen={sidebarOpen}
        >
          {t("nav.servers", { defaultValue: "Servers" })}
        </SidebarLink>

        <SidebarLink
          to={marketHref}
          icon={<Store size={20} />}
          sidebarOpen={sidebarOpen}
        >
          {t("nav.market", { defaultValue: "Market" })}
        </SidebarLink>

        {/* Tools removed per feedback */}

        <SidebarSectionLabelSlot
          sidebarOpen={sidebarOpen}
          label={t("nav.developer", { defaultValue: "Advanced" })}
          className={sidebarOpen ? "mt-4" : undefined}
        />

        <SidebarLink
          to="/audit"
          icon={<FileSearch size={20} />}
          sidebarOpen={sidebarOpen}
        >
          {t("nav.audit", { defaultValue: "Logs" })}
        </SidebarLink>

        <SidebarLink
          to="/secrets"
          icon={<KeyRound size={20} />}
          sidebarOpen={sidebarOpen}
        >
          {t("nav.secrets", { defaultValue: "Secure Store" })}
        </SidebarLink>

        <SidebarLink
          to="/runtime"
          icon={<Activity size={20} />}
          sidebarOpen={sidebarOpen}
        >
          {t("nav.runtime", { defaultValue: "Runtime" })}
        </SidebarLink>

        <SidebarInspectorLink icon={<Microscope size={20} />} sidebarOpen={sidebarOpen}>
          {t("nav.inspector", { defaultValue: "Inspector" })}
        </SidebarInspectorLink>

        {showApiDocsMenu && (
          <SidebarLink
            to="/api-docs"
            icon={<Bug size={20} />}
            sidebarOpen={sidebarOpen}
          >
            {t("nav.apiDocs", { defaultValue: "API Docs" })}
          </SidebarLink>
        )}

        <div className={sidebarFooterClassName(sidebarOpen)}>
          <AccountSessionDialog sidebarOpen={sidebarOpen} />

          <SidebarLink
            to="/settings"
            icon={<Settings size={20} />}
            sidebarOpen={sidebarOpen}
          >
            {t("nav.settings", { defaultValue: "Settings" })}
          </SidebarLink>
        </div>
      </div>
    </div>
  );
}
