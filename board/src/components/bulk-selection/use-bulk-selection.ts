import { useCallback, useMemo, useState } from "react";
import type { BulkSelectionMode } from "./types";

export function useBulkSelection<TId extends string>() {
	const [mode, setMode] = useState<BulkSelectionMode>("browse");
	const [selectedIds, setSelectedIds] = useState<TId[]>([]);

	const selectedIdSet = useMemo(() => new Set(selectedIds), [selectedIds]);
	const selectedCount = selectedIds.length;
	const isBulkMode = mode === "bulk";

	const exitBulkMode = useCallback(() => {
		setMode("browse");
		setSelectedIds([]);
	}, []);

	const toggleMode = useCallback(() => {
		setMode((current) => {
			if (current === "bulk") {
				setSelectedIds([]);
				return "browse";
			}
			return "bulk";
		});
	}, []);

	const toggleItem = useCallback((id: TId) => {
		setSelectedIds((current) =>
			current.includes(id)
				? current.filter((item) => item !== id)
				: [...current, id],
		);
	}, []);

	const selectAll = useCallback((ids: TId[]) => {
		setSelectedIds(ids);
	}, []);

	const clearSelection = useCallback(() => {
		setSelectedIds([]);
	}, []);

	return useMemo(
		() => ({
			isBulkMode,
			selectedIds,
			selectedIdSet,
			selectedCount,
			exitBulkMode,
			toggleMode,
			toggleItem,
			selectAll,
			clearSelection,
		}),
		[isBulkMode, selectedIds, selectedIdSet, selectedCount, exitBulkMode, toggleMode, toggleItem, selectAll, clearSelection],
	);
}
