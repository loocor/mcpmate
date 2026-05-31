import type { ReactNode } from "react";
import { Download, Loader2, RefreshCw, Server } from "lucide-react";
import { Alert, AlertDescription } from "../../components/ui/alert";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Card, CardContent } from "../../components/ui/card";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "../../components/ui/tooltip";
import type { CatalogTagFilter } from "../../lib/admin-discovery";
import { cn } from "../../lib/utils";
import { formatTransportTag } from "../clients/transport-labels";

const LIST_PANEL_HEIGHT_CLASS = "h-[42vh]";
export const ONBOARDING_SCROLLABLE_LIST_CLASS = cn(
	LIST_PANEL_HEIGHT_CLASS,
	"overflow-y-auto pr-1",
);
export const ONBOARDING_TWO_COLUMN_LIST_CLASS = "grid gap-3 sm:grid-cols-2";

export const POPULAR_CLIENT_TAG_FILTERS = [
	"all",
	"editor",
	"agent",
	"application",
	"cli",
	"desktop",
	"browser",
] as const;

export const POPULAR_SERVER_TAG_FILTERS = [
	"all",
	"memory",
	"developer-tools",
	"browser",
	"documentation",
	"database",
	"design",
	"automation",
	"filesystem",
	"debugging",
	"knowledge",
	"3d",
	"content",
	"creative-tools",
	"frontend",
	"web",
] as const;

export type { CatalogTagFilter };

const TAG_PILL_CLASS =
	"inline-flex h-7 shrink-0 items-center rounded-full border px-3 text-xs font-medium transition-colors";

const TAG_PILL_ACTIVE_CLASS =
	"border-slate-900 bg-slate-900 text-white dark:border-slate-100 dark:bg-slate-100 dark:text-slate-900";

const TAG_PILL_INACTIVE_CLASS =
	"border-slate-200 bg-white text-slate-600 hover:border-slate-300 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-300 dark:hover:border-slate-600";

const REFRESH_PILL_CLASS = cn(
	TAG_PILL_CLASS,
	TAG_PILL_INACTIVE_CLASS,
	"h-7 w-7 justify-center px-0 disabled:opacity-50",
);

const CARD_DESCRIPTION_CLASS =
	"line-clamp-2 min-h-10 text-xs leading-5 text-slate-500";

const ONBOARDING_CARD_SELECTED_CLASS =
	"border-emerald-500 bg-emerald-50 dark:border-emerald-400 dark:bg-emerald-950/30";

const ONBOARDING_CARD_DEFAULT_CLASS =
	"border-slate-200 bg-white hover:border-slate-300 dark:border-slate-700 dark:bg-slate-900 dark:hover:border-slate-600";

export function catalogTagLabel(
	translate: (key: string, options?: { defaultValue?: string }) => string,
	namespace: "clients" | "servers",
	tag: CatalogTagFilter,
): string {
	return translate(`${namespace}.tags.${tag}`, {
		defaultValue: formatTransportTag(tag),
	});
}

function CatalogTagPill({
	active,
	label,
	onClick,
}: {
	active: boolean;
	label: string;
	onClick: () => void;
}) {
	return (
		<button
			type="button"
			onClick={onClick}
			className={cn(TAG_PILL_CLASS, active ? TAG_PILL_ACTIVE_CLASS : TAG_PILL_INACTIVE_CLASS)}
		>
			{label}
		</button>
	);
}

export function OnboardingStepHeader({
	icon,
	title,
	description,
}: {
	icon: ReactNode;
	title: string;
	description: string;
}) {
	return (
		<div className="mb-6 text-center">
			{icon}
			<h2 className="text-2xl font-bold tracking-tight">{title}</h2>
			<p className="mt-2 text-slate-600 dark:text-slate-400">{description}</p>
		</div>
	);
}

export function OnboardingCatalogSpinner({
	accent = "emerald",
}: {
	accent?: "emerald" | "violet";
}) {
	return (
		<div className="flex justify-center py-12">
			<Loader2
				className={cn(
					"h-8 w-8 animate-spin",
					accent === "violet" ? "text-violet-500" : "text-emerald-500",
				)}
			/>
		</div>
	);
}

export function OnboardingScrollableGrid({ children }: { children: ReactNode }) {
	return (
		<div className={ONBOARDING_SCROLLABLE_LIST_CLASS}>
			<div className={ONBOARDING_TWO_COLUMN_LIST_CLASS}>{children}</div>
		</div>
	);
}

export function OnboardingTabFootnote({ children }: { children: ReactNode }) {
	return (
		<p className="text-center text-xs text-slate-400 dark:text-slate-500">{children}</p>
	);
}

export function OnboardingCatalogTabPanel({
	footnote,
	children,
}: {
	footnote: ReactNode;
	children: ReactNode;
}) {
	return (
		<div className="space-y-3">
			{children}
			<OnboardingTabFootnote>{footnote}</OnboardingTabFootnote>
		</div>
	);
}

export function AdminDiscoveryPartialWarning({ message }: { message: string }) {
	return (
		<Alert className="border-amber-200 bg-amber-50 text-amber-900 dark:border-amber-900/60 dark:bg-amber-950/30 dark:text-amber-200">
			<AlertDescription>{message}</AlertDescription>
		</Alert>
	);
}

export function OnboardingCatalogToolbar({
	tagValue,
	onTagChange,
	tagOptions,
	tagLabel,
	refreshAriaLabel,
	onRefresh,
	refreshDisabled,
}: {
	tagValue: CatalogTagFilter;
	onTagChange: (value: CatalogTagFilter) => void;
	tagOptions: readonly CatalogTagFilter[];
	tagLabel: (tag: CatalogTagFilter) => string;
	refreshAriaLabel: string;
	onRefresh: () => void;
	refreshDisabled?: boolean;
}) {
	const hasAll = tagOptions.includes("all");
	const scrollTags = tagOptions.filter((tag) => tag !== "all");

	return (
		<div className="mb-4 flex h-7 items-center gap-2">
			{hasAll ? (
				<CatalogTagPill
					active={tagValue === "all"}
					label={tagLabel("all")}
					onClick={() => onTagChange("all")}
				/>
			) : null}
			{scrollTags.length > 0 ? (
				<div className="min-w-0 flex-1 overflow-x-auto [-ms-overflow-style:none] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
					<div className="flex w-max gap-2 pr-1">
						{scrollTags.map((tag) => (
							<CatalogTagPill
								key={tag}
								active={tagValue === tag}
								label={tagLabel(tag)}
								onClick={() => onTagChange(tag)}
							/>
						))}
					</div>
				</div>
			) : (
				<div className="flex-1" />
			)}
			<button
				type="button"
				onClick={onRefresh}
				disabled={refreshDisabled}
				aria-label={refreshAriaLabel}
				title={refreshAriaLabel}
				className={REFRESH_PILL_CLASS}
			>
				<RefreshCw className={cn("h-3.5 w-3.5", refreshDisabled && "animate-spin")} />
			</button>
		</div>
	);
}

function ClientLogo({
	name,
	logoUrl,
	showLogo,
	onError,
}: {
	name: string;
	logoUrl?: string;
	showLogo: boolean;
	onError: () => void;
}) {
	return (
		<div className="flex h-10 w-10 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-slate-100 text-sm font-semibold dark:bg-slate-800">
			{showLogo && logoUrl ? (
				<img
					src={logoUrl}
					alt={name}
					className="h-full w-full object-cover"
					loading="lazy"
					onError={onError}
				/>
			) : (
				name.charAt(0).toUpperCase()
			)}
		</div>
	);
}

export function OnboardingClientCard({
	name,
	description,
	logoUrl,
	showLogo,
	isSelected,
	onToggle,
	onLogoError,
	badgeLabel,
	badgeVariant = "success",
	homepageUrl,
	isDetected = false,
	onInstall,
	installAriaLabel,
	installTooltip,
	selectedAriaLabel,
	unselectedAriaLabel,
}: {
	name: string;
	description?: string;
	logoUrl?: string;
	showLogo: boolean;
	isSelected: boolean;
	onToggle: () => void;
	onLogoError: () => void;
	badgeLabel: string;
	badgeVariant?: "success" | "warning";
	homepageUrl?: string;
	isDetected?: boolean;
	onInstall?: () => void;
	installAriaLabel?: string;
	installTooltip?: string;
	selectedAriaLabel?: string;
	unselectedAriaLabel?: string;
}) {
	const canInstall = Boolean(homepageUrl?.trim()) && !isDetected && onInstall;
	const cardBody = (
		<>
			<ClientLogo name={name} logoUrl={logoUrl} showLogo={showLogo} onError={onLogoError} />
			<div className="min-w-0 flex-1">
				<div className="flex min-w-0 items-center gap-2">
					<span className="min-w-0 flex-1 truncate font-medium">{name}</span>
					<Badge
						variant={badgeVariant}
						className="shrink-0 px-2 py-0 text-[10px] font-medium"
					>
						{badgeLabel}
					</Badge>
				</div>
				<div className={cn("mt-0.5", CARD_DESCRIPTION_CLASS, canInstall && "pr-10")}>
					{description || "\u00a0"}
				</div>
			</div>
		</>
	);

	if (canInstall || selectedAriaLabel) {
		return (
			<div
				className={cn(
					"group relative rounded-lg border-2 transition-all",
					isSelected ? ONBOARDING_CARD_SELECTED_CLASS : ONBOARDING_CARD_DEFAULT_CLASS,
				)}
			>
				{canInstall ? (
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								type="button"
								size="icon"
								className="absolute bottom-3 right-3 z-10 h-7 w-7 rounded-md opacity-0 shadow-sm transition-opacity group-hover:opacity-100 group-focus-within:opacity-100"
								onClick={(event) => {
									event.stopPropagation();
									onInstall();
								}}
								aria-label={installAriaLabel}
							>
								<Download className="h-3.5 w-3.5" />
							</Button>
						</TooltipTrigger>
						<TooltipContent side="top">{installTooltip}</TooltipContent>
					</Tooltip>
				) : null}
				<button
					type="button"
					aria-pressed={isSelected}
					aria-label={isSelected ? selectedAriaLabel : unselectedAriaLabel}
					onClick={onToggle}
					className="flex w-full items-center gap-3 p-4 text-left"
				>
					{cardBody}
				</button>
			</div>
		);
	}

	return (
		<button
			type="button"
			onClick={onToggle}
			className={cn(
				"flex items-center gap-3 rounded-lg border-2 p-4 text-left transition-all",
				isSelected ? ONBOARDING_CARD_SELECTED_CLASS : ONBOARDING_CARD_DEFAULT_CLASS,
			)}
		>
			{cardBody}
		</button>
	);
}

function ServerLogo({
	name,
	logoUrl,
	showLogo,
	onError,
}: {
	name: string;
	logoUrl?: string;
	showLogo: boolean;
	onError: () => void;
}) {
	return (
		<div
			className={cn(
				"flex h-10 w-10 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-violet-100 dark:bg-violet-900/30",
				showLogo && logoUrl && "p-1",
			)}
		>
			{showLogo && logoUrl ? (
				<img
					src={logoUrl}
					alt={name}
					className="h-full w-full object-contain"
					loading="lazy"
					onError={onError}
				/>
			) : (
				<Server className="h-5 w-5 text-violet-600 dark:text-violet-400" />
			)}
		</div>
	);
}

export function OnboardingServerCard({
	name,
	kind,
	description,
	detail,
	logoUrl,
	showLogo,
	onLogoError,
	isSelected,
	onToggle,
	sourceLabel,
	selectedAriaLabel,
	unselectedAriaLabel,
}: {
	name: string;
	kind: string;
	description?: string;
	detail?: string;
	logoUrl?: string;
	showLogo: boolean;
	onLogoError: () => void;
	isSelected: boolean;
	onToggle: () => void;
	sourceLabel?: string;
	selectedAriaLabel: string;
	unselectedAriaLabel: string;
}) {
	return (
		<button
			type="button"
			aria-pressed={isSelected}
			aria-label={isSelected ? selectedAriaLabel : unselectedAriaLabel}
			onClick={onToggle}
			className={cn(
				"flex items-center gap-4 rounded-lg border-2 p-4 text-left transition-all",
				isSelected ? ONBOARDING_CARD_SELECTED_CLASS : ONBOARDING_CARD_DEFAULT_CLASS,
			)}
		>
			<ServerLogo name={name} logoUrl={logoUrl} showLogo={showLogo} onError={onLogoError} />
			<div className="min-w-0 flex-1">
				<div className="flex min-w-0 items-center gap-2">
					<span className="min-w-0 flex-1 truncate font-medium" title={name}>
						{name}
					</span>
					<span className="shrink-0 rounded-full bg-slate-100 px-2 py-0.5 text-xs text-slate-500 dark:bg-slate-800 dark:text-slate-400">
						{formatTransportTag(kind)}
					</span>
				</div>
				{description ? (
					<div className={cn("mt-0.5", CARD_DESCRIPTION_CLASS)}>{description}</div>
				) : detail ? (
					<div className="mt-0.5 truncate font-mono text-xs leading-5 text-slate-500 line-clamp-1">
						{detail}
					</div>
				) : null}
				{sourceLabel ? <div className="mt-0.5 text-xs leading-5 text-slate-500">{sourceLabel}</div> : null}
			</div>
		</button>
	);
}

export function OnboardingEmptyCard({
	message,
	actionLabel,
	onAction,
}: {
	message: string;
	actionLabel?: string;
	onAction?: () => void;
}) {
	return (
		<Card>
			<CardContent className="space-y-3 py-8 text-center text-slate-500">
				<p>{message}</p>
				{actionLabel && onAction ? (
					<Button variant="outline" size="sm" onClick={onAction}>
						{actionLabel}
					</Button>
				) : null}
			</CardContent>
		</Card>
	);
}
