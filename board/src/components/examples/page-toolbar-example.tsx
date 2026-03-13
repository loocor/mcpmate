import { PageToolbar, defaultPageToolbarConfig } from "../ui/page-toolbar";
import {
	useEntityList,
	defaultSearchFields,
	defaultSortOptions,
} from "../../hooks/use-entity-list";
import { Button } from "../ui/button";
import { RefreshCw, Plus } from "lucide-react";

// 示例数据
interface ExampleEntity {
	id: string;
	name: string;
	description: string;
	enabled: boolean;
	type: string;
	updatedAt: string;
	[key: string]: unknown; // 添加索引签名以符合 Entity 接口
}

const sampleData: ExampleEntity[] = [
	{
		id: "1",
		name: "MCP Server A",
		description: "A powerful MCP server for data processing",
		enabled: true,
		type: "data",
		updatedAt: "2024-01-15",
	},
	{
		id: "2",
		name: "MCP Server B",
		description: "Server for API integrations",
		enabled: false,
		type: "api",
		updatedAt: "2024-01-10",
	},
	{
		id: "3",
		name: "MCP Server C",
		description: "Database management server",
		enabled: true,
		type: "database",
		updatedAt: "2024-01-20",
	},
];

export function PageToolbarExample() {
	// 使用统一的实体列表 hook
	const { filteredData, search, setSearch, sort, setSort, stats } =
		useEntityList({
			data: sampleData,
			search: {
				fields: defaultSearchFields,
				debounceMs: 300,
			},
			sort: {
				options: defaultSortOptions,
				defaultSort: "name",
			},
		});

	// 工具栏配置
	const toolbarConfig = {
		...defaultPageToolbarConfig,
		search: {
			placeholder: "Search servers...",
			fields: defaultSearchFields,
		},
		viewMode: {
			enabled: true,
			defaultMode: "grid" as const,
		},
		sort: {
			enabled: true,
			options: defaultSortOptions,
		},
	};

	// 工具栏状态
	const toolbarState = {
		search,
		viewMode: "grid" as const,
		sort,
	};

	// 工具栏回调
	const toolbarCallbacks = {
		onSearchChange: setSearch,
		onViewModeChange: (mode: "grid" | "list") => {
			console.log("View mode changed to:", mode);
		},
		onSortChange: setSort,
	};

	// 操作按钮
	const actions = (
		<div className="flex items-center gap-2">
			<Button variant="outline" size="sm">
				<RefreshCw className="mr-2 h-4 w-4" />
				Refresh
			</Button>
			<Button size="sm">
				<Plus className="h-4 w-4" />
			</Button>
		</div>
	);

	return (
		<div className="space-y-4">
			<div>
				<h2 className="text-3xl font-bold tracking-tight">
					Page Toolbar Example
				</h2>
				<p className="text-slate-600 dark:text-slate-400">
					This example demonstrates the unified PageToolbar component with
					search, sorting, and view mode switching.
				</p>
			</div>

			{/* 统一的工具栏 */}
			<PageToolbar
				config={toolbarConfig}
				state={toolbarState}
				callbacks={toolbarCallbacks}
				actions={actions}
			/>

			{/* 统计信息 */}
			<div className="text-sm text-slate-600 dark:text-slate-400">
				Showing {stats.showing} of {stats.total} items
			</div>

			{/* 数据展示 */}
			<div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
				{filteredData.map((item) => (
					<div
						key={item.id}
						className="rounded-lg border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900"
					>
						<div className="flex items-center justify-between">
							<h3 className="font-semibold">{item.name}</h3>
							<span
								className={`rounded-full px-2 py-1 text-xs ${
									item.enabled
										? "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200"
										: "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200"
								}`}
							>
								{String(item.enabled ? "Enabled" : "Disabled")}
							</span>
						</div>
						<p className="mt-2 text-sm text-slate-600 dark:text-slate-400">
							{item.description}
						</p>
						<div className="mt-2 flex items-center justify-between text-xs text-slate-500">
							<span>{item.type}</span>
							<span>{item.updatedAt}</span>
						</div>
					</div>
				))}
			</div>

			{/* 检视信息 */}
			<div className="rounded-lg border border-slate-200 bg-slate-50 p-4 dark:border-slate-800 dark:bg-slate-900">
				<h4 className="font-semibold">Inspect Details:</h4>
				<pre className="mt-2 text-xs">
					{JSON.stringify(
						{
							search,
							sort,
							stats,
							filteredCount: filteredData.length,
						},
						null,
						2,
					)}
				</pre>
			</div>
		</div>
	);
}
