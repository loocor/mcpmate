"use client";

import * as TooltipPrimitive from "@radix-ui/react-tooltip";
import * as React from "react";

import { cn } from "../../lib/utils";

const TooltipProvider = TooltipPrimitive.Provider;

const Tooltip = TooltipPrimitive.Root;

const TooltipTrigger = TooltipPrimitive.Trigger;

const TooltipPortal = TooltipPrimitive.Portal;

const TooltipContent = React.forwardRef<
	React.ElementRef<typeof TooltipPrimitive.Content>,
	React.ComponentPropsWithoutRef<typeof TooltipPrimitive.Content>
>(({ className, sideOffset = 4, ...props }, ref) => (
	<TooltipPrimitive.Content
		ref={ref}
		sideOffset={sideOffset}
		className={cn(
			"z-50 overflow-hidden rounded-md border bg-foreground px-3 py-1.5 text-xs text-background shadow-md animate-in fade-in-0 zoom-in-95",
			className,
		)}
		{...props}
	/>
));
TooltipContent.displayName = TooltipPrimitive.Content.displayName;

const TooltipArrow = React.forwardRef<
	React.ElementRef<typeof TooltipPrimitive.Arrow>,
	React.ComponentPropsWithoutRef<typeof TooltipPrimitive.Arrow>
>(({ className, width = 10, height = 5, ...props }, ref) => (
	<TooltipPrimitive.Arrow
		ref={ref}
		width={width}
		height={height}
		className={cn("fill-background", className)}
		{...props}
	/>
));
TooltipArrow.displayName = TooltipPrimitive.Arrow.displayName;

export {
	Tooltip,
	TooltipTrigger,
	TooltipPortal,
	TooltipContent,
	TooltipArrow,
	TooltipProvider,
};
