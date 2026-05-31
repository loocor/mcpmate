import React from "react";
import { cn } from "../../lib/utils";

export const OPERATOR_PANEL_FRAME_CLASS =
	"mx-auto box-border flex h-dvh w-full max-w-[420px] bg-transparent p-2";

export const OPERATOR_PANEL_SHADOW_CLASS =
	"h-full min-h-0 w-full rounded-xl bg-transparent drop-shadow-[0_16px_40px_rgba(15,23,42,0.22),0_4px_14px_rgba(15,23,42,0.12)] dark:drop-shadow-[0_16px_40px_rgba(0,0,0,0.55),0_4px_14px_rgba(0,0,0,0.35)]";

export const OPERATOR_PANEL_SHELL_CLASS =
	"relative flex h-full min-h-0 w-full flex-col overflow-hidden rounded-xl border border-slate-900/10 bg-white text-slate-950 dark:border-white/10 dark:bg-slate-950 dark:text-slate-50";

export function OperatorPanelFrame({
	className,
	children,
	...props
}: React.ComponentProps<"div">) {
	return (
		<div
			className={cn(OPERATOR_PANEL_FRAME_CLASS, className)}
			data-operator-panel-frame="true"
			{...props}
		>
			{children}
		</div>
	);
}

export function OperatorPanelShell({
	className,
	children,
	...props
}: React.ComponentProps<"main">) {
	return (
		<div
			className={OPERATOR_PANEL_SHADOW_CLASS}
			data-operator-panel-shadow="true"
		>
			<main className={cn(OPERATOR_PANEL_SHELL_CLASS, className)} {...props}>
				{children}
			</main>
		</div>
	);
}
