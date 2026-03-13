import { useCallback, useState } from "react";

interface UseCursorPaginationOptions {
	/**
	 * Items per page
	 */
	limit: number;
	/**
	 * Callback when search/filter conditions change
	 */
	onReset?: () => void;
}

interface UseCursorPaginationResult {
	/**
	 * Current page number (1-based)
	 */
	currentPage: number;
	/**
	 * Current cursor for the current page
	 */
	currentCursor: string | undefined;
	/**
	 * Whether there is a previous page
	 */
	hasPreviousPage: boolean;
	/**
	 * Whether there is a next page
	 */
	hasNextPage: boolean;
	/**
	 * Items per page
	 */
	itemsPerPage: number;
	/**
	 * Go to next page
	 */
	goToNextPage: (nextCursor: string | undefined) => void;
	/**
	 * Go to previous page
	 */
	goToPreviousPage: () => void;
	/**
	 * Reset to first page
	 */
	resetToFirstPage: () => void;
	/**
	 * Set whether there is a next page
	 */
	setHasNextPage: (hasNext: boolean) => void;
}

/**
 * Hook for managing cursor-based pagination state
 */
export function useCursorPagination({
	limit,
	onReset,
}: UseCursorPaginationOptions): UseCursorPaginationResult {
	const [currentPage, setCurrentPage] = useState(1);
	const [cursorHistory, setCursorHistory] = useState<(string | undefined)[]>([
		undefined,
	]);
	const [hasNextPage, setHasNextPage] = useState(false);

	const currentCursor = cursorHistory[currentPage - 1];
	const hasPreviousPage = currentPage > 1;

	const goToNextPage = useCallback((nextCursor: string | undefined) => {
		if (!nextCursor) return;

		setCurrentPage((prev) => {
			const nextPage = prev + 1;
			setCursorHistory((history) => {
				const newHistory = [...history];
				// Ensure we have the cursor for the next page
				if (newHistory.length < nextPage) {
					newHistory.push(nextCursor);
				}
				return newHistory;
			});
			return nextPage;
		});
	}, []);

	const goToPreviousPage = useCallback(() => {
		if (!hasPreviousPage) return;

		setCurrentPage((prev) => prev - 1);
	}, [hasPreviousPage]);

	const resetToFirstPage = useCallback(() => {
		setCurrentPage(1);
		setCursorHistory([undefined]);
		setHasNextPage(false);
		onReset?.();
	}, [onReset]);

	return {
		currentPage,
		currentCursor,
		hasPreviousPage,
		hasNextPage,
		itemsPerPage: limit,
		goToNextPage,
		goToPreviousPage,
		resetToFirstPage,
		setHasNextPage,
	};
}
