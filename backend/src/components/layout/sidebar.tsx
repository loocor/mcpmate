import React from 'react';
import { NavLink } from 'react-router-dom';
import { cn } from '../../lib/utils';
import { useAppStore } from '../../lib/store';
import {
  LayoutDashboard,
  Server,
  Wrench,
  Activity,
  Settings,
  ChevronLeft,
  ChevronRight,
  Sliders,
  Bug
} from 'lucide-react';
import { Button } from '../ui/button';

interface SidebarLinkProps {
  to: string;
  icon: React.ReactNode;
  children: React.ReactNode;
}

function SidebarLink({ to, icon, children }: SidebarLinkProps) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) => cn(
        "flex items-center px-3 py-2 text-sm font-medium rounded-md transition-colors",
        "hover:bg-slate-200 dark:hover:bg-slate-800",
        isActive
          ? "bg-slate-200 text-slate-900 dark:bg-slate-800 dark:text-slate-50"
          : "text-slate-700 dark:text-slate-400"
      )}
    >
      <span className="mr-3 h-5 w-5">{icon}</span>
      <span>{children}</span>
    </NavLink>
  );
}

export function Sidebar() {
  const { sidebarOpen, toggleSidebar } = useAppStore();

  return (
    <div className={cn(
      "fixed inset-y-0 left-0 z-40 flex flex-col transition-all duration-300 ease-in-out",
      "border-r border-slate-200 bg-white dark:border-slate-800 dark:bg-slate-950",
      sidebarOpen ? "w-64" : "w-16"
    )}>
      <div className="flex h-16 items-center justify-between px-4">
        <div className={cn(
          "flex items-center",
          sidebarOpen ? "justify-between w-full" : "justify-center"
        )}>
          {sidebarOpen && (
            <span className="font-bold text-xl dark:text-white">MCPMate</span>
          )}
          <Button
            variant="ghost"
            size="icon"
            onClick={toggleSidebar}
            className="ml-auto"
            aria-label={sidebarOpen ? "Collapse sidebar" : "Expand sidebar"}
          >
            {sidebarOpen ? <ChevronLeft size={18} /> : <ChevronRight size={18} />}
          </Button>
        </div>
      </div>

      <div className="flex flex-col flex-1 gap-1 px-2 py-4">
        <div className={cn("flex", !sidebarOpen && "justify-center")}>
          {sidebarOpen ? (
            <span className="px-3 text-xs font-semibold text-slate-500 dark:text-slate-400 mb-1">
              MAIN
            </span>
          ) : null}
        </div>

        <SidebarLink to="/" icon={<LayoutDashboard size={20} />}>
          {sidebarOpen && "Dashboard"}
        </SidebarLink>

        <SidebarLink to="/config" icon={<Sliders size={20} />}>
          {sidebarOpen && "ConfSuits"}
        </SidebarLink>

        <SidebarLink to="/servers" icon={<Server size={20} />}>
          {sidebarOpen && "Servers"}
        </SidebarLink>

        <SidebarLink to="/tools" icon={<Wrench size={20} />}>
          {sidebarOpen && "Tools"}
        </SidebarLink>

        <SidebarLink to="/system" icon={<Activity size={20} />}>
          {sidebarOpen && "System"}
        </SidebarLink>

        <div className="mt-auto">
          <SidebarLink to="/debug/api-test" icon={<Bug size={20} />}>
            {sidebarOpen && "API Test"}
          </SidebarLink>

          <SidebarLink to="/settings" icon={<Settings size={20} />}>
            {sidebarOpen && "Settings"}
          </SidebarLink>
        </div>
      </div>
    </div>
  );
}