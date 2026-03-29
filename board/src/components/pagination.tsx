import { ChevronLeft, ChevronRight, ChevronsLeft, ChevronsRight } from "lucide-react";
import { useCallback, useEffect, useId, useMemo, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import { Button } from "./ui/button";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "./ui/select";

interface PaginationProps {
	/**
	 * Current page number (1-based)
	 */
	currentPage: number;
	/**
	 * Whether there is a previous page
	 */
	hasPreviousPage: boolean;
	/**
	 * Whether there is a next page
	 */
	hasNextPage: boolean;
	/**
	 * Whether pagination is in loading state
	 */
	isLoading?: boolean;
	/**
	 * Items per page
	 */
	itemsPerPage: number;
	/**
	 * Current page item count
	 */
	currentPageItemCount: number;
	/**
	 * Total items across all pages when known (e.g. offset APIs). Omit for cursor-only lists.
	 */
	totalItemCount?: number | null;
	/**
	 * Total number of pages when known (e.g. last page with no next). Omit when unknown (cursor APIs).
	 */
	totalPages?: number | null;
	/**
	 * When true and `totalPages` is unknown, the last-page control stays visible but disabled (cursor pagination).
	 * Callers such as audit and market set this explicitly.
	 */
	disableLastPageWhenTotalUnknown?: boolean;
	/**
	 * Jump to a 1-based page index (forward fetch or cursor seek is handled by the caller).
	 */
	onGoToPage?: (page: number) => void | Promise<void>;
	/**
	 * Callback when items per page changes
	 */
	onItemsPerPageChange?: (itemsPerPage: number) => void;
	/**
	 * Callback when previous page is clicked
	 */
	onPreviousPage: () => void;
	/**
	 * Callback when first page is clicked
	 */
	onFirstPage?: () => void;
	/**
	 * Callback when next page is clicked
	 */
	onNextPage: () => void;
	/**
	 * Callback when last page is clicked
	 */
	onLastPage?: () => void;
	/**
	 * Whether first page button should be enabled
	 */
	hasFirstPage?: boolean;
	/**
	 * Whether last page button should be enabled
	 */
	hasLastPage?: boolean;
	/**
	 * Optional page size options
	 */
	pageSizeOptions?: number[];
	/**
	 * Additional CSS classes
	 */
	className?: string;
	/**
	 * Optional content centered between the summary/page indicator (left) and navigation controls (right).
	 */
	centerSlot?: ReactNode;
}

const ICON_BTN = "h-8 w-8 shrink-0";

function PaginationSummary(props: {
	totalItemCount?: number | null;
	summaryId: string;
}) {
	const { totalItemCount, summaryId } = props;
	const { t } = useTranslation();

	const text = useMemo(() => {
		if (typeof totalItemCount !== "number" || totalItemCount < 0) {
			return null;
		}
		return t("pagination.totalItems", {
			total: totalItemCount,
			defaultValue: "Total {{total}} items",
		});
	}, [t, totalItemCount]);

	if (text === null) {
		return null;
	}

	return (
		<p
			id={summaryId}
			className="min-w-0 text-sm text-muted-foreground tabular-nums"
		>
			{text}
		</p>
	);
}

function PageIndicator(props: {
	currentPage: number;
	totalPages?: number | null;
	onGoToPage?: (page: number) => void | Promise<void>;
	isLoading?: boolean;
	describedBy?: string;
}) {
	const { currentPage, totalPages, onGoToPage, isLoading, describedBy } = props;
	const { t } = useTranslation();
	const inputId = useId();
	const [draft, setDraft] = useState(String(currentPage));
	const [focused, setFocused] = useState(false);

	useEffect(() => {
		setDraft(String(currentPage));
	}, [currentPage]);

	const commit = useCallback(() => {
		const n = parseInt(draft.trim(), 10);
		if (!Number.isFinite(n) || n < 1) {
			setDraft(String(currentPage));
			return;
		}
		let target = n;
		if (typeof totalPages === "number" && totalPages > 0 && target > totalPages) {
			target = totalPages;
			setDraft(String(target));
		}
		if (target === currentPage) {
			return;
		}
		void Promise.resolve(onGoToPage?.(target)).catch(() => {
			setDraft(String(currentPage));
		});
	}, [currentPage, draft, onGoToPage, totalPages]);

	const handleBlur = useCallback(() => {
		setFocused(false);
		commit();
	}, [commit]);

	const handleKeyDown = useCallback(
		(event: React.KeyboardEvent<HTMLInputElement>) => {
			if (event.key === "Enter") {
				event.currentTarget.blur();
			}
		},
		[],
	);

	const showTotal = typeof totalPages === "number" && totalPages > 0;
	const pageWord = t("pagination.pageWord", { defaultValue: "Page" });
	const pageSuffix = t("pagination.pageSuffix", { defaultValue: "" });

	const widthCh = useMemo(() => {
		const totalDigits =
			typeof totalPages === "number" && totalPages > 0 ? String(totalPages).length : 0;
		const core = Math.max(
			1,
			draft.length,
			String(currentPage).length,
			totalDigits,
		);
		// Padding for caret and focus ring; scales with digit count
		return Math.min(24, core + 1.75);
	}, [currentPage, draft, totalPages]);

	return (
		<div className="flex min-w-0 flex-wrap items-center gap-1.5 text-sm text-slate-600 dark:text-slate-400">
			{pageWord.trim() ? <span className="shrink-0">{pageWord}</span> : null}
			{onGoToPage ? (
				<input
					id={inputId}
					type="text"
					inputMode="numeric"
					autoComplete="off"
					disabled={isLoading}
					aria-label={t("pagination.goToPage", { defaultValue: "Go to page" })}
					aria-describedby={describedBy}
					value={draft}
					onChange={(event) => setDraft(event.target.value)}
					onFocus={() => setFocused(true)}
					onBlur={handleBlur}
					onKeyDown={handleKeyDown}
					style={{ width: `${widthCh}ch`, maxWidth: "100%" }}
					className={cn(
						"h-8 min-h-8 box-border min-w-0 rounded-md border text-center text-sm tabular-nums font-medium text-foreground transition-[width,box-shadow,border-color,background-color]",
						"disabled:cursor-not-allowed disabled:opacity-50",
						focused
							? "border-input bg-background px-2 py-0 ring-2 ring-ring ring-offset-2 ring-offset-background"
							: "border-transparent bg-transparent px-1 py-0 shadow-none ring-0 ring-offset-0 outline-none focus-visible:ring-0 focus-visible:ring-offset-0",
					)}
				/>
			) : (
				<span className="min-w-[2.5rem] tabular-nums font-medium text-foreground">{currentPage}</span>
			)}
			{pageSuffix ? (
				<span className="shrink-0 tabular-nums text-foreground">{pageSuffix}</span>
			) : null}
			{showTotal ? (
				<span className="shrink-0 tabular-nums">
					{t("pagination.ofTotal", { total: totalPages, defaultValue: "of {{total}}" })}
				</span>
			) : null}
		</div>
	);
}

export function Pagination({
	currentPage,
	hasPreviousPage,
	hasNextPage,
	isLoading = false,
	itemsPerPage,
	currentPageItemCount: _currentPageItemCount,
	totalItemCount,
	totalPages,
	disableLastPageWhenTotalUnknown,
	onGoToPage,
	onItemsPerPageChange,
	onPreviousPage,
	onFirstPage,
	onNextPage,
	onLastPage,
	hasFirstPage,
	hasLastPage,
	pageSizeOptions = [10, 20, 50, 100],
	className,
	centerSlot,
}: PaginationProps) {
	const { t } = useTranslation();
	const summaryId = useId();
	const summaryDescribes =
		typeof totalItemCount === "number" && totalItemCount >= 0 ? summaryId : undefined;
	const totalPagesKnown = typeof totalPages === "number" && totalPages > 0;
	const canGoFirst = hasFirstPage ?? hasPreviousPage;
	const canGoLast = hasLastPage ?? hasNextPage;
	const isFirstDisabled = !onFirstPage || !canGoFirst || isLoading;
	const isPreviousDisabled = !hasPreviousPage || isLoading;
	const isNextDisabled = !hasNextPage || isLoading;
	const lastPageAmbiguous = !totalPagesKnown;
	let isLastDisabled = !onLastPage || !canGoLast || isLoading;
	if (lastPageAmbiguous && disableLastPageWhenTotalUnknown) {
		isLastDisabled = true;
	}

	const handleItemsPerPageChange = useCallback(
		(value: string) => {
			onItemsPerPageChange?.(Number(value));
		},
		[onItemsPerPageChange],
	);

	const hasCenterSlot = centerSlot != null;

	return (
		<div
			className={cn(
				"flex flex-col gap-3 sm:flex-row sm:items-center",
				hasCenterSlot ? "sm:gap-2" : "sm:justify-between",
				className,
			)}
		>
			<div
				className={cn(
					"flex min-w-0 flex-col gap-2 sm:flex-row sm:items-baseline sm:gap-6",
					hasCenterSlot && "sm:min-w-0 sm:flex-1 sm:justify-start",
				)}
			>
				<PaginationSummary summaryId={summaryId} totalItemCount={totalItemCount} />
				<PageIndicator
					currentPage={currentPage}
					totalPages={totalPages}
					onGoToPage={onGoToPage}
					isLoading={isLoading}
					describedBy={summaryDescribes}
				/>
			</div>

			{hasCenterSlot ? (
				<div className="flex shrink-0 items-center justify-center px-1 text-xs text-muted-foreground sm:px-2">
					{centerSlot}
				</div>
			) : null}

			<div
				className={cn(
					"flex flex-wrap items-center gap-1",
					hasCenterSlot ? "sm:min-w-0 sm:flex-1 sm:justify-end" : "justify-end",
				)}
			>
				<Button
					type="button"
					variant="outline"
					size="icon"
					className={ICON_BTN}
					onClick={onFirstPage}
					disabled={isFirstDisabled}
					aria-label={t("pagination.first", { defaultValue: "First" })}
				>
					<ChevronsLeft className="h-4 w-4" aria-hidden />
				</Button>
				<Button
					type="button"
					variant="outline"
					size="icon"
					className={ICON_BTN}
					onClick={onPreviousPage}
					disabled={isPreviousDisabled}
					aria-label={t("pagination.previous", { defaultValue: "Previous" })}
				>
					<ChevronLeft className="h-4 w-4" aria-hidden />
				</Button>

				{onItemsPerPageChange ? (
					<Select
						value={String(itemsPerPage)}
						onValueChange={handleItemsPerPageChange}
						disabled={isLoading}
					>
						<SelectTrigger
							className="h-8 w-[4.25rem] shrink-0 px-2 text-xs"
							aria-label={t("pagination.perPage", { defaultValue: "Per page" })}
						>
							<SelectValue />
						</SelectTrigger>
						<SelectContent>
							{pageSizeOptions.map((size) => (
								<SelectItem key={size} value={String(size)}>
									{size}
								</SelectItem>
							))}
						</SelectContent>
					</Select>
				) : null}

				<Button
					type="button"
					variant="outline"
					size="icon"
					className={ICON_BTN}
					onClick={onNextPage}
					disabled={isNextDisabled}
					aria-label={t("pagination.next", { defaultValue: "Next" })}
				>
					<ChevronRight className="h-4 w-4" aria-hidden />
				</Button>
				<Button
					type="button"
					variant="outline"
					size="icon"
					className={ICON_BTN}
					onClick={onLastPage}
					disabled={isLastDisabled}
					aria-label={t("pagination.last", { defaultValue: "Last" })}
				>
					<ChevronsRight className="h-4 w-4" aria-hidden />
				</Button>
			</div>
		</div>
	);
}
