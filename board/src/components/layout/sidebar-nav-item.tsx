import type React from "react";
import { cn } from "../../lib/utils";

export function sidebarBodyClassName(sidebarOpen: boolean, className?: string) {
	return cn(
		"flex flex-col flex-1 gap-1 px-2 py-4",
		!sidebarOpen && "items-center",
		className,
	);
}

export function inspectorSidebarBodyClassName(sidebarOpen: boolean, className?: string) {
	return cn(
		sidebarOpen
			? "flex min-h-0 flex-col flex-1 gap-3 px-2 pb-3 pt-0"
			: "flex flex-col flex-1 gap-1 px-2 pb-4 pt-0 items-center",
		className,
	);
}

export function sidebarHeaderClassName() {
	return "flex h-16 shrink-0 items-center px-4";
}

export function sidebarHeaderBrandClassName(sidebarOpen: boolean) {
	return cn(
		"flex min-w-0 items-center gap-2",
		sidebarOpen ? "w-full" : "w-full justify-center",
	);
}

export function sidebarSectionLabelSlotClassName(sidebarOpen: boolean) {
	return cn("flex", !sidebarOpen && "justify-center");
}

export function SidebarSectionLabelSlot({
	sidebarOpen,
	label,
	className,
}: {
	sidebarOpen: boolean;
	label?: React.ReactNode;
	className?: string;
}) {
	return (
		<div className={cn(sidebarSectionLabelSlotClassName(sidebarOpen), className)}>
			{sidebarOpen && label ? (
				<span className="mb-1 px-3 text-xs font-semibold text-muted-foreground">
					{label}
				</span>
			) : null}
		</div>
	);
}

export function sidebarFooterClassName(sidebarOpen: boolean, className?: string) {
	return cn(
		"mt-auto flex flex-col gap-1",
		!sidebarOpen && "items-center",
		className,
	);
}

export function sidebarNavItemClassName(
	sidebarOpen: boolean,
	options?: {
		active?: boolean;
		className?: string;
	},
) {
	return cn(
		"flex items-center text-sm font-medium rounded-md transition-colors",
		sidebarOpen ? "w-full px-3 py-2" : "h-9 w-9 shrink-0 justify-center px-0",
		options?.active
			? "bg-accent text-accent-foreground"
			: "text-muted-foreground hover:bg-accent",
		options?.className,
	);
}

export function SidebarNavIcon({
	sidebarOpen,
	children,
}: {
	sidebarOpen: boolean;
	children: React.ReactNode;
}) {
	return (
		<span
			className={cn(
				"flex h-5 w-5 shrink-0 items-center justify-center",
				sidebarOpen && "mr-3",
			)}
		>
			{children}
		</span>
	);
}

export function sidebarScrollContentClassName(sidebarOpen: boolean) {
	return cn(
		"min-h-0 flex-1 overflow-y-auto",
		sidebarOpen ? "w-full px-1" : "w-full",
	);
}

export function inspectorSidebarScrollContentClassName() {
	return "flex min-h-0 w-full flex-1 flex-col overflow-hidden px-1";
}

/** Main workspace scroll body inside the inspector main area. */
export function inspectorWorkspaceContentClassName(className?: string) {
	return cn("flex min-h-0 flex-1 flex-col overflow-y-auto px-3 pb-3 pt-0", className);
}

/** Connect workspace shell; child panels own their internal scrolling. */
export function inspectorConnectWorkspaceClassName(className?: string) {
	return cn(
		"box-border flex min-h-0 flex-1 flex-col overflow-hidden px-0 pb-3 pt-3",
		className,
	);
}

/** Workspace target/capability header below the h-16 shell header. */
export function inspectorWorkspaceTargetHeaderClassName(className?: string) {
	return cn("flex shrink-0 items-center bg-background p-3", className);
}

export function inspectorSidebarFooterClassName(sidebarOpen: boolean) {
	return cn(
		"flex shrink-0 flex-col gap-1",
		sidebarOpen ? "px-1" : "mt-auto items-center",
	);
}

export function sidebarLogoToggleClassName() {
	return "flex h-9 w-9 items-center justify-center rounded-md transition-colors hover:bg-accent";
}

export function inspectorSidebarExpandedControlsClassName() {
	return "flex min-h-0 w-full flex-1 flex-col gap-3";
}

export function sidebarIconRailClassName(className?: string) {
	return cn(
		"w-full shrink-0 overflow-hidden rounded-md border border-border bg-card/60",
		className,
	);
}

export function sidebarIconRailGridClassName() {
	return "grid w-full grid-cols-4 divide-x divide-border";
}

export const sidebarFeatureTabIconSize = 20;

export function sidebarIconLabeledActionClassName(options?: {
	selected?: boolean;
	className?: string;
}) {
	return cn(
		"flex w-full flex-col items-center justify-center gap-0.5 px-1 py-3 transition",
		options?.selected
			? "bg-primary/10 text-foreground"
			: "text-muted-foreground hover:bg-muted/40",
		options?.className,
	);
}

export function sidebarIconActionClassName(options?: {
	selected?: boolean;
	className?: string;
}) {
	return cn(
		"flex h-9 w-full items-center justify-center p-0 transition",
		options?.selected
			? "bg-primary/10 text-foreground"
			: "text-muted-foreground hover:bg-muted/40",
		options?.className,
	);
}
