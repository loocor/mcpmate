import { ChevronLeft, ChevronRight, ChevronsLeft, ChevronsRight } from "lucide-react";
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
}

export function Pagination({
	currentPage,
	hasPreviousPage,
	hasNextPage,
	isLoading = false,
	itemsPerPage,
	currentPageItemCount,
	onItemsPerPageChange,
	onPreviousPage,
	onFirstPage,
	onNextPage,
	onLastPage,
	hasFirstPage,
	hasLastPage,
	pageSizeOptions = [10, 20, 50, 100],
	className,
}: PaginationProps) {
	const { t } = useTranslation();
	const hasItemsOnCurrentPage = currentPageItemCount > 0;
	const startItem = (currentPage - 1) * itemsPerPage + 1;
	const endItem = startItem + currentPageItemCount - 1;
	const canGoFirst = hasFirstPage ?? hasPreviousPage;
	const canGoLast = hasLastPage ?? hasNextPage;
	const isFirstDisabled = !onFirstPage || !canGoFirst || isLoading;
	const isPreviousDisabled = !hasPreviousPage || isLoading;
	const isNextDisabled = !hasNextPage || isLoading;
	const isLastDisabled = !onLastPage || !canGoLast || isLoading;

	const handleItemsPerPageChange = (value: string) => {
		onItemsPerPageChange?.(Number(value));
	};

	return (
		<div className={cn("flex items-center justify-between", className)}>
			<div className="flex items-center gap-4 text-sm text-slate-600 dark:text-slate-400">
				<span>
					{t("pagination.page", {
						page: currentPage,
						defaultValue: "Page {{page}}",
					})}
				</span>
				{hasItemsOnCurrentPage ? (
					<span>
						{t("pagination.showing", {
							start: startItem,
							end: endItem,
							defaultValue: "Showing {{start}}-{{end}} items",
						})}
					</span>
				) : null}
				{onItemsPerPageChange ? (
					<div className="flex items-center gap-2">
						<span>{t("pagination.perPage", { defaultValue: "Per page" })}</span>
						<Select
							value={String(itemsPerPage)}
							onValueChange={handleItemsPerPageChange}
							disabled={isLoading}
						>
							<SelectTrigger className="h-8 w-[90px]">
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
					</div>
				) : null}
			</div>

			<div className="flex items-center gap-2">
				<Button
					variant="outline"
					size="sm"
					onClick={onFirstPage}
					disabled={isFirstDisabled}
					className="gap-1"
				>
					<ChevronsLeft className="h-4 w-4" />
					{t("pagination.first", { defaultValue: "First" })}
				</Button>
				<Button
					variant="outline"
					size="sm"
					onClick={onPreviousPage}
					disabled={isPreviousDisabled}
					className="gap-1"
				>
					<ChevronLeft className="h-4 w-4" />
					{t("pagination.previous", { defaultValue: "Previous" })}
				</Button>
				<Button
					variant="outline"
					size="sm"
					onClick={onNextPage}
					disabled={isNextDisabled}
					className="gap-1"
				>
					{t("pagination.next", { defaultValue: "Next" })}
					<ChevronRight className="h-4 w-4" />
				</Button>
				<Button
					variant="outline"
					size="sm"
					onClick={onLastPage}
					disabled={isLastDisabled}
					className="gap-1"
				>
					{t("pagination.last", { defaultValue: "Last" })}
					<ChevronsRight className="h-4 w-4" />
				</Button>
			</div>
		</div>
	);
}
