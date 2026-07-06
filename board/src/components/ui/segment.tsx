import * as React from "react";
import * as TabsPrimitive from "@radix-ui/react-tabs";
import { cn } from "../../lib/utils";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "./tooltip";

export interface SegmentOption {
	value: string;
	label: string;
	/** Overrides visible label for assistive tech when `label` is non-text (e.g. emoji-only). */
	ariaLabel?: string;
	icon?: React.ReactNode;
	tooltip?: string;
	disabled?: boolean;
	status?: string;
}

export interface SegmentProps
	extends Omit<
		React.ComponentPropsWithoutRef<typeof TabsPrimitive.Root>,
		"orientation"
	> {
	options: SegmentOption[];
	value?: string;
	onValueChange?: (value: string) => void;
	showDots?: boolean;
	className?: string;
	listClassName?: string;
	triggerClassName?: string;
	dotClassName?: string;
	disabled?: boolean;
}

const Segment = React.forwardRef<
	React.ElementRef<typeof TabsPrimitive.Root>,
	SegmentProps
>(
	(
		{
			options,
			value,
			onValueChange,
			showDots = true,
			className,
			listClassName,
			triggerClassName,
			dotClassName,
			disabled = false,
			...props
		},
		ref,
	) => {
		return (
			<TabsPrimitive.Root
				ref={ref}
				value={value}
				onValueChange={onValueChange}
				orientation="horizontal"
				className={cn("w-full", className)}
				{...props}
			>
				<TabsPrimitive.List
					className={cn(
						"inline-flex h-8 items-stretch gap-0.5 rounded-md bg-slate-100 p-0.5 text-slate-500 dark:bg-slate-800 dark:text-slate-400",
						disabled && "opacity-50 pointer-events-none",
						listClassName,
					)}
				>
					{options.map((option) => {
						const isOptionDisabled = disabled || option.disabled;
						const accessibleLabel =
							option.ariaLabel ??
							(option.status ? `${option.label} (${option.status})` : option.label);

						const trigger = (
							<TabsPrimitive.Trigger
								key={option.value}
								value={option.value}
								disabled={isOptionDisabled}
								aria-label={accessibleLabel}
								className={cn(
									"group inline-flex flex-1 items-center justify-center self-stretch whitespace-nowrap rounded-sm px-3 py-0 text-sm font-medium leading-none ring-offset-white transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-slate-950 focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 data-[state=active]:bg-white data-[state=active]:text-slate-950 data-[state=active]:shadow-sm dark:ring-offset-slate-950 dark:focus-visible:ring-slate-300 dark:data-[state=active]:bg-slate-950 dark:data-[state=active]:text-slate-50",
									"flex items-center gap-2",
									option.tooltip && "w-full",
									option.disabled && "opacity-50 cursor-not-allowed",
									triggerClassName,
								)}
							>
								{showDots && (
									<div
										className={cn(
											"size-3 shrink-0 rounded-full border-2 transition-all",
											"border-slate-400 bg-transparent",
											"dark:border-slate-500",
											value === option.value &&
											"border-primary bg-primary dark:border-primary dark:bg-primary",
											option.disabled && "opacity-50",
											dotClassName,
										)}
									/>
								)}
								{option.icon && (
									<span className="flex-shrink-0">{option.icon}</span>
								)}
								<span className="min-w-0 text-center leading-tight">
									{option.label}
									{option.status ? (
										<sup className="ml-0.5">
											<span
												className={cn(
													"inline-flex h-3.5 min-w-3.5 items-center justify-center rounded-full border px-1 text-[9px] font-semibold leading-none tabular-nums",
													"border-slate-300/80 bg-white/90 text-slate-500",
													"group-data-[state=active]:border-slate-300 group-data-[state=active]:bg-slate-50 group-data-[state=active]:text-slate-700",
													"dark:border-slate-600 dark:bg-slate-900/70 dark:text-slate-400",
													"dark:group-data-[state=active]:border-slate-500 dark:group-data-[state=active]:bg-slate-800 dark:group-data-[state=active]:text-slate-200",
												)}
											>
												{option.status}
											</span>
										</sup>
									) : null}
								</span>
							</TabsPrimitive.Trigger>
						);

						if (option.tooltip) {
							return (
								<TooltipProvider key={option.value} delayDuration={200}>
									<Tooltip>
										<TooltipTrigger asChild>
											<span className="flex flex-1">{trigger}</span>
										</TooltipTrigger>
										<TooltipContent
											side="top"
											align="center"
											className="max-w-xs leading-relaxed"
										>
											{option.tooltip}
										</TooltipContent>
									</Tooltip>
								</TooltipProvider>
							);
						}

						return trigger;
					})}
				</TabsPrimitive.List>
			</TabsPrimitive.Root>
		);
	},
);

Segment.displayName = "Segment";

export { Segment };
