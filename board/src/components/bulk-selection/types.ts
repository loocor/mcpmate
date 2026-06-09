import type { LucideIcon } from "lucide-react";

export type BulkSelectionMode = "browse" | "bulk";

export type BulkAction = {
	id: string;
	icon: LucideIcon;
	label: string;
	variant?: "default" | "outline" | "secondary" | "destructive" | "ghost";
	disabled?: boolean;
	onClick: () => void;
};

export type BulkSelectionController = {
	selectedCount: number;
	selectedIds: string[];
	selectAll: (ids: string[]) => void;
	clearSelection: () => void;
};
