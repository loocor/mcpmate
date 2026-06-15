import { Plus, Target } from "lucide-react";
import React from "react";
import { Link } from "react-router-dom";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "../../components/ui/tooltip";
import { cn } from "../../lib/utils";

export const OPERATOR_CHIP_WIDTH_CLASS = "w-[52px]";
export const OPERATOR_CLEAR_VISIBLE_MAX = 3;
export const OPERATOR_STACK_PREVIEW_MAX = 3;

export const operatorNoDragRegionStyle = {
	WebkitAppRegion: "no-drag",
	appRegion: "no-drag",
} as React.CSSProperties;

export function toOperatorChipTitleCase(value: string): string {
	const trimmed = value.trim();
	if (!trimmed) {
		return trimmed;
	}

	return (
		trimmed
			.split(/[\s_-]+/)
			.filter(Boolean)
			.map((part) => part.charAt(0).toUpperCase() + part.slice(1).toLowerCase())
			.join(" ") || trimmed
	);
}

export type OperatorChipVisual = "active" | "attention" | "neutral" | "error";

export function operatorChipCircleClass(visual: OperatorChipVisual): string {
	switch (visual) {
		case "active":
			return "bg-emerald-600 text-white";
		case "attention":
			return "border border-amber-300 bg-amber-50 text-amber-800 dark:border-amber-700 dark:bg-amber-950/40 dark:text-amber-200";
		case "error":
			return "border border-red-300 bg-red-50 text-red-700 dark:border-red-800 dark:bg-red-950/40 dark:text-red-200";
		default:
			return "border border-slate-200 bg-slate-100 text-slate-700 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-200";
	}
}

export function operatorChipOuterRingClass(visual: OperatorChipVisual): string {
	if (visual === "active") {
		return "shadow-sm shadow-emerald-600/25 ring-2 ring-emerald-600/20";
	}
	return "";
}

export function operatorChipStackRingClass(): string {
	return "ring-2 ring-white dark:ring-slate-950";
}

const OPERATOR_CHIP_SIZE_CLASS = {
	md: "h-10 w-10",
	sm: "h-8 w-8",
} as const;

const OPERATOR_CHIP_TEXT_CLASS = {
	md: "text-sm",
	sm: "text-[11px]",
} as const;

export function OperatorChipAvatar({
	avatar,
	innerClassName,
	showStatusDot = true,
	size = "md",
	stackRing = false,
	visual,
}: {
	avatar: React.ReactNode;
	innerClassName?: string;
	showStatusDot?: boolean;
	size?: keyof typeof OPERATOR_CHIP_SIZE_CLASS;
	stackRing?: boolean;
	visual: OperatorChipVisual;
}) {
	const statusDotClass = showStatusDot ? operatorChipStatusDotClass(visual) : null;

	return (
		<span
			className={cn(
				"relative flex items-center justify-center",
				OPERATOR_CHIP_SIZE_CLASS[size],
			)}
		>
			<span
				className={cn(
					"flex h-full w-full items-center justify-center rounded-full font-semibold transition-all duration-200",
					OPERATOR_CHIP_TEXT_CLASS[size],
					operatorChipOuterRingClass(visual),
					stackRing && operatorChipStackRingClass(),
				)}
			>
				<span
					className={cn(
						"flex h-full w-full items-center justify-center overflow-hidden rounded-full",
						operatorChipCircleClass(visual),
						innerClassName,
					)}
				>
					{avatar}
				</span>
			</span>
			{statusDotClass ? (
				<span
					className={cn(
						"absolute bottom-0.5 right-0.5 z-[1] h-2 w-2 rounded-full ring-[1.5px]",
						statusDotClass,
					)}
					aria-hidden
				/>
			) : null}
		</span>
	);
}

export function operatorChipStatusDotClass(visual: OperatorChipVisual): string | null {
	switch (visual) {
		case "active":
			return "bg-white ring-emerald-600 dark:ring-emerald-500";
		case "attention":
			return "bg-amber-500 ring-white dark:ring-slate-950";
		case "error":
			return "bg-red-500 ring-white dark:ring-slate-950";
		default:
			return null;
	}
}

export function partitionAttentionFirst<T>(
	items: T[],
	needsAttention: (item: T) => boolean,
): { attention: T[]; clear: T[] } {
	const attention: T[] = [];
	const clear: T[] = [];
	for (const item of items) {
		if (needsAttention(item)) {
			attention.push(item);
		} else {
			clear.push(item);
		}
	}
	return { attention, clear };
}

export function splitClearForDisplay<T>(
	clearItems: T[],
	expanded: boolean,
	visibleMax = OPERATOR_CLEAR_VISIBLE_MAX,
): { visible: T[]; stacked: T[] } {
	if (expanded || clearItems.length <= visibleMax) {
		return { visible: clearItems, stacked: [] };
	}
	return {
		visible: clearItems.slice(0, visibleMax),
		stacked: clearItems.slice(visibleMax),
	};
}

export function OperatorRowDetailFrame({
	children,
	detailId,
}: {
	children: React.ReactNode;
	detailId: string;
}) {
	return (
		<div
			id={detailId}
			className="border-t border-slate-100 px-3 py-2.5 dark:border-slate-800"
			data-testid="operator-inline-detail"
		>
			{children}
		</div>
	);
}

export function OperatorHorizontalStrip({ children }: { children: React.ReactNode }) {
	return (
		<div className="-mx-1 flex items-start gap-2 overflow-x-auto px-1 py-0.5 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
			{children}
		</div>
	);
}

export function OperatorImportDropChip({
	chipLabel,
	dragActive,
	dropLabel,
	dropTip,
	onDragEnter,
	onDragLeave,
	onDragOver,
	onDrop,
}: {
	chipLabel: string;
	dragActive: boolean;
	dropLabel: string;
	dropTip: string;
	onDragEnter: (event: React.DragEvent<HTMLButtonElement>) => void;
	onDragLeave: (event: React.DragEvent<HTMLButtonElement>) => void;
	onDragOver: (event: React.DragEvent<HTMLButtonElement>) => void;
	onDrop: (event: React.DragEvent<HTMLButtonElement>) => void;
}) {
	const layoutClass = cn(OPERATOR_CHIP_WIDTH_CLASS, "flex shrink-0 flex-col items-center");
	const circleClass = cn(
		"flex h-10 w-10 items-center justify-center rounded-full border border-dashed transition-colors",
		dragActive
			? "border-emerald-500 bg-emerald-50 text-emerald-700 dark:border-emerald-500 dark:bg-emerald-950/40 dark:text-emerald-300"
			: "border-slate-300 bg-white text-slate-500 hover:border-slate-400 hover:bg-slate-50 hover:text-slate-700 dark:border-slate-600 dark:bg-slate-950 dark:text-slate-400 dark:hover:border-slate-500 dark:hover:bg-slate-900 dark:hover:text-slate-200",
	);
	const accessibleLabel = dragActive ? dropLabel : `${chipLabel}. ${dropTip}`;

	const chipButton = (
		<button
			data-desktop-drop-target="server-import"
			type="button"
			className={layoutClass}
			style={operatorNoDragRegionStyle}
			aria-label={accessibleLabel}
			onDragEnter={onDragEnter}
			onDragLeave={onDragLeave}
			onDragOver={onDragOver}
			onDrop={onDrop}
		>
			<span className={circleClass}>
				<Target className="h-4 w-4" aria-hidden />
			</span>
			<span className="mt-1 block w-full truncate text-center text-[10px] leading-3 text-slate-500 dark:text-slate-400">
				{chipLabel}
			</span>
		</button>
	);

	return (
		<Tooltip>
			<TooltipTrigger asChild>{chipButton}</TooltipTrigger>
			<TooltipContent side="top" className="max-w-[240px] text-xs">
				{dropTip}
			</TooltipContent>
		</Tooltip>
	);
}

export function OperatorMoreButton({
	href,
	isTauriShell,
	moreLabel,
	onOpenBoard,
	openLabel,
}: {
	href: string;
	isTauriShell: boolean;
	moreLabel: string;
	onOpenBoard: () => void;
	openLabel: string;
}) {
	const layoutClass = cn(OPERATOR_CHIP_WIDTH_CLASS, "flex shrink-0 flex-col items-center");
	const circleClass =
		"flex h-10 w-10 items-center justify-center rounded-full border border-dashed border-slate-300 bg-white text-slate-500 transition-colors hover:border-slate-400 hover:bg-slate-50 hover:text-slate-700 dark:border-slate-600 dark:bg-slate-950 dark:text-slate-400 dark:hover:border-slate-500 dark:hover:bg-slate-900 dark:hover:text-slate-200";

	const content = (
		<>
			<span className={circleClass}>
				<Plus className="h-4 w-4" aria-hidden />
			</span>
			<span className="mt-1 block w-full truncate text-center text-[10px] leading-3 text-slate-500 dark:text-slate-400">
				{moreLabel}
			</span>
		</>
	);

	if (isTauriShell) {
		return (
			<button
				type="button"
				className={layoutClass}
				style={operatorNoDragRegionStyle}
				aria-label={openLabel}
				title={openLabel}
				onClick={(event) => {
					event.stopPropagation();
					onOpenBoard();
				}}
			>
				{content}
			</button>
		);
	}

	return (
		<Link
			to={href}
			className={layoutClass}
			style={operatorNoDragRegionStyle}
			aria-label={openLabel}
			title={openLabel}
			onClick={(event) => event.stopPropagation()}
		>
			{content}
		</Link>
	);
}

function operatorChipLayoutClass(extra?: string): string {
	return cn(OPERATOR_CHIP_WIDTH_CLASS, "flex shrink-0 flex-col items-center text-left", extra);
}

function OperatorChipBody({
	avatar,
	children,
	displayName,
	visual,
}: {
	avatar: React.ReactNode;
	children?: React.ReactNode;
	displayName: string;
	visual: OperatorChipVisual;
}) {
	return (
		<>
			<OperatorChipAvatar avatar={avatar} visual={visual} />
			<span
				className="mt-1 block w-full truncate text-center text-[10px] leading-3 text-slate-600 dark:text-slate-300"
				title={displayName}
			>
				{displayName}
			</span>
			{children}
		</>
	);
}

export function OperatorStaticChip({
	avatar,
	displayName,
	visual,
}: {
	avatar: React.ReactNode;
	displayName: string;
	visual: OperatorChipVisual;
}) {
	return (
		<div className={operatorChipLayoutClass("cursor-default")}>
			<OperatorChipBody avatar={avatar} displayName={displayName} visual={visual} />
		</div>
	);
}

export function OperatorActionChip({
	ariaLabel,
	avatar,
	disabled,
	displayName,
	onAction,
	visual,
}: {
	ariaLabel: string;
	avatar: React.ReactNode;
	disabled?: boolean;
	displayName: string;
	onAction: () => void;
	visual: OperatorChipVisual;
}) {
	return (
		<button
			type="button"
			className={operatorChipLayoutClass(
				"disabled:cursor-not-allowed disabled:opacity-60",
			)}
			style={operatorNoDragRegionStyle}
			disabled={disabled}
			aria-label={ariaLabel}
			title={ariaLabel}
			onClick={(event) => {
				event.stopPropagation();
				onAction();
			}}
		>
			<OperatorChipBody avatar={avatar} displayName={displayName} visual={visual} />
		</button>
	);
}

export function OperatorBoardLinkChip({
	ariaLabel,
	avatar,
	children,
	displayName,
	href,
	isTauriShell,
	onOpenBoard,
	visual,
}: {
	ariaLabel: string;
	avatar: React.ReactNode;
	children?: React.ReactNode;
	displayName: string;
	href: string;
	isTauriShell: boolean;
	onOpenBoard: () => void;
	visual: OperatorChipVisual;
}) {
	const body = (
		<OperatorChipBody avatar={avatar} displayName={displayName} visual={visual}>
			{children}
		</OperatorChipBody>
	);

	if (isTauriShell) {
		return (
			<button
				type="button"
				className={operatorChipLayoutClass()}
				style={operatorNoDragRegionStyle}
				aria-label={ariaLabel}
				title={ariaLabel}
				onClick={(event) => {
					event.stopPropagation();
					onOpenBoard();
				}}
			>
				{body}
			</button>
		);
	}

	return (
		<Link
			to={href}
			className={operatorChipLayoutClass()}
			style={operatorNoDragRegionStyle}
			aria-label={ariaLabel}
			title={ariaLabel}
			onClick={(event) => event.stopPropagation()}
		>
			{body}
		</Link>
	);
}

export function OperatorAvatarStackToggle({
	collapseLabel,
	expandLabel,
	expanded,
	items,
	onToggle,
}: {
	collapseLabel: string;
	expandLabel: string;
	expanded: boolean;
	items: Array<{
		id: string;
		avatar: React.ReactNode;
		visual: OperatorChipVisual;
	}>;
	onToggle: () => void;
}) {
	const preview = items.slice(0, OPERATOR_STACK_PREVIEW_MAX);
	const overflow = items.length - preview.length;

	return (
		<button
			type="button"
			className={cn(OPERATOR_CHIP_WIDTH_CLASS, "flex shrink-0 flex-col items-center")}
			style={operatorNoDragRegionStyle}
			aria-expanded={expanded}
			aria-label={expanded ? collapseLabel : expandLabel}
			title={expanded ? collapseLabel : expandLabel}
			onClick={(event) => {
				event.stopPropagation();
				onToggle();
			}}
		>
			<span className="flex h-10 items-center justify-center py-0.5 pl-1.5">
				{preview.map((item, index) => (
					<span
						key={item.id}
						className={cn("relative shrink-0", index > 0 && "-ml-2.5")}
					>
						<OperatorChipAvatar
							avatar={item.avatar}
							showStatusDot={false}
							size="sm"
							stackRing
							visual={item.visual}
						/>
					</span>
				))}
				{overflow > 0 ? (
					<span
						className={cn(
							"-ml-2.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-slate-200 text-[10px] font-semibold text-slate-700 dark:bg-slate-800 dark:text-slate-200",
							operatorChipStackRingClass(),
						)}
					>
						+{overflow}
					</span>
				) : null}
			</span>
			<span className="mt-1 block w-full truncate text-center text-[10px] leading-3 text-slate-500 dark:text-slate-400">
				{expanded ? collapseLabel : expandLabel}
			</span>
		</button>
	);
}

export function OperatorRowDetailMessage({
	tone = "muted",
	children,
}: {
	tone?: "error" | "muted";
	children: React.ReactNode;
}) {
	return (
		<p
			className={cn(
				"text-xs",
				tone === "error"
					? "text-red-600 dark:text-red-400"
					: "text-slate-500 dark:text-slate-400",
			)}
		>
			{children}
		</p>
	);
}
