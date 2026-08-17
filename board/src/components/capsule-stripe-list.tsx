import * as React from "react";
import { cn } from "../lib/utils";

export const PROFILE_EDITOR_SIDEBAR_SCROLL_CLASS = "mx-3 mb-3 mt-0";
/**
 * Right-pane body / material-preview overlay inset after a `min-h-[62px] p-3` header.
 * `pt-1` is required so the scroll/preview frame lines up with the left list
 * (`PROFILE_EDITOR_SIDEBAR_SCROLL_CLASS` uses `mt-0`). Do not equalize this to
 * `p-3` -- overlay hover buttons use `right-4 top-2` so the visual gap from the
 * preview frame matches on top and right despite `pt-1` vs `px-3`.
 */
export const PROFILE_EDITOR_DETAIL_BODY_INSET_CLASS = "px-3 pb-3 pt-1";
export const PROFILE_EDITOR_SIDEBAR_LIST_CLASS =
	"overflow-visible rounded-none border-0";
export const PROFILE_EDITOR_SIDEBAR_ITEM_CLASS =
	"group relative min-h-16 px-3 transition-colors";
export const PROFILE_EDITOR_SIDEBAR_STICKY_ACTION_CLASS =
	"sticky bottom-3 z-[2] mt-2 px-2";
export const PROFILE_EDITOR_SIDEBAR_HOVER_ACTIONS_CLASS =
	"ml-0 flex w-0 shrink-0 gap-0 overflow-hidden opacity-0 pointer-events-none transition-[width,margin,opacity] group-hover:ml-auto group-hover:w-14 group-hover:pointer-events-auto group-hover:opacity-100 group-focus-within:ml-auto group-focus-within:w-14 group-focus-within:pointer-events-auto group-focus-within:opacity-100";

interface CapsuleStripeListProps {
	className?: string;
	children: React.ReactNode;
}

export function CapsuleStripeList({
	className,
	children,
}: CapsuleStripeListProps) {
	return (
		<div
			className={cn(
				"flex flex-col rounded-[10px] border border-slate-200/80 dark:border-slate-700/80 overflow-hidden",
				className,
			)}
		>
			{children}
		</div>
	);
}

type DivProps = React.HTMLAttributes<HTMLDivElement>;

interface CapsuleStripeListItemProps extends DivProps {
	interactive?: boolean;
}

export const CapsuleStripeListItem = React.forwardRef<
	HTMLDivElement,
	CapsuleStripeListItemProps
>(({ className, interactive = false, role, tabIndex, ...rest }, ref) => {
	return (
		<div
			ref={ref}
			role={role ?? (interactive ? "button" : undefined)}
			tabIndex={tabIndex ?? (interactive ? 0 : undefined)}
			className={cn(
				"p-2 text-sm flex items-center justify-between gap-3",
				"even:bg-white odd:bg-slate-50 dark:even:bg-slate-900 dark:odd:bg-slate-800/70",
				interactive &&
				"cursor-pointer transition-colors hover:bg-accent/50 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent/60 focus-visible:ring-offset-2 focus-visible:ring-offset-background",
				className,
			)}
			{...rest}
		/>
	);
});

CapsuleStripeListItem.displayName = "CapsuleStripeListItem";
