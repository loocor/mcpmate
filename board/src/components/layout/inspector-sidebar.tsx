import { Menu, Settings2 } from "lucide-react";
import type React from "react";
import { useTranslation } from "react-i18next";
import { useAppStore } from "../../lib/store";
import { cn } from "../../lib/utils";
import { Button } from "../ui/button";
import { TooltipProvider } from "../ui/tooltip";
import { SidebarBrandBadge } from "./sidebar-brand-badge";
import {
	SidebarNavIcon,
	inspectorSidebarBodyClassName,
	inspectorSidebarFooterClassName,
	inspectorSidebarScrollContentClassName,
	sidebarHeaderBrandClassName,
	sidebarHeaderClassName,
	sidebarLogoToggleClassName,
	sidebarNavItemClassName,
} from "./sidebar-nav-item";
import type { InspectorFooterWorkspace } from "../../pages/inspector/inspector-feature-config";

type InspectorSidebarProps = {
	children: React.ReactNode;
	footerWorkspace: InspectorFooterWorkspace | null;
	onFooterWorkspaceChange: (workspace: InspectorFooterWorkspace) => void;
};

function InspectorFooterButton({
	icon,
	label,
	sidebarOpen,
	active,
	onClick,
}: {
	icon: React.ReactNode;
	label: string;
	sidebarOpen: boolean;
	active: boolean;
	onClick: () => void;
}) {
	return (
		<button
			type="button"
			onClick={onClick}
			className={sidebarNavItemClassName(sidebarOpen, { active })}
		>
			<SidebarNavIcon sidebarOpen={sidebarOpen}>{icon}</SidebarNavIcon>
			{sidebarOpen ? <span>{label}</span> : null}
		</button>
	);
}

export function InspectorSidebar({
	children,
	footerWorkspace,
	onFooterWorkspaceChange,
}: InspectorSidebarProps) {
	const sidebarOpen = useAppStore((state) => state.sidebarOpen);
	const toggleSidebar = useAppStore((state) => state.toggleSidebar);
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
								<SidebarBrandBadge variant="inspector" />
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

			<div
				className={inspectorSidebarBodyClassName(
					sidebarOpen,
					sidebarOpen ? "min-h-0" : undefined,
				)}
			>
				<TooltipProvider delayDuration={200}>
					{sidebarOpen ? (
						<div className={inspectorSidebarScrollContentClassName()}>{children}</div>
					) : (
						children
					)}
				</TooltipProvider>

				<div className={inspectorSidebarFooterClassName(sidebarOpen)}>
					<InspectorFooterButton
						icon={<Settings2 size={20} />}
						label="Configuration"
						sidebarOpen={sidebarOpen}
						active={footerWorkspace === "configuration"}
						onClick={() => onFooterWorkspaceChange("configuration")}
					/>
				</div>
			</div>
		</div>
	);
}
