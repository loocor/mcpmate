import { Search, X } from "lucide-react";
import { useEffect, useRef } from "react";
import { INSPECTOR_BOTTOM_BAR_ICON_BUTTON_CLASSNAME } from "../lib/inspector-bottom-bar";
import { cn } from "../lib/utils";
import { Input } from "./ui/input";

export type InspectorExpandableSearchProps = {
	value: string;
	onChange: (value: string) => void;
	open: boolean;
	onOpenChange: (open: boolean) => void;
	placeholder?: string;
	ariaLabel?: string;
	clearAriaLabel?: string;
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
	className,
	expandedClassName = "w-36 sm:w-44",
	iconButtonClassName = INSPECTOR_BOTTOM_BAR_ICON_BUTTON_CLASSNAME,
	closeOnClickOutside = false,
}: InspectorExpandableSearchProps) {
	const rootRef = useRef<HTMLDivElement | null>(null);
	const inputRef = useRef<HTMLInputElement | null>(null);

	useEffect(() => {
		if (open) {
			inputRef.current?.focus();
		}
	}, [open]);

	useEffect(() => {
		if (!open || !closeOnClickOutside) {
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
	}, [closeOnClickOutside, onOpenChange, open]);

	if (!open) {
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
				"group relative min-w-0 shrink-0 transition-[width,opacity] duration-200 ease-in-out",
				expandedClassName,
				className,
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
						} else {
							onOpenChange(false);
						}
					}
				}}
				onBlur={() => {
					if (!value) {
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
	);
}
