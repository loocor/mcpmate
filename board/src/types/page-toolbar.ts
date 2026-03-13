// 搜索字段配置
export interface SearchField {
	key: string;
	label: string;
	weight?: number; // 搜索权重
}

// 排序选项
export interface SortOption {
	value: string;
	label: string;
	direction?: "asc" | "desc"; // 保留向后兼容
	defaultDirection?: "asc" | "desc"; // 默认排序方向
	icon?: string; // 图标名称
	showDirection?: boolean; // 是否显示方向图标
}

// 排序状态
export interface SortState {
	field: string;
	direction: "asc" | "desc";
}
