import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useSearchParams } from "react-router-dom";

import { useUrlState } from "./use-url-state";

export type CatalogViewMode = "grid" | "list";

const GRID_ROWS_PER_PAGE = 3;
const LIST_ITEMS_PER_PAGE = 6;
export const CATALOG_PAGE_SIZE_OPTIONS = [3, 6, 9, 12] as const;

function isValidPageParam(value: string): boolean {
	return /^[1-9]\d*$/.test(value);
}

export function getCatalogPageSize(
	viewMode: CatalogViewMode,
	gridColumnCount: number,
): number {
	if (viewMode === "list") {
		return LIST_ITEMS_PER_PAGE;
	}

	const columnCount = Math.min(3, Math.max(1, gridColumnCount));
	return GRID_ROWS_PER_PAGE * columnCount;
}

export function getCatalogTotalPages(
	itemCount: number,
	pageSize: number,
): number {
	return Math.max(1, Math.ceil(itemCount / pageSize));
}

export function clampCatalogPage(page: number, totalPages: number): number {
	return Math.min(Math.max(1, page), Math.max(1, totalPages));
}

export function paginateCatalogItems<T>(
	items: readonly T[],
	page: number,
	pageSize: number,
): T[] {
	const start = (page - 1) * pageSize;
	return items.slice(start, start + pageSize);
}

function getCatalogGridColumnCount(): number {
	if (typeof window === "undefined") {
		return 1;
	}
	if (window.matchMedia("(min-width: 1280px)").matches) {
		return 3;
	}
	if (window.matchMedia("(min-width: 768px)").matches) {
		return 2;
	}
	return 1;
}

/** Responsive grid column count for catalog and market card grids (`md:2`, `xl:3`). */
export { getCatalogGridColumnCount };

export function useCatalogGridColumnCount(): number {
	const [gridColumnCount, setGridColumnCount] = useState(getCatalogGridColumnCount);

	useEffect(() => {
		const updateColumnCount = () =>
			setGridColumnCount(getCatalogGridColumnCount());

		window.addEventListener("resize", updateColumnCount);
		return () => window.removeEventListener("resize", updateColumnCount);
	}, []);

	return gridColumnCount;
}

interface ResponsiveCatalogPagination<T> {
	currentPage: number;
	pageSize: number;
	totalPages: number;
	pageItems: T[];
	hasPreviousPage: boolean;
	hasNextPage: boolean;
	goToPage: (page: number) => void;
	goToFirstPage: () => void;
	goToPreviousPage: () => void;
	goToNextPage: () => void;
	goToLastPage: () => void;
	onItemsPerPageChange: (pageSize: number) => void;
}

export function useResponsiveCatalogPagination<T>(
	items: readonly T[],
	viewMode: CatalogViewMode,
	isDataReady = true,
): ResponsiveCatalogPagination<T> {
	const [searchParams] = useSearchParams();
	const gridColumnCount = useCatalogGridColumnCount();
	const [requestedPage, setRequestedPage] = useUrlState({
		paramName: "page",
		defaultValue: 1,
		validate: isValidPageParam,
		deserialize: Number,
	});
	const [selectedPageSize, setSelectedPageSize] = useState<number | null>(null);

	const responsivePageSize = getCatalogPageSize(viewMode, gridColumnCount);
	const pageSize = selectedPageSize ?? responsivePageSize;
	const totalPages = getCatalogTotalPages(items.length, pageSize);
	const currentPage = isDataReady
		? clampCatalogPage(requestedPage, totalPages)
		: requestedPage;
	const pageParam = searchParams.get("page");
	const hasInvalidPageParam =
		pageParam !== null && !isValidPageParam(pageParam);
	const resetSignature = ["q", "filter", "sort", "view"]
		.map((key) => `${key}=${searchParams.get(key) ?? ""}`)
		.concat(`mode=${viewMode}`)
		.join("&");
	const previousResetSignature = useRef(resetSignature);
	const previousPageSize = useRef(pageSize);
	const previousResponsivePageSize = useRef(responsivePageSize);

	useEffect(() => {
		if (!isDataReady) {
			return;
		}
		if (hasInvalidPageParam) {
			setRequestedPage(1);
			return;
		}

		const shouldReset =
			previousResetSignature.current !== resetSignature ||
			previousPageSize.current !== pageSize ||
			previousResponsivePageSize.current !== responsivePageSize;

		previousResetSignature.current = resetSignature;
		previousPageSize.current = pageSize;
		previousResponsivePageSize.current = responsivePageSize;

		if (shouldReset) {
			setRequestedPage(1);
			return;
		}

		if (requestedPage !== currentPage) {
			setRequestedPage(currentPage);
		}
	}, [
		currentPage,
		hasInvalidPageParam,
		isDataReady,
		pageSize,
		requestedPage,
		resetSignature,
		responsivePageSize,
		setRequestedPage,
	]);

	const pageItems = useMemo(
		() => paginateCatalogItems(items, currentPage, pageSize),
		[items, currentPage, pageSize],
	);
	const goToPage = useCallback(
		(page: number) => setRequestedPage(clampCatalogPage(page, totalPages)),
		[setRequestedPage, totalPages],
	);
	const goToFirstPage = useCallback(() => goToPage(1), [goToPage]);
	const goToPreviousPage = useCallback(
		() => goToPage(currentPage - 1),
		[currentPage, goToPage],
	);
	const goToNextPage = useCallback(
		() => goToPage(currentPage + 1),
		[currentPage, goToPage],
	);
	const goToLastPage = useCallback(
		() => goToPage(totalPages),
		[goToPage, totalPages],
	);
	const onItemsPerPageChange = useCallback(
		(nextPageSize: number) => {
			setSelectedPageSize(nextPageSize);
			setRequestedPage(1);
		},
		[setRequestedPage],
	);

	return {
		currentPage,
		pageSize,
		totalPages,
		pageItems,
		hasPreviousPage: currentPage > 1,
		hasNextPage: currentPage < totalPages,
		goToPage,
		goToFirstPage,
		goToPreviousPage,
		goToNextPage,
		goToLastPage,
		onItemsPerPageChange,
	};
}
