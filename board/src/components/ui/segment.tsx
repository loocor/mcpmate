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
						"inline-flex h-10 w-full items-center justify-center rounded-md bg-slate-100 p-1 text-slate-500 dark:bg-slate-800 dark:text-slate-400",
						disabled && "opacity-50 pointer-events-none",
					)}
				>
					{options.map((option) => {
						const isOptionDisabled = disabled || option.disabled;
						const displayLabel = option.status
							? `${option.label} (${option.status})`
							: option.label;

						const trigger = (
							<TabsPrimitive.Trigger
								key={option.value}
								value={option.value}
								disabled={isOptionDisabled}
								className={cn(
									"inline-flex flex-1 items-center justify-center whitespace-nowrap rounded-sm px-3 py-1.5 text-sm font-medium ring-offset-white transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-slate-950 focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 data-[state=active]:bg-white data-[state=active]:text-slate-950 data-[state=active]:shadow-sm dark:ring-offset-slate-950 dark:focus-visible:ring-slate-300 dark:data-[state=active]:bg-slate-950 dark:data-[state=active]:text-slate-50",
									"flex items-center gap-2",
									option.disabled && "opacity-50 cursor-not-allowed",
								)}
							>
								{showDots && (
									<div
										className={cn(
											"w-3 h-3 rounded-full border-2 transition-all",
											"border-slate-400 bg-transparent",
											"dark:border-slate-500",
											value === option.value && "border-primary bg-primary",
											value === option.value &&
												"dark:border-primary dark:bg-primary",
											option.disabled && "opacity-50",
										)}
									/>
								)}
								{option.icon && (
									<span className="flex-shrink-0">{option.icon}</span>
								)}
								<span>{displayLabel}</span>
							</TabsPrimitive.Trigger>
						);

						if (option.tooltip) {
							return (
								<TooltipProvider key={option.value} delayDuration={200}>
									<Tooltip>
										<TooltipTrigger asChild>{trigger}</TooltipTrigger>
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
