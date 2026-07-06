import { ListFilter, Search, X } from "lucide-react";
import { useEffect, useRef } from "react";
import { INSPECTOR_BOTTOM_BAR_ICON_BUTTON_CLASSNAME } from "../lib/inspector-bottom-bar";
import { cn } from "../lib/utils";
import { Input } from "./ui/input";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "./ui/tooltip";

export type InspectorSearchContextFilter = {
	label: string;
	value: string;
	onClear: () => void;
	clearAriaLabel?: string;
};

export type InspectorExpandableSearchProps = {
	value: string;
	onChange: (value: string) => void;
	open: boolean;
	onOpenChange: (open: boolean) => void;
	placeholder?: string;
	ariaLabel?: string;
	clearAriaLabel?: string;
	contextFilter?: InspectorSearchContextFilter | null;
	className?: string;
	expandedClassName?: string;
	iconButtonClassName?: string;
	closeOnClickOutside?: boolean;
};

export function InspectorExpandableSearch({
	value,
	onChange,
	open,
	onOpenChange,
	placeholder,
	ariaLabel = "Search",
	clearAriaLabel = "Clear search",
	contextFilter = null,
	className,
	expandedClassName = "w-36 sm:w-44",
	iconButtonClassName = INSPECTOR_BOTTOM_BAR_ICON_BUTTON_CLASSNAME,
	closeOnClickOutside = false,
}: InspectorExpandableSearchProps) {
	const rootRef = useRef<HTMLDivElement | null>(null);
	const inputRef = useRef<HTMLInputElement | null>(null);
	const expanded = open || contextFilter != null;
	const hasContextFilter = contextFilter != null;

	useEffect(() => {
		if (expanded) {
			inputRef.current?.focus();
		}
	}, [expanded]);

	useEffect(() => {
		if (!expanded || !closeOnClickOutside) {
			return;
		}
		const handlePointerDown = (event: PointerEvent) => {
			if (rootRef.current?.contains(event.target as Node)) {
				return;
			}
			onOpenChange(false);
		};
		document.addEventListener("pointerdown", handlePointerDown);
		return () => document.removeEventListener("pointerdown", handlePointerDown);
	}, [closeOnClickOutside, expanded, onOpenChange]);

	if (!expanded) {
		return (
			<button
				type="button"
				className={cn(iconButtonClassName, className)}
				aria-label={ariaLabel}
				aria-expanded={false}
				onClick={() => onOpenChange(true)}
			>
				<Search className="h-3.5 w-3.5" aria-hidden />
			</button>
		);
	}

	return (
		<div
			ref={rootRef}
			className={cn(
				"group relative flex shrink-0 items-center gap-1.5 transition-[width,opacity] duration-200 ease-in-out",
				hasContextFilter ? "min-w-0" : expandedClassName,
				className,
			)}
		>
			{hasContextFilter ? (
				<TooltipProvider delayDuration={200}>
					<Tooltip>
						<TooltipTrigger asChild>
							<div
								className="group/filter flex h-7 shrink-0 items-center gap-1 rounded-full border border-border bg-muted/40 py-0 pl-1.5 pr-1 text-[10px] leading-none"
								aria-label={`${contextFilter.label}: ${contextFilter.value}`}
							>
								<ListFilter
									className="h-3 w-3 shrink-0 text-muted-foreground"
									aria-hidden
								/>
								<span className="whitespace-nowrap text-foreground">
									{contextFilter.label}
								</span>
								<button
									type="button"
									aria-label={
										contextFilter.clearAriaLabel ??
										`Clear ${contextFilter.label} filter`
									}
									className="flex h-4 w-4 shrink-0 items-center justify-center rounded-sm text-muted-foreground opacity-0 transition-opacity hover:bg-accent hover:text-foreground focus-visible:opacity-100 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring group-hover/filter:opacity-100"
									onMouseDown={(event) => event.preventDefault()}
									onClick={contextFilter.onClear}
								>
									<X className="h-3 w-3" aria-hidden />
								</button>
							</div>
						</TooltipTrigger>
						<TooltipContent side="top" align="center" className="max-w-xs font-mono text-xs">
							{contextFilter.value}
						</TooltipContent>
					</Tooltip>
				</TooltipProvider>
			) : null}
			<div
				className={cn(
					"relative shrink-0",
					hasContextFilter ? "w-36 sm:w-44" : "w-full",
				)}
			>
				<Input
					ref={inputRef}
					value={value}
					onChange={(event) => onChange(event.target.value)}
					placeholder={placeholder}
					aria-label={ariaLabel}
					className={cn(
						"h-7 rounded-full border border-input bg-background px-3 text-xs shadow-none",
						value && "pr-7",
						"focus-visible:border-muted-foreground/40 focus-visible:outline-none focus-visible:ring-0 focus-visible:ring-offset-0",
					)}
					onKeyDown={(event) => {
						if (event.key === "Escape") {
							if (value) {
								onChange("");
							} else if (contextFilter) {
								contextFilter.onClear();
							} else {
								onOpenChange(false);
							}
						}
					}}
					onBlur={() => {
						if (!value && !contextFilter) {
							onOpenChange(false);
						}
					}}
				/>
				{value ? (
					<button
						type="button"
						aria-label={clearAriaLabel}
						className="absolute right-1.5 top-1/2 flex h-4 w-4 -translate-y-1/2 items-center justify-center rounded-sm text-slate-500 opacity-0 transition-opacity hover:bg-accent hover:text-foreground focus:opacity-100 focus:outline-none focus:ring-0 group-hover:opacity-100 group-focus-within:opacity-100"
						onMouseDown={(event) => event.preventDefault()}
						onClick={() => onChange("")}
					>
						<X className="h-3 w-3" aria-hidden />
					</button>
				) : null}
			</div>
		</div>
	);
}
