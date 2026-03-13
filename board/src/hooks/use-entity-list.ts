import { useEffect, useMemo, useState } from "react";
import { useDebouncedValue } from "../lib/hooks/use-debounced-value";
import type { SearchField, SortOption, SortState } from "../types/page-toolbar";

// 通用实体接口
export interface Entity {
	id: string;
	name: string;
	description?: string;
	[key: string]: unknown;
}

// 搜索配置
export interface SearchConfig {
	fields: SearchField[];
	debounceMs?: number;
}

// 排序配置
export interface SortConfig {
	options: SortOption[];
	defaultSort?: string;
	defaultDirection?: "asc" | "desc";
}

// Hook 配置
export interface UseEntityListConfig<T extends Entity> {
	// 数据源
	data: T[];
	loading?: boolean;

	// 搜索配置
	search?: SearchConfig;

	// 排序配置
	sort?: SortConfig;

	// 分页配置
	pagination?: {
		pageSize?: number;
		enabled?: boolean;
	};
}

// Hook 返回值
export interface UseEntityListReturn<T extends Entity> {
	// 处理后的数据
	filteredData: T[];

	// 搜索状态
	search: string;
	setSearch: (search: string) => void;
	debouncedSearch: string;

	// 排序状态
	sort: string;
	setSort: (sort: string) => void;
	sortState: SortState;
	setSortState: (sortState: SortState) => void;

	// 分页状态
	currentPage: number;
	setCurrentPage: (page: number) => void;
	totalPages: number;

	// 统计信息
	stats: {
		total: number;
		filtered: number;
		showing: number;
	};
}

export function useEntityList<T extends Entity>({
	data,
	search: searchConfig,
	sort: sortConfig,
	pagination: paginationConfig,
}: UseEntityListConfig<T>): UseEntityListReturn<T> {
	// 搜索状态
	const [search, setSearch] = useState("");
	const debouncedSearch = useDebouncedValue(
		search,
		searchConfig?.debounceMs || 300,
	);

	// 排序状态
	const [sort, setSort] = useState(
		sortConfig?.defaultSort || sortConfig?.options?.[0]?.value || "name",
	);

	// 排序状态对象
	const [sortState, setSortState] = useState<SortState>(() => {
		const defaultField =
			sortConfig?.defaultSort || sortConfig?.options?.[0]?.value || "name";
		const defaultOption = sortConfig?.options.find(
			(opt) => opt.value === defaultField,
		);
		const defaultDirection =
			defaultOption?.defaultDirection ||
			defaultOption?.direction ||
			sortConfig?.defaultDirection ||
			"asc";
		return { field: defaultField, direction: defaultDirection };
	});

	// 分页状态
	const [currentPage, setCurrentPage] = useState(1);
	const pageSize = paginationConfig?.pageSize || 20;

	// 搜索逻辑
	const searchData = useMemo(() => {
		if (!searchConfig || !debouncedSearch.trim()) {
			return data;
		}

		const searchTerm = debouncedSearch.toLowerCase();

		return data.filter((item) => {
			// 按配置的字段搜索
			for (const field of searchConfig.fields) {
				const value = getNestedValue(item, field.key);
				if (value && String(value).toLowerCase().includes(searchTerm)) {
					return true;
				}
			}

			// 如果配置了权重，按权重排序
			if (searchConfig.fields.some((f) => f.weight)) {
				// 这里可以实现更复杂的权重搜索逻辑
				return false;
			}

			return false;
		});
	}, [data, debouncedSearch, searchConfig]);

	// 直接使用搜索结果，不再进行过滤
	const filteredData = searchData;

	// 排序逻辑
	const sortedData = useMemo(() => {
		if (!sortConfig) {
			return filteredData;
		}

		return [...filteredData].sort((a, b) => {
			const aValue = getNestedValue(a, sortState.field);
			const bValue = getNestedValue(b, sortState.field);

			let comparison = 0;

			if (typeof aValue === "string" && typeof bValue === "string") {
				comparison = aValue.localeCompare(bValue);
			} else if (typeof aValue === "number" && typeof bValue === "number") {
				comparison = aValue - bValue;
			} else if (aValue instanceof Date && bValue instanceof Date) {
				comparison = aValue.getTime() - bValue.getTime();
			} else {
				comparison = String(aValue).localeCompare(String(bValue));
			}

			return sortState.direction === "desc" ? -comparison : comparison;
		});
	}, [filteredData, sortState, sortConfig]);

	// 分页逻辑
	const paginatedData = useMemo(() => {
		if (!paginationConfig?.enabled) {
			return sortedData;
		}

		const startIndex = (currentPage - 1) * pageSize;
		const endIndex = startIndex + pageSize;

		return sortedData.slice(startIndex, endIndex);
	}, [sortedData, currentPage, pageSize, paginationConfig]);

	// 计算统计信息
	const stats = useMemo(() => {
		return {
			total: data.length,
			filtered: filteredData.length,
			showing: paginatedData.length,
		};
	}, [data.length, filteredData.length, paginatedData.length]);

	// 计算总页数
	const totalPages = useMemo(() => {
		if (!paginationConfig?.enabled) {
			return 1;
		}

		return Math.ceil(filteredData.length / pageSize);
	}, [filteredData.length, pageSize, paginationConfig]);

	// 重置分页当搜索或过滤改变时
	useEffect(() => {
		if (paginationConfig?.enabled) {
			setCurrentPage(1);
		}
	}, [paginationConfig?.enabled]);

	return {
		filteredData: paginatedData,
		search,
		setSearch,
		debouncedSearch,
		sort,
		setSort,
		sortState,
		setSortState,
		currentPage,
		setCurrentPage,
		totalPages,
		stats,
	};
}

// 辅助函数：获取嵌套属性值
function getNestedValue(obj: unknown, path: string): unknown {
	return path.split(".").reduce((current: unknown, key: string) => {
		return (current as Record<string, unknown>)?.[key];
	}, obj);
}

// 默认搜索字段配置
export const defaultSearchFields: SearchField[] = [
	{ key: "name", label: "Name", weight: 10 },
	{ key: "description", label: "Description", weight: 5 },
	{ key: "title", label: "Title", weight: 8 },
];

// 默认排序选项
export const defaultSortOptions: SortOption[] = [
	{ value: "name", label: "Name", direction: "asc" },
	{ value: "updated", label: "Recently updated", direction: "desc" },
	{ value: "created", label: "Recently created", direction: "desc" },
];
