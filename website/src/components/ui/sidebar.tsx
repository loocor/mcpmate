import type * as React from "react";

// Minimal shadcn/ui-compatible Sidebar primitives for the website project.
// API mirrors the docs enough for our docs sidebar usage.

export function Sidebar({
	className = "",
	children,
}: {
	className?: string;
	children: React.ReactNode;
}) {
	return (
		<nav className={`text-sm ${className}`.trim()} aria-label="Docs sidebar">
			{children}
		</nav>
	);
}

export function SidebarContent({
	className = "",
	children,
}: {
	className?: string;
	children: React.ReactNode;
}) {
	return <div className={className}>{children}</div>;
}

export function SidebarGroup({
	className = "",
	children,
}: {
	className?: string;
	children: React.ReactNode;
}) {
	return <div className={`mb-1 ${className}`.trim()}>{children}</div>;
}

export function SidebarGroupLabel({
	className = "",
	children,
}: {
	className?: string;
	children: React.ReactNode;
}) {
	return <div className={`${className}`.trim()}>{children}</div>;
}

export function SidebarGroupContent({
	className = "",
	children,
}: {
	className?: string;
	children: React.ReactNode;
}) {
	return <div className={`mt-1 ${className}`.trim()}>{children}</div>;
}

export function SidebarMenu({
	className = "",
	children,
}: {
	className?: string;
	children: React.ReactNode;
}) {
	return <ul className={`space-y-1 ${className}`.trim()}>{children}</ul>;
}

export function SidebarMenuItem({
	className = "",
	children,
}: {
	className?: string;
	children: React.ReactNode;
}) {
	return <li className={className}>{children}</li>;
}

export function SidebarMenuButton({
	className = "",
	active,
	children,
	onClick,
	onMouseEnter,
}: {
	className?: string;
	active?: boolean;
	children: React.ReactNode;
	onClick?: React.ButtonHTMLAttributes<HTMLButtonElement>["onClick"];
	onMouseEnter?: React.ButtonHTMLAttributes<HTMLButtonElement>["onMouseEnter"];
}) {
	return (
		<button
			type="button"
			onClick={onClick}
			onMouseEnter={onMouseEnter}
			data-active={active ? "true" : undefined}
			className={`group w-full text-left rounded-md px-2 py-2.5 transition-colors hover:bg-slate-100 dark:hover:bg-slate-800 ${
				active ? "bg-slate-100 dark:bg-slate-800 font-medium" : ""
			} ${className}`.trim()}
		>
			{children}
		</button>
	);
}
