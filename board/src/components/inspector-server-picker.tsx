import { CircleDot, Plug, RefreshCcw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";

export type InspectorServerSource = "managed" | "scratch";

export type InspectorServerPickerOption = {
	id: string;
	name: string;
	serverType?: string | null;
	endpointSummary?: string | null;
	iconSrc?: string | null;
	source?: InspectorServerSource;
};

type InspectorServerPickerProps = {
	serverName?: string | null;
	connected?: boolean;
	restoring?: boolean;
	onOpenConnect: () => void;
	className?: string;
};

export function InspectorServerPicker({
	serverName,
	connected = false,
	restoring = false,
	onOpenConnect,
	className,
}: InspectorServerPickerProps) {
	const { t } = useTranslation("inspector");
	const hasActiveServer = (connected || restoring) && !!serverName;

	if (!hasActiveServer) {
		return (
			<button
				type="button"
				className={cn(
					"flex h-9 w-full items-center gap-2 rounded-lg border border-dashed border-slate-200 bg-slate-50 px-3 text-sm font-medium text-slate-600 transition-colors hover:bg-slate-100 dark:border-slate-700 dark:bg-slate-900/40 dark:text-slate-300 dark:hover:bg-slate-900/60",
					className,
				)}
				onClick={onOpenConnect}
			>
				<Plug className="h-4 w-4 shrink-0" aria-hidden />
				<span className="min-w-0 flex-1 truncate text-left">
					{t("connect.connectServer", { defaultValue: "Connect Server" })}
				</span>
				<span className="shrink-0 text-slate-400 dark:text-slate-500" aria-hidden>···</span>
			</button>
		);
	}

	return (
		<button
			type="button"
			className={cn(
				"flex h-9 w-full items-center gap-2 rounded-lg border px-3 text-sm font-medium transition-colors",
				restoring
					? "border-sky-300 bg-sky-50 text-sky-700 hover:bg-sky-100 dark:border-sky-700 dark:bg-sky-950/40 dark:text-sky-300 dark:hover:bg-sky-950/60"
					: "border-emerald-300 bg-emerald-50 text-emerald-700 hover:bg-emerald-100 dark:border-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-300 dark:hover:bg-emerald-950/60",
				className,
			)}
			onClick={onOpenConnect}
		>
			{restoring ? (
				<RefreshCcw
					className="h-3.5 w-3.5 shrink-0 animate-spin text-sky-500 dark:text-sky-400"
					aria-hidden
				/>
			) : (
				<CircleDot
					className="h-3.5 w-3.5 shrink-0 text-emerald-500 dark:text-emerald-400"
					aria-hidden
				/>
			)}
			<span className="min-w-0 flex-1 truncate text-left">{serverName}</span>
			<span
				className={cn(
					"shrink-0 text-[10px] font-semibold uppercase tracking-wide",
					restoring
						? "text-sky-500 dark:text-sky-400"
						: "text-emerald-500 dark:text-emerald-400",
				)}
			>
				{restoring ? "Restoring" : "Ready"}
			</span>
		</button>
	);
}
