import React from "react";

import { cn } from "../../lib/utils";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "./select";

export interface PageToolbarSelectOption {
	value: string;
	label: string;
}

export interface PageToolbarSelectProps {
	value: string;
	onValueChange: (value: string) => void;
	options: PageToolbarSelectOption[];
	placeholder?: string;
	"aria-label"?: string;
	className?: string;
	triggerClassName?: string;
}

export function getPageToolbarSelectLabel(
	value: string,
	options: PageToolbarSelectOption[],
	placeholder?: string,
): string {
	return options.find((option) => option.value === value)?.label ?? placeholder ?? "";
}

const mirrorClassName =
	"invisible col-start-1 row-start-1 flex h-9 items-center whitespace-pre border border-transparent px-3 pr-8 text-sm leading-none";

export function PageToolbarSelect({
	value,
	onValueChange,
	options,
	placeholder,
	"aria-label": ariaLabel,
	className,
	triggerClassName,
}: PageToolbarSelectProps) {
	const selectedLabel = getPageToolbarSelectLabel(value, options, placeholder);

	return (
		<div className={cn("inline-grid", className)}>
			<span className={mirrorClassName} aria-hidden="true">
				{selectedLabel}
			</span>
			<div className="col-start-1 row-start-1 min-w-0">
				<Select value={value} onValueChange={onValueChange}>
					<SelectTrigger
						className={cn(
							"h-9 w-full min-w-0 border-slate-200 dark:border-slate-700 [&>span]:line-clamp-none",
							triggerClassName,
						)}
						aria-label={ariaLabel}
					>
						<SelectValue placeholder={placeholder}>{selectedLabel}</SelectValue>
					</SelectTrigger>
					<SelectContent align="end">
						{options.map((option) => (
							<SelectItem key={option.value} value={option.value}>
								{option.label}
							</SelectItem>
						))}
					</SelectContent>
				</Select>
			</div>
		</div>
	);
}
