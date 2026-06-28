import { useEffect, useRef, useState } from "react";
import {
	CapsuleStripeList,
	CapsuleStripeListItem,
} from "./capsule-stripe-list";
import { cn } from "../lib/utils";

type CapabilityListSkeletonProps = {
	rows?: number;
	showSectionLabel?: boolean;
	className?: string;
	/** When true, grow to the scroll body height and derive row count from available space. */
	fillContainer?: boolean;
};

const ROW_HEIGHT_PX = 56;
const SECTION_LABEL_HEIGHT_PX = 24;
const MIN_ROWS = 3;
const EXTRA_ROWS = 1;

export function CapabilityListSkeleton({
	rows,
	showSectionLabel = false,
	className,
	fillContainer = false,
}: CapabilityListSkeletonProps) {
	const containerRef = useRef<HTMLDivElement>(null);
	const [autoRowCount, setAutoRowCount] = useState(MIN_ROWS + EXTRA_ROWS);

	const useAutoFill = fillContainer && rows == null;

	useEffect(() => {
		if (!useAutoFill) return;
		const container = containerRef.current;
		const scrollHost = container?.parentElement;
		if (!container || !scrollHost) return;

		const updateRowCount = () => {
			const reserved = showSectionLabel ? SECTION_LABEL_HEIGHT_PX : 0;
			const available = scrollHost.clientHeight - reserved;
			const nextCount =
				Math.max(MIN_ROWS, Math.floor(available / ROW_HEIGHT_PX)) + EXTRA_ROWS;
			setAutoRowCount((current) => (current === nextCount ? current : nextCount));
		};

		updateRowCount();
		const observer = new ResizeObserver(updateRowCount);
		observer.observe(scrollHost);
		return () => observer.disconnect();
	}, [showSectionLabel, useAutoFill]);

	const rowCount = rows ?? (fillContainer ? autoRowCount : 5);

	return (
		<div
			ref={containerRef}
			className={cn(
				"p-3",
				fillContainer && "h-full min-h-0 overflow-hidden",
				className,
			)}
			aria-busy="true"
			aria-live="polite"
		>
			{showSectionLabel ? (
				<div className="mb-3 h-3 w-12 animate-pulse rounded bg-slate-200 dark:bg-slate-800" />
			) : null}
			<CapsuleStripeList className="rounded-none border-0 overflow-visible">
				{Array.from({ length: rowCount }, (_, index) => (
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
