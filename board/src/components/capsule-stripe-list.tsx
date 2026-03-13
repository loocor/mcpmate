import * as React from "react";
import { cn } from "../lib/utils";

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
				"flex flex-col rounded-[10px] border border-slate-200/80 dark:border-slate-800/80 overflow-hidden",
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
                "even:bg-white odd:bg-slate-50 dark:even:bg-slate-950 dark:odd:bg-slate-900",
                interactive &&
                    "cursor-pointer transition-colors hover:bg-accent/50 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent/60 focus-visible:ring-offset-2 focus-visible:ring-offset-background",
                className,
            )}
            {...rest}
        />
    );
});

CapsuleStripeListItem.displayName = "CapsuleStripeListItem";
