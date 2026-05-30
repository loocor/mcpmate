import { Copy, Play, RotateCcw, Square } from "lucide-react";
import React from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "../../components/ui/tooltip";
import { writeClipboardText } from "../../lib/clipboard";
import { notifySuccess } from "../../lib/notify";
import { cn } from "../../lib/utils";

const noDragRegionStyle = { WebkitAppRegion: "no-drag" } as React.CSSProperties;

const CORE_CONTROL_PILL_BASE =
	"h-7 min-w-0 flex-1 gap-1.5 rounded-full px-3 text-xs font-medium shadow-sm transition-all duration-200 hover:shadow-md active:scale-[0.98] disabled:pointer-events-none disabled:opacity-50 disabled:shadow-none";

const CORE_RESTART_PILL_CLASS = cn(
	CORE_CONTROL_PILL_BASE,
	"border border-slate-200/80 bg-white text-slate-700 hover:border-slate-300 hover:bg-slate-50 dark:border-slate-700 dark:bg-slate-950 dark:text-slate-200 dark:hover:border-slate-600 dark:hover:bg-slate-900",
);

const CORE_STOP_PILL_CLASS = cn(
	CORE_CONTROL_PILL_BASE,
	"border-0 bg-red-500 text-white shadow-red-500/20 hover:bg-red-600 hover:shadow-red-500/30 dark:bg-red-600 dark:hover:bg-red-500",
);

const CORE_START_PILL_CLASS = cn(
	CORE_CONTROL_PILL_BASE,
	"border-0 bg-emerald-600 text-white shadow-emerald-600/20 hover:bg-emerald-700 hover:shadow-emerald-600/30 dark:bg-emerald-600 dark:hover:bg-emerald-700",
);

export function OperatorCoreRowDetail({
	busyAction,
	detailId,
	isTauriShell,
	mcpEndpointUrl,
	mcpEndpointLoading,
	onRestart,
	onToggleService,
	restartAvailable,
	serviceControlsAvailable,
	serviceRunning,
}: {
	busyAction: "start" | "stop" | "restart" | null;
	detailId: string;
	isTauriShell: boolean;
	mcpEndpointUrl: string | null;
	mcpEndpointLoading: boolean;
	onRestart: () => void;
	onToggleService: () => void;
	restartAvailable: boolean;
	serviceControlsAvailable: boolean;
	serviceRunning: boolean;
}) {
	const { t } = useTranslation();
	const controlsBusy = busyAction !== null;
	const serviceControlDisabled = !serviceControlsAvailable || controlsBusy;
	const restartDisabled = !restartAvailable || controlsBusy || mcpEndpointLoading;

	const restartLabel = t("operator:detail.core.controls.restart", {
		defaultValue: "Restart",
	});
	const startLabel = t("operator:detail.core.controls.start", {
		defaultValue: "Start",
	});
	const stopLabel = t("operator:detail.core.controls.stop", {
		defaultValue: "Stop",
	});
	const serviceLabel = serviceRunning ? stopLabel : startLabel;
	const desktopOnlyHint = t("operator:detail.core.serviceDesktopOnly", {
		defaultValue: "Start and stop are available in MCPMate Desktop.",
	});
	const localSourceOnlyHint = t("operator:detail.core.localSourceOnly", {
		defaultValue:
			"Local Core controls are available only when Desktop uses the localhost source.",
	});
	const disabledControlHint = isTauriShell ? localSourceOnlyHint : desktopOnlyHint;
	const copyLabel = t("operator:detail.core.copyEndpoint", {
		defaultValue: "Copy MCPMate Server Endpoint",
	});

	const handleCopyEndpoint = React.useCallback(async () => {
		if (!mcpEndpointUrl) {
			return;
		}
		await writeClipboardText(mcpEndpointUrl);
		notifySuccess(
			t("operator:detail.core.copySuccess", {
				defaultValue: "Endpoint copied",
			}),
			t("operator:detail.core.copySuccessMessage", {
				defaultValue:
					"MCPMate Server Endpoint (Streamable HTTP) copied to clipboard.",
			}),
		);
	}, [mcpEndpointUrl, t]);

	const serviceButton = (
		<Button
			type="button"
			variant={serviceRunning ? "destructive" : "default"}
			size="sm"
			className={serviceRunning ? CORE_STOP_PILL_CLASS : CORE_START_PILL_CLASS}
			style={noDragRegionStyle}
			disabled={serviceControlDisabled}
			aria-label={serviceLabel}
			onClick={(event) => {
				event.stopPropagation();
				onToggleService();
			}}
		>
			{serviceRunning ? (
				<Square className="h-3 w-3 shrink-0" aria-hidden />
			) : (
				<Play className="h-3 w-3 shrink-0" aria-hidden />
			)}
			<span className="truncate">{serviceLabel}</span>
		</Button>
	);

	return (
		<div
			id={detailId}
			className="border-t border-slate-100 px-3 py-2.5 dark:border-slate-800"
			data-testid="operator-inline-detail"
		>
			<div className="flex items-center gap-2">
				{serviceControlDisabled && !controlsBusy ? (
					<Tooltip>
						<TooltipTrigger asChild>
							<span className="flex min-w-0 flex-1">
								{serviceButton}
							</span>
						</TooltipTrigger>
						<TooltipContent side="top" className="max-w-[220px] text-xs">
							{disabledControlHint}
						</TooltipContent>
					</Tooltip>
				) : (
					serviceButton
				)}
				<Button
					type="button"
					variant="outline"
					size="sm"
					className={CORE_RESTART_PILL_CLASS}
					style={noDragRegionStyle}
					disabled={restartDisabled}
					aria-label={restartLabel}
					onClick={(event) => {
						event.stopPropagation();
						onRestart();
					}}
				>
					<RotateCcw
						className={cn(
							"h-3 w-3 shrink-0",
							busyAction === "restart" && "animate-spin",
						)}
						aria-hidden
					/>
					<span className="truncate">{restartLabel}</span>
				</Button>
			</div>

			<div className="mt-2.5 flex items-baseline justify-between gap-2">
				<Tooltip>
					<TooltipTrigger asChild>
						<p className="cursor-default text-[10px] font-semibold uppercase tracking-wide text-slate-400 dark:text-slate-500">
							{t("operator:detail.core.mcpEndpoint", {
								defaultValue: "Server endpoint",
							})}
						</p>
					</TooltipTrigger>
					<TooltipContent side="top" className="max-w-[240px] text-xs">
						{t("operator:detail.core.mcpEndpointTooltip", {
							defaultValue: "MCPMate Server Endpoint (Streamable HTTP)",
						})}
					</TooltipContent>
				</Tooltip>
				<span className="shrink-0 text-[10px] text-slate-400/90 dark:text-slate-500/90">
					{t("operator:detail.core.mcpEndpointTransport", {
						defaultValue: "Streamable HTTP",
					})}
				</span>
			</div>
			<div className="group relative mt-1 w-full">
				<p
					className="w-full truncate rounded border border-slate-100 bg-slate-50 px-2 py-1 pr-7 font-mono text-[11px] leading-4 text-slate-700 dark:border-slate-800 dark:bg-slate-900/70 dark:text-slate-300"
					title={
						mcpEndpointUrl ??
						t("operator:detail.core.mcpEndpointTooltip", {
							defaultValue: "MCPMate Server Endpoint (Streamable HTTP)",
						})
					}
					data-testid="operator-core-mcp-endpoint"
				>
					{mcpEndpointLoading
						? t("operator:detail.core.mcpEndpointLoading", {
								defaultValue: "Loading endpoint…",
							})
						: (mcpEndpointUrl ??
							t("operator:detail.core.mcpEndpointUnavailable", {
								defaultValue: "Endpoint unavailable",
							}))}
				</p>
				<Tooltip>
					<TooltipTrigger asChild>
						<Button
							type="button"
							variant="ghost"
							size="icon"
							className="absolute right-0.5 top-1/2 h-6 w-6 -translate-y-1/2 border-0 bg-transparent p-0 text-slate-400 opacity-0 shadow-none transition-opacity group-hover:opacity-100 group-focus-within:opacity-100 hover:bg-transparent hover:text-slate-700 disabled:pointer-events-none disabled:opacity-0 dark:text-slate-500 dark:hover:bg-transparent dark:hover:text-slate-200"
							style={noDragRegionStyle}
							disabled={!mcpEndpointUrl || mcpEndpointLoading}
							aria-label={copyLabel}
							onClick={(event) => {
								event.stopPropagation();
								void handleCopyEndpoint();
							}}
						>
							<Copy className="h-3 w-3" aria-hidden />
						</Button>
					</TooltipTrigger>
					<TooltipContent side="left">{copyLabel}</TooltipContent>
				</Tooltip>
			</div>
		</div>
	);
}
