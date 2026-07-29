import { AlertTriangle } from "lucide-react";

import { Button } from "./ui/button";

type CapabilityEmptyStateProps = {
	title: string;
	description: string;
	actionLabel: string;
	onAction: () => void;
};

export function CapabilityEmptyState({
	title,
	description,
	actionLabel,
	onAction,
}: CapabilityEmptyStateProps) {
	return (
		<div className="flex min-h-full items-center justify-center rounded-lg border border-slate-200 bg-white px-4 py-8 text-center dark:border-slate-800 dark:bg-slate-950/40">
			<div className="flex max-w-lg flex-col items-center gap-3">
				<AlertTriangle className="h-5 w-5 text-amber-500" />
				<div className="space-y-1">
					<p className="font-medium text-slate-900 dark:text-slate-100">
						{title}
					</p>
					<p className="text-sm text-slate-500 dark:text-slate-400">
						{description}
					</p>
				</div>
				<Button type="button" size="sm" variant="outline" onClick={onAction}>
					{actionLabel}
				</Button>
			</div>
		</div>
	);
}
