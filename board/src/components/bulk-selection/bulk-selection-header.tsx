import type { ReactNode } from "react";
import { cn } from "../../lib/utils";
import { BulkSelectionToolbar } from "./bulk-selection-toolbar";
import type { BulkAction } from "./types";
import { useBulkSelectionLabels } from "./use-bulk-selection-labels";

type BulkSelectionHeaderProps = {
	title: ReactNode;
	description: ReactNode;
	isBulkMode: boolean;
	onToggleBulkMode: () => void;
	actions: BulkAction[];
	trailing?: ReactNode;
	className?: string;
};

export function BulkSelectionHeader({
	title,
	description,
	isBulkMode,
	onToggleBulkMode,
	actions,
	trailing,
	className,
}: BulkSelectionHeaderProps) {
	const { modeToggleLabel, modeExitLabel } = useBulkSelectionLabels();
	return (
		<div
			className={cn(
				"mb-3 flex shrink-0 items-center justify-between gap-3",
				className,
			)}
		>
			<div className="min-w-0">
				<div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
					{title}
				</div>
				<div className="text-xs text-slate-500 dark:text-slate-400">
					{description}
				</div>
			</div>
			<div className="flex shrink-0 items-center gap-2">
				{trailing}
				<BulkSelectionToolbar
					isBulkMode={isBulkMode}
					onToggleMode={onToggleBulkMode}
					modeToggleLabel={modeToggleLabel}
					modeExitLabel={modeExitLabel}
					actions={actions}
				/>
			</div>
		</div>
	);
}
