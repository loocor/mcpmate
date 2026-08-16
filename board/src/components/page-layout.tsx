import type { HTMLAttributes, ReactNode } from "react";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "./ui/card";
import { cn } from "../lib/utils";

/** Shared stats card grid: one row from sm up, 2x2 on mobile. */
export const STATS_CARD_GRID_CLASS = "grid grid-cols-2 gap-4 sm:grid-cols-4";

export interface PageLayoutProps {
	title: string;
	children: ReactNode;
	headerActions?: ReactNode;
	statsCards?: ReactNode;
	className?: string;
}

export function PageLayout({
	title,
	children,
	headerActions,
	statsCards,
	className = "",
}: PageLayoutProps) {
	return (
		<div className={`space-y-4 ${className}`}>
			{/* Page header (single-line, squeezable description) */}
			<div className="flex items-center gap-2 min-w-0">
				<p className="flex-1 min-w-0 truncate whitespace-nowrap text-base text-muted-foreground">
					{title}
				</p>
				{headerActions && (
					<div className="flex items-center gap-2 whitespace-nowrap flex-shrink-0">
						{headerActions}
					</div>
				)}
			</div>

			{statsCards && (
				<div className={STATS_CARD_GRID_CLASS}>
					{statsCards}
				</div>
			)}

			{children}
		</div>
	);
}

export interface StatsCardProps {
	title: string;
	value: string | number;
	description: string;
	icon?: ReactNode;
	action?: ReactNode;
	tone?: "default" | "warning" | "destructive";
	tooltip?: string;
	className?: string;
	onHeaderClick?: () => void;
}

function statsCardToneClassName(tone: StatsCardProps["tone"]): string {
	switch (tone) {
		case "warning":
			return "border-amber-300/80 bg-amber-50/60 text-amber-950 shadow-amber-200/30 dark:border-amber-700/70 dark:bg-amber-950/20 dark:text-amber-100";
		case "destructive":
			return "border-red-300/80 bg-red-50/60 text-red-950 shadow-red-200/30 dark:border-red-800/70 dark:bg-red-950/20 dark:text-red-100";
		default:
			return "";
	}
}

function statsCardDescriptionClassName(tone: StatsCardProps["tone"]): string {
	switch (tone) {
		case "warning":
			return "text-amber-700 dark:text-amber-300";
		case "destructive":
			return "text-red-700 dark:text-red-300";
		default:
			return "";
	}
}

export function StatsCard({
	title,
	value,
	description,
	action,
	tone = "default",
	tooltip,
	className = "",
	onHeaderClick,
}: StatsCardProps) {
	return (
		<Card
			className={cn(
				"group relative flex min-w-0 flex-col overflow-hidden transition-colors",
				statsCardToneClassName(tone),
				className,
			)}
			title={tooltip}
		>
			<CardHeader
				className={cn(
					"min-w-0 pb-0",
					action && "pr-14",
				)}
			>
				{onHeaderClick ? (
					<button
						type="button"
						onClick={onHeaderClick}
						className="block min-w-0 cursor-pointer text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
					>
						<CardTitle className="truncate text-sm" title={title}>
							{title}
						</CardTitle>
					</button>
				) : (
					<CardTitle className="truncate text-sm" title={title}>
						{title}
					</CardTitle>
				)}
				{action ? (
					<div className="absolute right-3 top-3 flex gap-1 opacity-70 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
						{action}
					</div>
				) : null}
			</CardHeader>
			<CardContent className="min-w-0 pt-0">
				<div className="truncate text-2xl font-bold" title={String(value)}>
					{value}
				</div>
				<CardDescription
					className={cn(
						"truncate",
						statsCardDescriptionClassName(tone),
					)}
					title={description}
				>
					{description}
				</CardDescription>
			</CardContent>
		</Card>
	);
}

export interface EmptyStateProps {
	icon: ReactNode;
	title: string;
	titleTooltip?: string;
	description?: string;
	action?: ReactNode;
}

export function EmptyState({
	icon,
	title,
	titleTooltip,
	description,
	action,
}: EmptyStateProps) {
	return (
		<div className="text-center py-8">
			<div className="mx-auto h-12 w-12 text-slate-400 mb-4">{icon}</div>
			<p
				className={cn("text-slate-500", description ? "mb-2" : "mb-4")}
				title={titleTooltip}
			>
				{title}
			</p>
			{description ? (
				<p className="text-sm text-slate-400 mb-4">{description}</p>
			) : null}
			{action}
		</div>
	);
}

export interface FullHeightEmptyStateCardProps {
	children: ReactNode;
	contentProps?: HTMLAttributes<HTMLDivElement>;
}

export function FullHeightEmptyStateCard({
	children,
	contentProps,
}: FullHeightEmptyStateCardProps) {
	const { className, ...restContentProps } = contentProps ?? {};

	return (
		<div className="flex h-full min-h-[20rem]">
			<Card className="flex flex-1">
				<CardContent
					{...restContentProps}
					className={cn(
						"flex flex-1 flex-col items-center justify-center p-6",
						className,
					)}
				>
					{children}
				</CardContent>
			</Card>
		</div>
	);
}
