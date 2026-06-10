import type { ReactNode } from "react";
import { useAppStore } from "../lib/store";
import { cn } from "../lib/utils";

export interface ListGridContainerProps {
	children: ReactNode;
	loading?: boolean;
	loadingSkeleton?: ReactNode;
	emptyState?: ReactNode;
	className?: string;
	emptyClassName?: string;
}

export function ListGridContainer({
	children,
	loading = false,
	loadingSkeleton,
	emptyState,
	className,
	emptyClassName,
}: ListGridContainerProps) {
	const defaultView = useAppStore(
		(state) => state.dashboardSettings.defaultView,
	);

	if (loading) {
		return (
			<div
				className={cn(
					defaultView === "grid"
						? "grid gap-4 md:grid-cols-2 xl:grid-cols-3"
						: "space-y-4",
					className,
				)}
			>
				{loadingSkeleton}
			</div>
		);
	}

	if (emptyState) {
		return (
			<div className={cn("col-span-full", emptyClassName)}>{emptyState}</div>
		);
	}

	return (
		<div
			className={cn(
				defaultView === "grid"
					? "grid gap-4 md:grid-cols-2 xl:grid-cols-3"
					: "space-y-4",
				className,
			)}
		>
			{children}
		</div>
	);
}

export interface EntityListItemProps {
	children: ReactNode;
	onClick?: () => void;
	onKeyDown?: (e: React.KeyboardEvent) => void;
	className?: string;
}

export function EntityListItem({
	children,
	onClick,
	onKeyDown,
	className = "",
}: EntityListItemProps) {
	return (
		<button
			type="button"
			className={`flex items-center justify-between rounded-lg border border-slate-200 bg-white px-4 py-4 cursor-pointer shadow-[0_4px_12px_-10px_rgba(15,23,42,0.2)] transition-all duration-200 hover:border-primary/40 hover:shadow-xl hover:-translate-y-0.5 dark:border-slate-700 dark:bg-slate-900 dark:shadow-[0_4px_12px_-10px_rgba(15,23,42,0.5)] ${className}`}
			onClick={onClick}
			onKeyDown={onKeyDown}
		>
			{children}
		</button>
	);
}

export interface EntityCardProps {
	children: ReactNode;
	onClick?: () => void;
	onKeyDown?: (e: React.KeyboardEvent) => void;
	className?: string;
}

export function EntityCard({
	children,
	onClick,
	onKeyDown,
	className = "",
}: EntityCardProps) {
	return (
		<button
			type="button"
			className={`group overflow-hidden cursor-pointer shadow-[0_4px_12px_-10px_rgba(15,23,42,0.2)] hover:border-primary/40 transition-all duration-200 hover:shadow-xl hover:-translate-y-0.5 dark:shadow-[0_4px_12px_-10px_rgba(15,23,42,0.5)] ${className}`}
			onClick={onClick}
			onKeyDown={onKeyDown}
		>
			{children}
		</button>
	);
}
