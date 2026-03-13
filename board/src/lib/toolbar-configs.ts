import type { PageToolbarConfig } from "../components/ui/page-toolbar";
import type { SearchField, SortOption } from "../types/page-toolbar";

// 通用搜索字段
export const commonSearchFields: SearchField[] = [
	{ key: "name", label: "Name", weight: 10 },
	{ key: "title", label: "Title", weight: 8 },
	{ key: "description", label: "Description", weight: 5 },
];

// 通用排序选项
export const commonSortOptions: SortOption[] = [
	{
		value: "name",
		label: "Name",
		direction: "asc",
	},
	{
		value: "updated",
		label: "Recently Updated",
		direction: "desc",
	},
	{
		value: "created",
		label: "Recently Created",
		direction: "desc",
	},
];

// 页面类型配置
export type PageType = "clients" | "servers" | "profiles" | "market";

// 页面特定配置
export const pageSpecificConfigs: Record<
	PageType,
	Partial<PageToolbarConfig>
> = {
	clients: {
		search: {
			placeholder: "Search clients...",
			fields: [
				{ key: "display_name", label: "Display Name", weight: 10 },
				{ key: "identifier", label: "Identifier", weight: 8 },
				{ key: "description", label: "Description", weight: 5 },
			],
		},
		sort: {
			enabled: true,
			options: [
				{
					value: "display_name",
					label: "Name",
					defaultDirection: "asc",
				},
				{
					value: "detected",
					label: "Detection Status",
					defaultDirection: "desc",
				},
				{
					value: "managed",
					label: "Management Status",
					defaultDirection: "desc",
				},
			],
		},
	},
	servers: {
		search: {
			placeholder: "Search servers...",
			fields: [
				{ key: "name", label: "Name", weight: 10 },
				{ key: "description", label: "Description", weight: 8 },
			],
		},
		sort: {
			enabled: true,
			options: [
				{
					value: "name",
					label: "Name",
					defaultDirection: "asc",
				},
				{
					value: "status",
					label: "Status",
					defaultDirection: "desc",
				},
			],
		},
	},
	profiles: {
		search: {
			placeholder: "Search profiles...",
			fields: [
				{ key: "name", label: "Name", weight: 10 },
				{ key: "description", label: "Description", weight: 8 },
			],
		},
		sort: {
			enabled: true,
			options: commonSortOptions,
		},
	},
	market: {
		search: {
			placeholder: "Search market...",
			fields: [
				{ key: "name", label: "Name", weight: 10 },
				{ key: "description", label: "Description", weight: 8 },
			],
		},
		sort: {
			enabled: true,
			options: [
				{
					value: "recent",
					label: "Recently Updated",
					defaultDirection: "desc",
				},
				{
					value: "name",
					label: "Alphabetical",
					defaultDirection: "asc",
				},
			],
		},
	},
};

// 生成工具栏配置
export function createToolbarConfig(
	pageType: PageType,
	customConfig?: Partial<PageToolbarConfig>,
): PageToolbarConfig {
	const baseConfig: PageToolbarConfig = {
		search: {
			placeholder: "Search...",
			fields: commonSearchFields,
			debounceMs: 300,
		},
		viewMode: {
			enabled: true,
			defaultMode: "grid",
		},
		sort: {
			enabled: true,
			options: commonSortOptions,
		},
		layout: "responsive",
		compact: {
			enabled: true,
			showExpandButton: true,
		},
	};

	// 合并页面特定配置
	const pageConfig = pageSpecificConfigs[pageType];
	const mergedConfig = { ...baseConfig, ...pageConfig, ...customConfig };

	return mergedConfig;
}
