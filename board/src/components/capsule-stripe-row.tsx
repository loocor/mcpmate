import { Check, Pencil, Plus } from "lucide-react";
import * as React from "react";
import { cn } from "../lib/utils";

const leadCircleBase =
	"flex h-6 w-6 shrink-0 items-center justify-center rounded-full border-2";

export type CapsuleStripeLeadCircleProps =
	| { variant: "toggle"; selected: boolean }
	| { variant: "readOnlyActive" }
	| { variant: "ghost"; hasProfile: boolean };

/**
 * Leading circle for rows inside {@link CapsuleStripeList}: checkbox, read-only check, or ghost action.
 */
export function CapsuleStripeLeadCircle(props: CapsuleStripeLeadCircleProps) {
	if (props.variant === "readOnlyActive") {
		return (
			<div
				className={cn(
					leadCircleBase,
					"border-slate-300 bg-slate-100 dark:border-slate-600 dark:bg-slate-700",
				)}
			>
				<Check className="h-3 w-3 shrink-0 text-slate-500" />
			</div>
		);
	}

	if (props.variant === "toggle") {
		const { selected } = props;
		return (
			<div
				className={cn(
					leadCircleBase,
					"transition-all duration-200",
					selected
						? "border-primary bg-primary text-white"
						: "border-slate-300 bg-white dark:border-slate-600 dark:bg-slate-700",
				)}
			>
				{selected ? <Check className="h-3 w-3" /> : null}
			</div>
		);
	}

	const { hasProfile } = props;
	return (
		<div
			className={cn(
				leadCircleBase,
				hasProfile
					? "border-slate-400 bg-slate-100 dark:border-slate-500 dark:bg-slate-800"
					: "border-dashed border-slate-300 dark:border-slate-600",
			)}
		>
			{hasProfile ? (
				<Pencil className="h-3 w-3 text-slate-500 dark:text-slate-400" />
			) : (
				<Plus className="h-3 w-3 text-slate-400" />
			)}
		</div>
	);
}

interface CapsuleStripeRowBodyProps {
	lead: React.ReactNode;
	children: React.ReactNode;
	trailing?: React.ReactNode;
}

/**
 * Standard horizontal layout for {@link CapsuleStripeListItem} rows: lead + main + optional trailing actions.
 */
export function CapsuleStripeRowBody({ lead, children, trailing }: CapsuleStripeRowBodyProps) {
	return (
		<div className="flex w-full items-center gap-3">
			{lead}
			<div className="flex-1 min-w-0">{children}</div>
			{trailing != null ? (
				<div className="flex shrink-0 items-center gap-2">{trailing}</div>
			) : null}
		</div>
	);
}
