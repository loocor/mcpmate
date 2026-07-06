import type { ReactNode } from "react";
import { HelpCircle, Star } from "lucide-react";
import { CapsuleStripeList, CapsuleStripeListItem } from "../../components/capsule-stripe-list";
import { Segment, type SegmentOption } from "../../components/ui/segment";
import { Input } from "../../components/ui/input";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../../components/ui/select";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "../../components/ui/tooltip";
import { cn } from "../../lib/utils";

const inspectorSidebarSegmentListClassName =
	"min-h-8 h-auto w-full gap-1 bg-muted p-1 text-muted-foreground dark:bg-muted";

const inspectorSidebarSegmentTriggerClassName =
	"min-w-0 basis-0 gap-1 p-1.5 text-[11px] leading-tight transition-colors group-hover:text-muted-foreground group-hover:opacity-100 data-[state=inactive]:text-muted-foreground data-[state=inactive]:opacity-75 data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-sm dark:data-[state=active]:bg-background dark:data-[state=active]:text-foreground";

const inspectorSidebarSegmentDotClassName =
	"group-hover:border-slate-400 group-hover:bg-slate-400 dark:group-hover:border-slate-500 dark:group-hover:bg-slate-500";

const inspectorSidebarChoiceGridShellClassName =
	"grid w-full grid-cols-[repeat(auto-fill,minmax(5.25rem,1fr))] gap-1 rounded-md bg-muted p-1 text-muted-foreground dark:bg-muted";

const inspectorSidebarChoiceCellClassName = (selected: boolean) =>
	cn(
		"group inline-flex w-full items-center justify-center gap-1 rounded-sm px-1.5 py-1.5 text-[11px] font-medium leading-tight transition-colors",
		"focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
		"group-hover:text-muted-foreground group-hover:opacity-100",
		selected
			? "bg-background text-foreground shadow-sm ring-1 ring-border/80 dark:bg-background dark:text-foreground"
			: "bg-transparent text-muted-foreground opacity-75",
	);

function InspectorSidebarChoiceDot({ selected }: { selected: boolean }) {
	return (
		<div
			aria-hidden
			className={cn(
				"size-3 shrink-0 rounded-full border-2 transition-all",
				"border-slate-400 bg-transparent dark:border-slate-500",
				selected && "border-primary bg-primary dark:border-primary dark:bg-primary",
				"group-hover:border-slate-400 group-hover:bg-slate-400 dark:group-hover:border-slate-500 dark:group-hover:bg-slate-500",
			)}
		/>
	);
}

function InspectorSidebarChoiceGrid<T extends string>({
	options,
	mode,
	value,
	selected,
	onValueChange,
	onSelectedChange,
}: {
	options: Array<{ value: T; label: string }>;
	mode: "single" | "multiple";
	value?: T;
	selected?: T[];
	onValueChange?: (value: T) => void;
	onSelectedChange?: (value: T[]) => void;
}) {
	if (mode === "single") {
		return (
			<div role="radiogroup" className={inspectorSidebarChoiceGridShellClassName}>
				{options.map((option) => {
					const isSelected = value === option.value;
					return (
						<button
							key={option.value}
							type="button"
							role="radio"
							aria-checked={isSelected}
							onClick={() => onValueChange?.(option.value)}
							className={inspectorSidebarChoiceCellClassName(isSelected)}
						>
							<InspectorSidebarChoiceDot selected={isSelected} />
							<span className="min-w-0 truncate text-center leading-tight">{option.label}</span>
						</button>
					);
				})}
			</div>
		);
	}

	const activeValues = selected ?? [];

	const toggle = (optionValue: T) => {
		if (activeValues.includes(optionValue)) {
			onSelectedChange?.(activeValues.filter((entry) => entry !== optionValue));
			return;
		}
		onSelectedChange?.([...activeValues, optionValue]);
	};

	return (
		<div role="group" className={inspectorSidebarChoiceGridShellClassName}>
			{options.map((option) => {
				const isSelected = activeValues.includes(option.value);
				return (
					<button
						key={option.value}
						type="button"
						aria-pressed={isSelected}
						onClick={() => toggle(option.value)}
						className={inspectorSidebarChoiceCellClassName(isSelected)}
					>
						<InspectorSidebarChoiceDot selected={isSelected} />
						<span className="min-w-0 truncate text-center leading-tight">{option.label}</span>
					</button>
				);
			})}
		</div>
	);
}

export function InspectorSidebarSettingsShell({
	children,
	notes,
}: {
	children: ReactNode;
	notes?: ReactNode;
}) {
	return (
		<div className="flex min-h-0 flex-1 flex-col">
			<div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto">{children}</div>
			{notes ? (
				<p className="mt-auto shrink-0 pt-3 text-xs leading-relaxed text-muted-foreground">
					{notes}
				</p>
			) : null}
		</div>
	);
}

export function InspectorSidebarSectionTitle({
	children,
	className,
}: {
	children: ReactNode;
	className?: string;
}) {
	return (
		<p
			className={cn(
				"truncate py-1.5 text-sm font-medium text-foreground",
				className,
			)}
		>
			{children}
		</p>
	);
}

export function InspectorSidebarSectionHeader({
	title,
	tooltip,
}: {
	title: string;
	tooltip: ReactNode;
}) {
	return (
		<div className="flex items-center gap-2 py-1.5">
			<p className="min-w-0 flex-1 truncate text-sm font-medium text-foreground">
				{title}
			</p>
			<TooltipProvider delayDuration={200}>
				<Tooltip>
					<TooltipTrigger asChild>
						<button
							type="button"
							aria-label={`About ${title}`}
							className="inline-flex shrink-0 items-center justify-center rounded-sm text-muted-foreground opacity-75 outline-none transition-opacity hover:opacity-100 focus-visible:ring-1 focus-visible:ring-ring"
						>
							<HelpCircle className="h-3.5 w-3.5" />
						</button>
					</TooltipTrigger>
					<TooltipContent
						side="right"
						align="start"
						className="max-w-xs space-y-1.5 text-left text-xs leading-relaxed"
					>
						{tooltip}
					</TooltipContent>
				</Tooltip>
			</TooltipProvider>
		</div>
	);
}

export function InspectorSidebarOptionTooltipBody({
	summary,
	options,
}: {
	summary: string;
	options: Array<{ label: string; description: string }>;
}) {
	return (
		<>
			<p>{summary}</p>
			{options.map((option) => (
				<div key={option.label}>
					<span className="font-medium">{option.label}</span>
					<span className="text-muted-foreground"> — {option.description}</span>
				</div>
			))}
		</>
	);
}

export function inspectorSidebarSegmentTooltipOptions<
	T extends { segmentLabel: string; label: string; description: string },
>(options: T[]) {
	return options.map((option) => ({
		label: option.segmentLabel,
		description:
			option.label === option.segmentLabel
				? option.description
				: `${option.label}. ${option.description}`,
	}));
}

function inspectorSidebarSegmentOptions<T extends string>(
	options: Array<{ value: T; label: string; ariaLabel?: string }>,
): SegmentOption[] {
	return options.map((option) => ({
		value: option.value,
		label: option.label,
		...(option.ariaLabel ? { ariaLabel: option.ariaLabel } : {}),
	}));
}

/** Standard Segment row — best for 2–3 compact options (e.g. Safety). */
export function InspectorSidebarSegmentControl<T extends string>({
	options,
	value,
	onValueChange,
}: {
	options: Array<{ value: T; label: string; ariaLabel?: string }>;
	value: T;
	onValueChange: (value: T) => void;
}) {
	return (
		<Segment
			options={inspectorSidebarSegmentOptions(options)}
			value={value}
			onValueChange={(next) => onValueChange(next as T)}
			showDots
			listClassName={inspectorSidebarSegmentListClassName}
			triggerClassName={inspectorSidebarSegmentTriggerClassName}
			dotClassName={inspectorSidebarSegmentDotClassName}
		/>
	);
}

/** Auto-wrapping choice grid with dot indicators — for wider option sets (e.g. Compact). */
export function InspectorSidebarChoiceGridControl<T extends string>({
	options,
	value,
	onValueChange,
}: {
	options: Array<{ value: T; label: string }>;
	value: T;
	onValueChange: (value: T) => void;
}) {
	return (
		<InspectorSidebarChoiceGrid
			mode="single"
			options={options}
			value={value}
			onValueChange={onValueChange}
		/>
	);
}

/** Auto-wrapping multi-select choice grid — for LLM focus dimensions. */
export function InspectorSidebarMultiSegmentControl<T extends string>({
	options,
	selected,
	onChange,
}: {
	options: Array<{ value: T; label: string }>;
	selected: T[];
	onChange: (value: T[]) => void;
}) {
	return (
		<InspectorSidebarChoiceGrid
			mode="multiple"
			options={options}
			selected={selected}
			onSelectedChange={onChange}
		/>
	);
}

type InspectorSidebarListOption<T extends string> = {
	value: T;
	label: string;
	description?: string;
};

export function InspectorSidebarSelectList<T extends string>({
	options,
	value,
	onValueChange,
}: {
	options: InspectorSidebarListOption<T>[];
	value: T;
	onValueChange: (value: T) => void;
}) {
	return (
		<CapsuleStripeList>
			{options.map((option) => {
				const selected = value === option.value;
				return (
					<CapsuleStripeListItem key={option.value} className="p-0">
						<button
							type="button"
							onClick={() => onValueChange(option.value)}
							className={cn(
								"flex w-full min-w-0 flex-col items-start px-2 py-2 text-left transition-opacity",
								selected
									? "text-foreground"
									: "text-muted-foreground opacity-75 hover:opacity-100",
							)}
						>
							<span
								className={cn(
									"truncate text-sm font-medium",
									selected && "text-primary",
								)}
							>
								{option.label}
							</span>
							{option.description ? (
								<span className="mt-0.5 line-clamp-2 text-xs text-muted-foreground">
									{option.description}
								</span>
							) : null}
						</button>
					</CapsuleStripeListItem>
				);
			})}
		</CapsuleStripeList>
	);
}

export function InspectorSidebarToggleList<T extends string>({
	options,
	selected,
	onChange,
}: {
	options: InspectorSidebarListOption<T>[];
	selected: T[];
	onChange: (value: T[]) => void;
}) {
	const toggle = (value: T) => {
		if (selected.includes(value)) {
			onChange(selected.filter((entry) => entry !== value));
			return;
		}
		onChange([...selected, value]);
	};

	return (
		<CapsuleStripeList>
			{options.map((option) => {
				const active = selected.includes(option.value);
				return (
					<CapsuleStripeListItem key={option.value} className="p-0">
						<button
							type="button"
							onClick={() => toggle(option.value)}
							className={cn(
								"flex w-full min-w-0 flex-col items-start px-2 py-2 text-left transition-opacity",
								active
									? "text-foreground"
									: "text-muted-foreground opacity-75 hover:opacity-100",
							)}
						>
							<span
								className={cn(
									"truncate text-sm font-medium",
									active && "text-primary",
								)}
							>
								{option.label}
							</span>
							{option.description ? (
								<span className="mt-0.5 line-clamp-2 text-xs text-muted-foreground">
									{option.description}
								</span>
							) : null}
						</button>
					</CapsuleStripeListItem>
				);
			})}
		</CapsuleStripeList>
	);
}

export function InspectorSidebarFieldInput({
	id,
	value,
	onChange,
	placeholder,
}: {
	id: string;
	value: string;
	onChange: (value: string) => void;
	placeholder?: string;
}) {
	return (
		<Input
			id={id}
			value={value}
			onChange={(event) => onChange(event.target.value)}
			placeholder={placeholder}
			className="h-8 border-input bg-background text-xs shadow-none focus-visible:border-muted-foreground/40 focus-visible:ring-0"
		/>
	);
}

export function InspectorSidebarProviderOptionLabel({
	name,
	isDefault = false,
}: {
	name: string;
	isDefault?: boolean;
}) {
	return (
		<span className="inline-flex min-w-0 items-center gap-1">
			<span className="truncate">{name}</span>
			{isDefault ? (
				<Star className="h-3 w-3 shrink-0 fill-current text-muted-foreground" aria-hidden />
			) : null}
		</span>
	);
}

export function InspectorSidebarSelect({
	id,
	value,
	onValueChange,
	options,
	placeholder,
	disabled = false,
}: {
	id: string;
	value: string;
	onValueChange: (value: string) => void;
	options: Array<{ value: string; label: string; isDefault?: boolean }>;
	placeholder?: string;
	disabled?: boolean;
}) {
	const selectedOption = options.find((option) => option.value === value);

	return (
		<Select value={value} onValueChange={onValueChange} disabled={disabled}>
			<SelectTrigger
				id={id}
				className="h-8 border-input bg-background text-xs shadow-none focus:ring-0 focus:ring-offset-0"
			>
				<SelectValue placeholder={placeholder}>
					{selectedOption ? (
						<InspectorSidebarProviderOptionLabel
							name={selectedOption.label}
							isDefault={selectedOption.isDefault}
						/>
					) : null}
				</SelectValue>
			</SelectTrigger>
			<SelectContent>
				{options.map((option) => (
					<SelectItem key={option.value} value={option.value} className="text-xs">
						<InspectorSidebarProviderOptionLabel
							name={option.label}
							isDefault={option.isDefault}
						/>
					</SelectItem>
				))}
			</SelectContent>
		</Select>
	);
}
