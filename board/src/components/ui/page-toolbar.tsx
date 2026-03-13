import {
	ArrowUp,
	ArrowDown,
	ChevronLeft,
	ChevronRight,
	Grid3X3,
	List,
	Search,
} from "lucide-react";
import React from "react";
import { cn } from "../../lib/utils";
import type {
	SearchField,
	SortOption,
	SortState,
} from "../../types/page-toolbar";

// 通用实体接口
export interface Entity {
	id: string;
	name: string;
	description?: string;
	[key: string]: unknown;
}
import { Button } from "./button";
import { Input } from "./input";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "./select";

// 工具栏配置
export interface PageToolbarConfig<T extends Entity = Entity> {
	// 数据源
	data?: T[];

	// 搜索配置
	search?: {
		placeholder?: string;
		fields?: SearchField[];
		debounceMs?: number;
	};

	// 视图模式配置
	viewMode?: {
		enabled?: boolean;
		defaultMode?: "grid" | "list";
	};

	// 排序配置
	sort?: {
		enabled?: boolean;
		options: SortOption[];
		defaultSort?: string;
	};

	// 布局配置
	layout?: "horizontal" | "vertical" | "responsive";
	className?: string;

	// 精简模式配置
	compact?: {
		enabled?: boolean;
		showExpandButton?: boolean;
	};
}

// 工具栏状态
export interface PageToolbarState {
	search: string;
	viewMode: "grid" | "list";
	sort: string;
	sortState?: SortState;
	expanded?: boolean;
}

// 工具栏回调
export interface PageToolbarCallbacks<T extends Entity = Entity> {
	onSearchChange: (search: string) => void;
	onViewModeChange: (mode: "grid" | "list") => void;
	onSortedDataChange?: (sortedData: T[]) => void;
	onExpandedChange?: (expanded: boolean) => void;
}

// 工具栏属性
export interface PageToolbarProps<T extends Entity = Entity> {
    config: PageToolbarConfig<T>;
    state: PageToolbarState;
    callbacks: PageToolbarCallbacks<T>;
    actions?: React.ReactNode;
    filters?: React.ReactNode;
    className?: string;
}

export function PageToolbar<T extends Entity = Entity>({
    config,
    state,
    callbacks,
    actions,
    filters,
    className,
}: PageToolbarProps<T>) {
	const {
		data = [],
		search: searchConfig,
		viewMode: viewModeConfig,
		sort: sortConfig,
		compact: compactConfig,
	} = config;

	const { search, viewMode, expanded = false } = state;

	const {
		onSearchChange,
		onViewModeChange,
		onSortedDataChange,
		onExpandedChange,
	} = callbacks;

	// 辅助函数：获取嵌套属性值
	const getNestedValue = React.useCallback(
		(obj: unknown, path: string): unknown => {
			return path.split(".").reduce((current: unknown, key: string) => {
				return (current as Record<string, unknown>)?.[key];
			}, obj);
		},
		[],
	);

	// 内部排序状态
	const [sort, setSort] = React.useState(
		sortConfig?.defaultSort || sortConfig?.options?.[0]?.value || "name",
	);
	const [sortState, setSortState] = React.useState<SortState>(() => {
		const defaultField =
			sortConfig?.defaultSort || sortConfig?.options?.[0]?.value || "name";
		const defaultOption = sortConfig?.options.find(
			(opt) => opt.value === defaultField,
		);
		const defaultDirection =
			defaultOption?.defaultDirection || defaultOption?.direction || "asc";
		return { field: defaultField, direction: defaultDirection };
	});

	// 搜索过滤
	const filteredData = React.useMemo(() => {
		if (!searchConfig || !search.trim()) return data;

		const searchLower = search.toLowerCase();
		return data.filter((item) => {
			return (
				searchConfig.fields?.some((field) => {
					const value = getNestedValue(item, field.key);
					return String(value).toLowerCase().includes(searchLower);
				}) || false
			);
		});
	}, [data, search, searchConfig, getNestedValue]);

	// 排序处理
	const sortedData = React.useMemo(() => {
		if (!sortConfig) return filteredData;

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
	}, [filteredData, sortState, sortConfig, getNestedValue]);

	// 通知排序后的数据变化（避免无限循环）
	const lastSortedRef = React.useRef<T[] | null>(null);
	React.useEffect(() => {
		if (!onSortedDataChange) return;

		const previous = lastSortedRef.current;
		const isSameAsPrevious =
			previous !== null &&
			previous.length === sortedData.length &&
			previous.every((item, index) => item === sortedData[index]);

		if (isSameAsPrevious) {
			return;
		}

		lastSortedRef.current = sortedData;
		onSortedDataChange(sortedData);
	}, [sortedData, onSortedDataChange]);

	// 是否启用精简模式
	const isCompact = compactConfig?.enabled !== false;
	const isExpanded = expanded || !isCompact;

	// 渲染搜索框
	const renderSearch = () => {
		if (!searchConfig) return null;

		return (
			<div className="flex-1">
				<div className="flex items-center gap-2">
					{/* 展开/收起按钮 */}
					{isCompact && compactConfig?.showExpandButton !== false && (
						<Button
							variant="ghost"
							size="sm"
							onClick={() => onExpandedChange?.(!expanded)}
							className="h-9 w-9 p-0 shrink-0 text-slate-600 hover:text-slate-900 hover:bg-slate-100 dark:text-slate-400 dark:hover:text-slate-100 dark:hover:bg-slate-800"
						>
							{expanded ? (
								<ChevronRight className="h-4 w-4" />
							) : (
								<ChevronLeft className="h-4 w-4" />
							)}
						</Button>
					)}

					{/* 搜索输入框 */}
					<div className="relative flex-1">
						<Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-500" />
						<Input
							value={search}
							onChange={(e) => onSearchChange(e.target.value)}
							placeholder={searchConfig.placeholder || "Search..."}
							className="h-9 w-full rounded-md border border-slate-200 bg-white px-4 py-2 pl-10 text-sm placeholder:text-slate-500 focus:outline-none focus:ring-2 focus:ring-slate-300 dark:border-slate-800 dark:bg-slate-950 dark:placeholder:text-slate-400 dark:focus:ring-slate-600"
						/>
					</div>
				</div>
			</div>
		);
	};

	// 渲染视图切换
	const renderViewMode = () => {
		if (!viewModeConfig?.enabled || !isExpanded) return null;

		return (
			<div className="flex items-center rounded-md border border-slate-200 dark:border-slate-800 h-9">
				<Button
					variant={viewMode === "grid" ? "default" : "ghost"}
					size="sm"
					onClick={() => onViewModeChange("grid")}
					className="h-9 px-3 rounded-l-md rounded-r-none"
				>
					<Grid3X3 className="h-4 w-4" />
				</Button>
				<Button
					variant={viewMode === "list" ? "default" : "ghost"}
					size="sm"
					onClick={() => onViewModeChange("list")}
					className="h-9 px-3 rounded-r-md rounded-l-none"
				>
					<List className="h-4 w-4" />
				</Button>
			</div>
		);
	};

	// 渲染排序
	const renderSort = () => {
		if (!sortConfig?.enabled || !isExpanded) return null;

		// 获取当前排序选项
		const currentOption = sortConfig.options.find((opt) => opt.value === sort);
		const currentDirection =
			sortState?.direction ||
			currentOption?.defaultDirection ||
			currentOption?.direction ||
			"asc";

		// 处理排序字段切换
		const handleSortFieldChange = (newSort: string) => {
			setSort(newSort);

			// 切换到不同的字段，使用默认方向
			const newOption = sortConfig.options.find((opt) => opt.value === newSort);
			const newDirection =
				newOption?.defaultDirection || newOption?.direction || "asc";
			setSortState({ field: newSort, direction: newDirection });
		};

		// 处理排序方向切换
		const handleSortDirectionToggle = () => {
			const newDirection = currentDirection === "asc" ? "desc" : "asc";
			setSortState({ field: sort, direction: newDirection });
		};

		// 获取当前排序选项的标签
		const currentLabel = currentOption?.label || "Sort";

		return (
			<div className="flex items-center rounded-md border border-slate-200 dark:border-slate-800 h-9">
				{/* 排序方向切换按钮 - 在左侧 */}
				<Button
					variant="outline"
					size="sm"
					onClick={handleSortDirectionToggle}
					className="h-9 w-9 p-0 shrink-0 rounded-r-none border-r border-slate-200 dark:border-slate-800"
					title={`Sort ${currentDirection === "asc" ? "Descending" : "Ascending"}`}
				>
					{currentDirection === "asc" ? (
						<ArrowUp className="h-4 w-4" />
					) : (
						<ArrowDown className="h-4 w-4" />
					)}
				</Button>

				{/* 排序字段选择器 */}
				<Select value={sort} onValueChange={handleSortFieldChange}>
					<SelectTrigger className="h-9 w-full sm:w-[200px] border-l-0 rounded-l-none">
						<SelectValue placeholder="Sort">{currentLabel}</SelectValue>
					</SelectTrigger>
					<SelectContent align="end">
						{sortConfig.options.map((option) => (
							<SelectItem key={option.value} value={option.value}>
								{option.label}
							</SelectItem>
						))}
					</SelectContent>
				</Select>
			</div>
		);
	};

	return (
		<div className={cn("flex items-center gap-2", className)}>
			{/* 搜索框 */}
			{renderSearch()}

			{/* 控制按钮组 */}
			<div className="flex items-center gap-2">
            {/* 排序 */}
            {renderSort()}

            {/* 过滤器（在展开模式下显示） */}
            {isExpanded && filters}

            {/* 视图切换：位于过滤之后 */}
            {renderViewMode()}

            {/* 自定义操作（刷新 / 新增） */}
            {actions}
			</div>
		</div>
	);
}

// 默认配置
export const defaultPageToolbarConfig: PageToolbarConfig = {
	search: {
		placeholder: "Search...",
		debounceMs: 300,
	},
	viewMode: {
		enabled: true,
		defaultMode: "grid",
	},
	sort: {
		enabled: true,
		options: [
			{ value: "name", label: "Name", defaultDirection: "asc" },
			{ value: "updated", label: "Recently updated", defaultDirection: "desc" },
		],
		defaultSort: "name",
	},
	layout: "responsive",
	compact: {
		enabled: true,
		showExpandButton: true,
	},
};

// 默认状态
export const defaultPageToolbarState: PageToolbarState = {
	search: "",
	viewMode: "grid",
	sort: "name",
	expanded: false,
};
