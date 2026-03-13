import type { ReactNode } from "react";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "./ui/card";

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

        {/* Stats cards */}
        {statsCards && (
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
                {statsCards}
            </div>
        )}

			{/* Main content */}
			{children}
		</div>
	);
}

export interface StatsCardProps {
	title: string;
	value: string | number;
	description: string;
	icon?: ReactNode;
	className?: string;
}

export function StatsCard({
	title,
	value,
	description,
	className = "",
}: StatsCardProps) {
	return (
		<Card className={`flex flex-col ${className}`}>
			<CardHeader className="pb-2">
				<CardTitle className="text-sm">{title}</CardTitle>
			</CardHeader>
			<CardContent className="pt-0">
				<div className="text-2xl font-bold">{value}</div>
				<CardDescription>{description}</CardDescription>
			</CardContent>
		</Card>
	);
}

export interface EmptyStateProps {
	icon: ReactNode;
	title: string;
	description: string;
	action?: ReactNode;
}

export function EmptyState({
	icon,
	title,
	description,
	action,
}: EmptyStateProps) {
	return (
		<div className="text-center py-8">
			<div className="mx-auto h-12 w-12 text-slate-400 mb-4">{icon}</div>
			<p className="text-slate-500 mb-2">{title}</p>
			<p className="text-sm text-slate-400 mb-4">{description}</p>
			{action}
		</div>
	);
}
