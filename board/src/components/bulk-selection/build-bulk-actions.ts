import type { TFunction } from "i18next";
import { Check, CheckCheck, Square, XSquare } from "lucide-react";
import type { BulkAction, BulkSelectionController } from "./types";

function buildSelectClearBulkActions(
	bulk: BulkSelectionController,
	visibleIds: string[],
	selectAllLabel: string,
	clearLabel: string,
): BulkAction[] {
	return [
		{
			id: "select-all",
			icon: CheckCheck,
			label: selectAllLabel,
			variant: "outline",
			onClick: () => bulk.selectAll(visibleIds),
		},
		{
			id: "clear",
			icon: XSquare,
			label: clearLabel,
			variant: "outline",
			onClick: () => bulk.clearSelection(),
		},
	];
}

function buildCheckSquarePairBulkActions({
	bulk,
	visibleIds,
	selectAllLabel,
	clearLabel,
	positive,
	negative,
	disabled = false,
}: {
	bulk: BulkSelectionController;
	visibleIds: string[];
	selectAllLabel: string;
	clearLabel: string;
	positive: { id: string; label: string; onClick: () => void };
	negative: { id: string; label: string; onClick: () => void };
	disabled?: boolean;
}): BulkAction[] {
	const pairDisabled = disabled || bulk.selectedCount === 0;
	return [
		...buildSelectClearBulkActions(bulk, visibleIds, selectAllLabel, clearLabel),
		{
			id: positive.id,
			icon: Check,
			label: positive.label,
			disabled: pairDisabled,
			onClick: positive.onClick,
		},
		{
			id: negative.id,
			icon: Square,
			label: negative.label,
			variant: "secondary",
			disabled: pairDisabled,
			onClick: negative.onClick,
		},
	];
}

export function buildEnableDisableBulkActions({
	bulk,
	visibleIds,
	isPending,
	onEnable,
	onDisable,
	t,
}: {
	bulk: BulkSelectionController;
	visibleIds: string[];
	isPending: boolean;
	onEnable: () => void;
	onDisable: () => void;
	t: TFunction;
}): BulkAction[] {
	return buildCheckSquarePairBulkActions({
		bulk,
		visibleIds,
		selectAllLabel: t("profiles:detail.buttons.selectAll", {
			defaultValue: "Select all",
		}),
		clearLabel: t("profiles:detail.buttons.clearSelection", {
			defaultValue: "Clear",
		}),
		disabled: isPending,
		positive: {
			id: "enable",
			label: t("profiles:detail.buttons.enable", { defaultValue: "Enable" }),
			onClick: onEnable,
		},
		negative: {
			id: "disable",
			label: t("profiles:detail.buttons.disable", { defaultValue: "Disable" }),
			onClick: onDisable,
		},
	});
}

export function buildIncludeExcludeBulkActions({
	bulk,
	visibleIds,
	onInclude,
	onExclude,
	t,
}: {
	bulk: BulkSelectionController;
	visibleIds: string[];
	onInclude: () => void;
	onExclude: () => void;
	t: TFunction;
}): BulkAction[] {
	return buildCheckSquarePairBulkActions({
		bulk,
		visibleIds,
		selectAllLabel: t("manual.bulk.selectAll", { defaultValue: "Select all" }),
		clearLabel: t("manual.bulk.clearSelection", { defaultValue: "Clear" }),
		positive: {
			id: "include",
			label: t("manual.bulk.includeInImport", {
				defaultValue: "Add to import",
			}),
			onClick: onInclude,
		},
		negative: {
			id: "exclude",
			label: t("manual.bulk.excludeFromImport", {
				defaultValue: "Remove from import",
			}),
			onClick: onExclude,
		},
	});
}
