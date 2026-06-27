import { ChevronDown } from "lucide-react";
import type { CSSProperties, ReactNode } from "react";
import { cn } from "../lib/utils";
import {
	DropdownMenu,
	DropdownMenuCheckboxItem,
	DropdownMenuContent,
	DropdownMenuTrigger,
} from "./ui/dropdown-menu";
import { Input } from "./ui/input";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "./ui/select";

export type CapabilityToolbarOption = {
	value: string;
	label: string;
	title?: string;
};

export type CapabilityToolbarMultiFilter = {
	label: string;
	allLabel: string;
	options: CapabilityToolbarOption[];
	selectedValues: string[];
	onClear: () => void;
	onToggle: (value: string, checked: boolean) => void;
	contentClassName?: string;
};

export type CapabilityToolbarSingleFilter = {
	label: string;
	value: string;
	placeholder?: string;
	options: CapabilityToolbarOption[];
	onValueChange: (value: string) => void;
	contentClassName?: string;
};

type CapabilityToolbarProps = {
	searchValue: string;
	onSearchChange: (value: string) => void;
	searchPlaceholder: string;
	kindFilter: CapabilityToolbarMultiFilter;
	serverFilter?: CapabilityToolbarMultiFilter;
	statusFilter?: CapabilityToolbarSingleFilter;
	action?: ReactNode;
	className?: string;
	containedFocus?: boolean;
};

const compactDropdownTriggerClass =
	"relative flex h-9 w-full min-w-9 items-center rounded-md border border-input bg-background px-2 pr-8 text-left text-sm ring-offset-background transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2";
const containedFocusClass =
	"focus:ring-inset focus:ring-offset-0 focus-visible:ring-inset focus-visible:ring-offset-0";
const compactDropdownLabelClass = "min-w-0 flex-1 truncate";
const compactDropdownIconClass =
	"pointer-events-none absolute right-2.5 top-1/2 h-4 w-4 -translate-y-1/2 shrink-0 opacity-50";
const compactSelectTriggerClass =
	"relative h-9 w-full min-w-9 px-2 pr-8 [&>span]:min-w-0 [&>span]:truncate [&>svg]:pointer-events-none [&>svg]:absolute [&>svg]:right-2.5 [&>svg]:top-1/2 [&>svg]:-translate-y-1/2";

function CapabilityToolbarMultiSelect({
	filter,
	containedFocus,
}: {
	filter: CapabilityToolbarMultiFilter;
	containedFocus?: boolean;
}) {
	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<button
					type="button"
					title={filter.label}
					className={cn(
						compactDropdownTriggerClass,
						containedFocus && containedFocusClass,
					)}
				>
					<span className={compactDropdownLabelClass}>{filter.label}</span>
					<ChevronDown className={compactDropdownIconClass} />
				</button>
			</DropdownMenuTrigger>
			<DropdownMenuContent
				align="start"
				className={cn(
					"w-max min-w-[var(--radix-dropdown-menu-trigger-width)] max-w-[min(22rem,var(--radix-dropdown-menu-content-available-width))]",
					filter.contentClassName,
				)}
			>
				<DropdownMenuCheckboxItem
					checked={filter.selectedValues.length === 0}
					onCheckedChange={() => filter.onClear()}
					onSelect={(event) => event.preventDefault()}
				>
					{filter.allLabel}
				</DropdownMenuCheckboxItem>
				{filter.options.map((option) => (
					<DropdownMenuCheckboxItem
						key={option.value}
						checked={filter.selectedValues.includes(option.value)}
						onCheckedChange={(checked) =>
							filter.onToggle(option.value, checked === true)
						}
						onSelect={(event) => event.preventDefault()}
					>
						<span className="truncate" title={option.title ?? option.label}>
							{option.label}
						</span>
					</DropdownMenuCheckboxItem>
				))}
			</DropdownMenuContent>
		</DropdownMenu>
	);
}

function CapabilityToolbarSingleSelect({
	filter,
	containedFocus,
}: {
	filter: CapabilityToolbarSingleFilter;
	containedFocus?: boolean;
}) {
	return (
		<Select value={filter.value} onValueChange={filter.onValueChange}>
			<SelectTrigger
				title={filter.label}
				className={cn(
					compactSelectTriggerClass,
					containedFocus && containedFocusClass,
				)}
			>
				<SelectValue placeholder={filter.placeholder} />
			</SelectTrigger>
			<SelectContent className={cn("min-w-[8rem]", filter.contentClassName)}>
				{filter.options.map((option) => (
					<SelectItem key={option.value} value={option.value}>
						{option.label}
					</SelectItem>
				))}
			</SelectContent>
		</Select>
	);
}

export function CapabilityToolbar({
	searchValue,
	onSearchChange,
	searchPlaceholder,
	kindFilter,
	serverFilter,
	statusFilter,
	action,
	className,
	containedFocus = false,
}: CapabilityToolbarProps) {
	const gridColumns = [
		"minmax(8rem,2fr)",
		serverFilter ? "minmax(2.25rem,0.72fr)" : null,
		"minmax(2.25rem,0.72fr)",
		statusFilter ? "minmax(2.25rem,0.72fr)" : null,
		action ? "max-content" : null,
	]
		.filter(Boolean)
		.join(" ");
	const style = {
		gridTemplateColumns: gridColumns,
	} satisfies CSSProperties;

	return (
		<div
			className={cn("grid min-w-0 items-center gap-2", className)}
			style={style}
		>
			<Input
				placeholder={searchPlaceholder}
				value={searchValue}
				onChange={(event) => onSearchChange(event.target.value)}
				className={cn("h-9 min-w-0", containedFocus && containedFocusClass)}
			/>
			{serverFilter ? (
				<CapabilityToolbarMultiSelect
					filter={serverFilter}
					containedFocus={containedFocus}
				/>
			) : null}
			<CapabilityToolbarMultiSelect
				filter={kindFilter}
				containedFocus={containedFocus}
			/>
			{statusFilter ? (
				<CapabilityToolbarSingleSelect
					filter={statusFilter}
					containedFocus={containedFocus}
				/>
			) : null}
			{action ? (
				<div className="flex min-w-0 items-center justify-end gap-2">
					{action}
				</div>
			) : null}
		</div>
	);
}
