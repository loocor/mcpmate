import {
	CapsuleStripeList,
	CapsuleStripeListItem,
} from "./capsule-stripe-list";
import { cn } from "../lib/utils";

type CapabilityListSkeletonProps = {
	rows?: number;
	showSectionLabel?: boolean;
	className?: string;
};

export function CapabilityListSkeleton({
	rows = 5,
	showSectionLabel = false,
	className,
}: CapabilityListSkeletonProps) {
	return (
		<div
			className={cn("p-3", className)}
			aria-busy="true"
			aria-live="polite"
		>
			{showSectionLabel ? (
				<div className="mb-3 h-3 w-12 animate-pulse rounded bg-slate-200 dark:bg-slate-800" />
			) : null}
			<CapsuleStripeList className="rounded-none border-0 overflow-visible">
				{Array.from({ length: rows }, (_, index) => (
					<CapsuleStripeListItem
						key={`capability-list-skeleton-${index}`}
						className="items-start"
					>
						<div className="flex w-full items-start gap-3">
							<div className="mt-0.5 h-4 w-4 shrink-0 animate-pulse rounded bg-slate-200 dark:bg-slate-800" />
							<div className="min-w-0 flex-1 space-y-2 py-0.5">
								<div className="h-4 max-w-[42%] animate-pulse rounded bg-slate-200 dark:bg-slate-800" />
								<div className="h-3 w-full animate-pulse rounded bg-slate-200 dark:bg-slate-800" />
								<div className="h-3 max-w-[88%] animate-pulse rounded bg-slate-200 dark:bg-slate-800" />
							</div>
						</div>
					</CapsuleStripeListItem>
				))}
			</CapsuleStripeList>
		</div>
	);
}
