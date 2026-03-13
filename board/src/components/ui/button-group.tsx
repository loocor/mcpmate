"use client";

import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "../../lib/utils";

const buttonGroupVariants = cva(
	"inline-flex items-center justify-center gap-0 [&>*:not(:first-child)]:ml-[-1px] [&>*:not(:first-child)]:rounded-l-none [&>*:not(:last-child)]:rounded-r-none [&>*:first-child]:rounded-r-none [&>*:last-child]:rounded-l-none",
	{
		variants: {
			orientation: {
				horizontal: "flex-row",
				vertical:
					"flex-col [&>*:not(:first-child)]:mt-[-1px] [&>*:not(:first-child)]:rounded-t-none [&>*:not(:last-child)]:rounded-b-none [&>*:first-child]:rounded-b-none [&>*:last-child]:rounded-t-none",
			},
		},
		defaultVariants: {
			orientation: "horizontal",
		},
	},
);

export interface ButtonGroupProps
	extends React.HTMLAttributes<HTMLDivElement>,
		VariantProps<typeof buttonGroupVariants> {}

const ButtonGroup = React.forwardRef<HTMLDivElement, ButtonGroupProps>(
	({ className, orientation, ...props }, ref) => (
		<div
			ref={ref}
			className={cn(buttonGroupVariants({ orientation }), className)}
			{...props}
		/>
	),
);
ButtonGroup.displayName = "ButtonGroup";

const ButtonGroupSeparator = React.forwardRef<
	HTMLDivElement,
	React.HTMLAttributes<HTMLDivElement> & {
		orientation?: "horizontal" | "vertical";
	}
>(({ className, orientation = "vertical", ...props }, ref) => (
	<div
		ref={ref}
		role="separator"
		aria-orientation={orientation}
		className={cn(
			"shrink-0 bg-border",
			orientation === "horizontal" ? "h-full w-px" : "h-px w-full",
		)}
		{...props}
	/>
));
ButtonGroupSeparator.displayName = "ButtonGroupSeparator";

const ButtonGroupText = React.forwardRef<
	HTMLDivElement,
	React.HTMLAttributes<HTMLDivElement> & {
		asChild?: boolean;
	}
>(({ className, asChild = false, ...props }, ref) => {
	const Comp = asChild ? React.Fragment : "div";
	return (
		<Comp
			ref={asChild ? undefined : ref}
			className={cn(
				"flex h-9 items-center justify-center whitespace-nowrap rounded-md border border-input bg-background px-3 py-1 text-sm font-medium text-foreground shadow-sm",
				className,
			)}
			{...props}
		/>
	);
});
ButtonGroupText.displayName = "ButtonGroupText";

export { ButtonGroup, ButtonGroupSeparator, ButtonGroupText };
