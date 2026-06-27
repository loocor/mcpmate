import type { ReactNode } from "react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { capabilityRecordMatchesSearch } from "../lib/capability-search";
import { cn } from "../lib/utils";
import type { CapabilityRecord } from "../types/capabilities";
import CapabilityList, { type CapabilityKind } from "./capability-list";
import { CapabilityListSkeleton } from "./capability-list-skeleton";
import { CardListScrollBody } from "./card-list-scroll-body";
import { Input } from "./ui/input";
import { toolbarSearchInputClassName } from "./ui/page-toolbar";

export type CapabilityPreviewKind = CapabilityKind;

export type CapabilityPreviewFlatItem = {
	kind: CapabilityPreviewKind;
	item: CapabilityRecord;
};

type CapabilityPreviewListProps = {
	tools?: CapabilityRecord[];
	resources?: CapabilityRecord[];
	prompts?: CapabilityRecord[];
	templates?: CapabilityRecord[];
	className?: string;
	contentClassName?: string;
	heading?: string;
	showHeader?: boolean;
	framed?: boolean;
	hasSource?: boolean;
	isLoading?: boolean;
	error?: string | null;
	searchValue?: string;
	onSearchChange?: (value: string) => void;
	searchPlaceholder?: string;
	headerActions?: ReactNode;
	toolbar?: ReactNode;
	selectHintText?: string;
	emptyText?: string;
	emptySearchText?: string;
	showSectionLabels?: boolean;
	showSectionCounts?: boolean;
	renderFlatList?: (items: CapabilityPreviewFlatItem[]) => ReactNode;
	renderList?: (
		kind: CapabilityPreviewKind,
		items: CapabilityRecord[],
	) => ReactNode;
};

type CapabilitySection = {
	kind: CapabilityPreviewKind;
	items: CapabilityRecord[];
};

type CapabilityPreviewFrameProps = {
	children: ReactNode;
	className?: string;
	framed: boolean;
};

function CapabilityPreviewFrame({
	children,
	className,
	framed,
}: CapabilityPreviewFrameProps): ReactNode {
	if (framed) {
		return (
			<CardListScrollBody className={className}>{children}</CardListScrollBody>
		);
	}
	return (
		<div className={cn("flex min-h-0 flex-1 flex-col overflow-hidden", className)}>
			{children}
		</div>
	);
}

const CAPABILITY_SECTION_ORDER: CapabilityPreviewKind[] = [
	"tools",
	"resources",
	"templates",
	"prompts",
];

function sectionLabel(kind: CapabilityPreviewKind, t: ReturnType<typeof useTranslation>["t"]) {
	if (kind === "templates") {
		return t("detail.capabilityList.labels.templates", {
			defaultValue: "Resource Templates",
		});
	}
	return t(`detail.capabilityList.labels.${kind}`, {
		defaultValue: kind.charAt(0).toUpperCase() + kind.slice(1),
	});
}

function countLabel(
	kind: CapabilityPreviewKind,
	count: number,
	t: ReturnType<typeof useTranslation>["t"],
): string {
	const isSingular = count === 1;

	if (kind === "tools") {
		const key = isSingular
			? "wizard.preview.capabilities.tool"
			: "wizard.preview.capabilities.tools";
		const defaultValue = isSingular ? "tool" : "tools";
		return t(key, { defaultValue });
	}

	if (kind === "resources") {
		const key = isSingular
			? "wizard.preview.capabilities.resource"
			: "wizard.preview.capabilities.resources";
		const defaultValue = isSingular ? "resource" : "resources";
		return t(key, { defaultValue });
	}

	if (kind === "templates") {
		const key = isSingular
			? "wizard.preview.capabilities.template"
			: "wizard.preview.capabilities.templates";
		const defaultValue = isSingular ? "template" : "templates";
		return t(key, { defaultValue });
	}

	const key = isSingular
		? "wizard.preview.capabilities.prompt"
		: "wizard.preview.capabilities.prompts";
	const defaultValue = isSingular ? "prompt" : "prompts";
	return t(key, { defaultValue });
}

export function CapabilityPreviewList({
	tools = [],
	resources = [],
	prompts = [],
	templates = [],
	className,
	contentClassName,
	heading,
	showHeader = true,
	framed = true,
	hasSource = true,
	isLoading = false,
	error,
	searchValue = "",
	onSearchChange,
	searchPlaceholder,
	headerActions,
	toolbar,
	selectHintText,
	emptyText,
	emptySearchText,
	showSectionLabels = true,
	showSectionCounts = false,
	renderFlatList,
	renderList,
}: CapabilityPreviewListProps) {
	const { t } = useTranslation("servers");
	const searchQuery = searchValue.trim();

	const sourceSections = useMemo<CapabilitySection[]>(
		() => [
			{ kind: "tools", items: tools },
			{ kind: "resources", items: resources },
			{ kind: "templates", items: templates },
			{ kind: "prompts", items: prompts },
		],
		[prompts, resources, templates, tools],
	);

	const visibleSections = useMemo<CapabilitySection[]>(
		() =>
			sourceSections
				.map((section) => ({
					...section,
					items: searchQuery
						? section.items.filter((item) =>
								capabilityRecordMatchesSearch(item, searchQuery),
							)
						: section.items,
				}))
				.filter((section) => section.items.length > 0)
				.sort(
					(a, b) =>
						CAPABILITY_SECTION_ORDER.indexOf(a.kind) -
						CAPABILITY_SECTION_ORDER.indexOf(b.kind),
				),
		[sourceSections, searchQuery],
	);

	const totalSourceCount = sourceSections.reduce(
		(total, section) => total + section.items.length,
		0,
	);
	const flatVisibleItems = useMemo(
		() =>
			visibleSections.flatMap((section) =>
				section.items.map((item) => ({
					kind: section.kind,
					item,
				})),
			),
		[visibleSections],
	);
	const summaryParts = visibleSections.map((section) => {
		const count = section.items.length;
		return `${count} ${countLabel(section.kind, count, t)}`;
	});
	const defaultHeading = t("wizard.preview.capabilitiesTitle", {
		defaultValue: "Capabilities",
	});
	const computedHeading =
		!isLoading && summaryParts.length
			? t("wizard.preview.capabilitiesSummary", {
					summary: summaryParts.join(" · "),
					defaultValue: "Capabilities · {{summary}}",
				})
			: defaultHeading;
	const headingText = heading ?? computedHeading;

	const defaultRenderList = (
		kind: CapabilityPreviewKind,
		items: CapabilityRecord[],
	): ReactNode => (
		<CapabilityList
			asCard={false}
			kind={kind}
			context="server"
			items={items}
			clickToToggleDetails
		/>
		);
		const bodyClassName = cn("p-3", contentClassName);
		const emptyStateClassName =
			"flex min-h-full items-center justify-center rounded-lg border border-slate-200 bg-white px-4 py-8 text-center text-sm text-slate-500 dark:border-slate-800 dark:bg-slate-950/40 dark:text-slate-400";
		const loadingContent = renderFlatList ? (
			<div className={bodyClassName}>
				<CardListScrollBody scrollLocked>
					<CapabilityListSkeleton className="p-0" fillContainer />
			</CardListScrollBody>
		</div>
	) : (
		<CapabilityListSkeleton showSectionLabel />
	);

	return (
		<CapabilityPreviewFrame className={className} framed={framed}>
			{showHeader ? (
				<div className="sticky top-0 z-10 flex items-center justify-between gap-2 border-b border-slate-200/80 bg-background/95 p-3 backdrop-blur-sm dark:border-slate-700/80">
					<div className="min-w-0 truncate font-medium text-sm" title={headingText}>
						{headingText}
					</div>
					{toolbar ? (
						<div className="min-w-0 flex-1">{toolbar}</div>
					) : onSearchChange || headerActions ? (
						<div className="flex shrink-0 items-center gap-2">
							{onSearchChange ? (
								<Input
									type="search"
									value={searchValue}
									onChange={(event) => onSearchChange(event.target.value)}
									placeholder={
										searchPlaceholder ??
										t("wizard.preview.filterCapabilities", {
											defaultValue: "Filter capabilities...",
										})
									}
									className={cn(toolbarSearchInputClassName, "h-8 w-48")}
								/>
							) : null}
							{headerActions}
						</div>
					) : null}
				</div>
			) : null}

			{isLoading ? (
				loadingContent
			) : error ? (
				<div className="overflow-hidden break-words px-4 py-3 text-xs text-red-500">
					{error}
				</div>
				) : !hasSource ? (
					<div className={emptyStateClassName}>
						{selectHintText ??
							t("wizard.preview.selectServerHint", {
								defaultValue:
									"Select this server from the list to generate its capability preview.",
						})}
					</div>
				) : totalSourceCount === 0 ? (
					<div className={emptyStateClassName}>
						{emptyText ??
							t("wizard.preview.emptyCapabilities", {
								defaultValue: "No capabilities discovered for this server.",
							})}
					</div>
				) : searchQuery && visibleSections.length === 0 ? (
					<div className={emptyStateClassName}>
						{emptySearchText ??
							t("wizard.preview.emptyCapabilitySearch", {
								defaultValue: "No capabilities match this search.",
						})}
				</div>
			) : (
				<div className={bodyClassName}>
					{renderFlatList ? (
						renderFlatList(flatVisibleItems)
					) : (
						<div className={showSectionLabels ? "space-y-5" : "space-y-3"}>
							{visibleSections.map((section) => (
								<div
									key={section.kind}
									className={showSectionLabels ? "space-y-3" : undefined}
								>
									{showSectionLabels ? (
										<div className="text-xs font-semibold uppercase tracking-wide text-slate-700 dark:text-slate-200">
											{sectionLabel(section.kind, t)}
											{showSectionCounts ? (
												<span className="ml-1 text-[11px] font-normal text-slate-400">
													({section.items.length})
												</span>
											) : null}
										</div>
									) : null}
									{renderList
										? renderList(section.kind, section.items)
										: defaultRenderList(section.kind, section.items)}
								</div>
							))}
						</div>
					)}
				</div>
			)}
		</CapabilityPreviewFrame>
	);
}
