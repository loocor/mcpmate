import { ArrowDown, Loader2, RefreshCw } from "lucide-react";
import CapabilityList, { type CapabilityKind } from "../../components/capability-list";
import { CardListScrollBody } from "../../components/card-list-scroll-body";
import {
	CapsuleStripeList,
	CapsuleStripeListItem,
} from "../../components/capsule-stripe-list";
import { InspectorExpandableSearch } from "../../components/inspector-expandable-search";
import {
	Accordion,
	AccordionContent,
	AccordionItem,
	AccordionTrigger,
} from "../../components/ui/accordion";
import { Badge } from "../../components/ui/badge";
import { cn } from "../../lib/utils";
import type { CapabilityRecord } from "../../types/capabilities";
import {
	INSPECTOR_CAPABILITY_FAMILIES,
	type InspectorCapabilityFamily,
	type InspectorCapabilityFamilyOption,
	type InspectorCapabilityFamilyState,
	type InspectorCapabilityListItem,
} from "./inspector-feature-config";
import { BrushCleaning } from "./inspector-icons";

type InspectorCapabilityAccordionSidebarProps = {
	familyStates: Record<InspectorCapabilityFamily, InspectorCapabilityFamilyState>;
	families?: InspectorCapabilityFamilyOption[];
	activeFamily: InspectorCapabilityFamily | null;
	onActiveFamilyChange: (family: InspectorCapabilityFamily | null) => void;
	onList: (family: InspectorCapabilityFamily) => void;
	onClear: (family: InspectorCapabilityFamily) => void;
	onSelectItem: (family: InspectorCapabilityFamily, key: string) => void;
	capabilitySearch: string;
	onCapabilitySearchChange: (value: string) => void;
	capabilitySearchOpen: boolean;
	onCapabilitySearchOpenChange: (open: boolean) => void;
	disabled?: boolean;
};

const DEFAULT_OPEN_FAMILY: InspectorCapabilityFamily = "tools";

function capabilityFamilyCountValue({
	listing,
	listed,
	listedCount,
	advertisedCount,
}: {
	listing: boolean;
	listed: boolean;
	listedCount: number;
	advertisedCount: number | null;
}): number | null {
	if (listing) return null;
	if (listed) return listedCount;
	if (advertisedCount != null) return advertisedCount;
	return null;
}

const capabilityHeaderIconClassName =
	"inline-flex h-7 w-7 shrink-0 items-center justify-center text-muted-foreground transition-opacity disabled:pointer-events-none disabled:opacity-40";

const capabilityHeaderIconRevealClassName =
	"opacity-0 group-hover:opacity-50 hover:opacity-100 focus-visible:opacity-100";

const iconActionLightClassName = "opacity-50 hover:opacity-100";

function capabilityHeaderIconInteractiveClassName(revealed: boolean) {
	return cn(
		capabilityHeaderIconClassName,
		revealed ? "opacity-100" : capabilityHeaderIconRevealClassName,
	);
}

type InspectorSidebarCapabilityRecord = CapabilityRecord & {
	__inspectorKey: string;
};

function familyToCapabilityKind(
	family: InspectorCapabilityFamily,
): CapabilityKind | null {
	switch (family) {
		case "tools":
			return "tools";
		case "prompts":
			return "prompts";
		case "resources":
			return "resources";
		case "resource_templates":
			return "templates";
		default:
			return null;
	}
}

function toSidebarCapabilityRecord(
	item: InspectorCapabilityListItem,
	family: InspectorCapabilityFamily,
): InspectorSidebarCapabilityRecord {
	const { key, title, description } = item;
	const base: InspectorSidebarCapabilityRecord = {
		__inspectorKey: key,
		id: key,
		name: title,
		description,
	};

	switch (family) {
		case "tools":
			return {
				...base,
				tool_name: title,
				unique_name: key,
			};
		case "prompts":
			return {
				...base,
				prompt_name: title,
				unique_name: key,
			};
		case "resources":
			return {
				...base,
				resource_uri: key,
				uri: key,
			};
		case "resource_templates":
			return {
				...base,
				uriTemplate: key,
				uri_template: key,
			};
		default:
			return {
				...base,
				unique_name: key,
			};
	}
}

type InspectorCapabilityFamilyListProps = {
	family: InspectorCapabilityFamilyOption;
	state: InspectorCapabilityFamilyState;
	disabled?: boolean;
	onSelectItem: (family: InspectorCapabilityFamily, key: string) => void;
};

function inspectorCapabilityListActionIcon(listed: boolean) {
	return listed ? RefreshCw : ArrowDown;
}

function InspectorCapabilityListActionGlyph({
	listed,
	listing,
	className,
}: {
	listed: boolean;
	listing: boolean;
	className?: string;
}) {
	if (listing) {
		return <Loader2 className={cn(className, "animate-spin")} aria-hidden />;
	}
	const Icon = inspectorCapabilityListActionIcon(listed);
	return <Icon className={className} aria-hidden />;
}

function InspectorCapabilityFamilyList({
	family,
	state,
	disabled = false,
	onSelectItem,
	onList,
	filterText,
	fillHeight = false,
}: InspectorCapabilityFamilyListProps & {
	fillHeight?: boolean;
	onList?: () => void;
	filterText?: string;
}) {
	const kind = familyToCapabilityKind(family.value);
	const listShellClassName = fillHeight ? "flex min-h-0 flex-1 flex-col" : undefined;

	if (!kind) {
		const message =
			state.items.length === 0
				? "List completed with no items."
				: "Capability list preview is not available for this family yet.";
		if (fillHeight) {
			return (
				<div className={listShellClassName}>
					<CardListScrollBody>
						<div className="flex min-h-full w-full flex-col items-center justify-center px-4 text-center">
							<p className="text-xs text-muted-foreground">{message}</p>
						</div>
					</CardListScrollBody>
				</div>
			);
		}
		return (
			<div className={listShellClassName}>
				<CapsuleStripeList>
					<CapsuleStripeListItem className="px-2 py-3">
						<p className="w-full text-xs text-muted-foreground">{message}</p>
					</CapsuleStripeListItem>
				</CapsuleStripeList>
			</div>
		);
	}

	const items = state.listed
		? state.items.map((item) => toSidebarCapabilityRecord(item, family.value))
		: [];
	const emptyText = state.listed
		? "List completed with no items."
		: `Run ${family.listMethod} on the selected server.`;
	const listActionLabel = state.listed ? `Refresh ${family.label}` : `List ${family.label}`;

	return (
		<div className={listShellClassName}>
			<CapabilityList<InspectorSidebarCapabilityRecord>
				asCard={false}
				kind={kind}
				context="profile"
				rowLayout="compact"
				items={items}
				loading={state.listing}
				filterText={filterText}
				clickToToggleDetails={false}
				selectable={state.listed}
				selectedIds={state.selectedKey ? [state.selectedKey] : []}
				onSelectToggle={(id) => {
					if (!disabled) {
						onSelectItem(family.value, id);
					}
				}}
				getId={(item) => item.__inspectorKey}
				emptyText={emptyText}
				scrollContainedBody={fillHeight}
				emptyAction={
					onList
						? {
							onClick: onList,
							disabled,
							loading: state.listing,
							ariaLabel: listActionLabel,
							headerIcon: (
								<InspectorCapabilityListActionGlyph
									listed={state.listed}
									listing={state.listing}
									className="h-10 w-10"
								/>
							),
						}
						: undefined
				}
			/>
		</div>
	);
}

export function InspectorCapabilityAccordionSidebar({
	familyStates,
	families = INSPECTOR_CAPABILITY_FAMILIES,
	activeFamily,
	onActiveFamilyChange,
	onList,
	onClear,
	onSelectItem,
	capabilitySearch,
	onCapabilitySearchChange,
	capabilitySearchOpen,
	onCapabilitySearchOpenChange,
	disabled = false,
}: InspectorCapabilityAccordionSidebarProps) {
	const firstFamily = families[0]?.value ?? DEFAULT_OPEN_FAMILY;
	const resolvedActiveFamily = activeFamily ?? firstFamily;
	const openFamilies = families.length > 0 ? [resolvedActiveFamily] : [];

	if (families.length === 0) {
		return (
			<p className="px-1 text-xs leading-relaxed text-muted-foreground">
				Authorize this target to discover the capability types it advertises.
			</p>
		);
	}

	return (
		<Accordion
			type="multiple"
			value={openFamilies}
			onValueChange={(values) => {
				const next = values[values.length - 1] as InspectorCapabilityFamily | undefined;
				onActiveFamilyChange(next ?? firstFamily);
			}}
			className="flex min-h-0 w-full flex-1 flex-col gap-1"
		>
			{families.map((family) => {
				const state = familyStates[family.value];
				const listedCount = state.items.length;
				const advertisedCount = family.advertisedCount ?? null;
				const countValue = capabilityFamilyCountValue({
					listing: state.listing,
					listed: state.listed,
					listedCount,
					advertisedCount,
				});
				const displayLabel = family.shortLabel ?? family.label;
				const listLabel = state.listed ? `Refresh ${family.label}` : `List ${family.label}`;

				const isActiveFamily = family.value === resolvedActiveFamily;
				const searchExpanded = isActiveFamily && capabilitySearchOpen && state.listed;
				const headerActionsWidth = state.listed ? "pr-[4.75rem]" : "pr-7";

				return (
					<AccordionItem
						key={family.value}
						value={family.value}
						className={cn(
							"group border-b-0 px-0",
							isActiveFamily ? "flex min-h-0 flex-1 flex-col" : "shrink-0",
						)}
					>
						<div className="relative shrink-0">
							<AccordionTrigger
								className={cn(
									"min-w-0 justify-start gap-1 overflow-visible px-0 py-1.5 text-sm hover:no-underline [&>svg]:hidden",
									isActiveFamily ? "text-foreground" : "text-muted-foreground",
									isActiveFamily && !searchExpanded && headerActionsWidth,
								)}
							>
								<span
									className={cn(
										"flex min-w-0 items-start gap-1.5 overflow-hidden text-left transition-[max-width,opacity] duration-200 ease-in-out",
										searchExpanded ? "max-w-0 opacity-0" : "max-w-full opacity-100",
									)}
								>
									<span className="inline-flex min-w-0 max-w-full items-start gap-0.5">
										<span className="truncate font-medium" title={family.label}>
											{displayLabel}
										</span>
										{countValue != null ? (
											<Badge
												variant="secondary"
												className="h-4 min-w-4 shrink-0 justify-center px-1 py-0 text-[10px] font-semibold leading-none"
											>
												{countValue}
											</Badge>
										) : null}
									</span>
									{family.placeholder ? (
										<Badge variant="outline" className="h-4 shrink-0 px-1 text-[10px]">
											2026
										</Badge>
									) : null}
								</span>
							</AccordionTrigger>
							{isActiveFamily ? (
								<div
									className={cn(
										"absolute inset-y-0 flex items-center justify-end gap-1 transition-[left,right] duration-200 ease-in-out",
										searchExpanded ? "left-0 right-0" : "right-0",
									)}
									onClick={(event) => event.stopPropagation()}
									onPointerDown={(event) => event.stopPropagation()}
								>
									{state.listed ? (
										<InspectorExpandableSearch
											value={capabilitySearch}
											onChange={onCapabilitySearchChange}
											open={capabilitySearchOpen}
											onOpenChange={onCapabilitySearchOpenChange}
											placeholder="Search"
											ariaLabel="Search capabilities"
											clearAriaLabel="Clear capability search"
											iconButtonClassName={capabilityHeaderIconInteractiveClassName(
												searchExpanded,
											)}
											expandedClassName="min-w-0 flex-1"
											closeOnClickOutside
										/>
									) : null}
									{state.listed ? (
										<button
											type="button"
											className={capabilityHeaderIconInteractiveClassName(searchExpanded)}
											disabled={disabled || state.listing}
											aria-label={`Clear ${family.label}`}
											title={`Clear ${family.label}`}
											onClick={(event) => {
												event.stopPropagation();
												onClear(family.value);
											}}
										>
											<BrushCleaning className="h-3.5 w-3.5" />
										</button>
									) : (
										<button
											type="button"
											className={cn(
												capabilityHeaderIconClassName,
												iconActionLightClassName,
											)}
											disabled={disabled || state.listing}
											aria-label={listLabel}
											title={listLabel}
											onClick={(event) => {
												event.stopPropagation();
												onList(family.value);
											}}
										>
											<InspectorCapabilityListActionGlyph
												listed={false}
												listing={state.listing}
												className="h-3.5 w-3.5"
											/>
										</button>
									)}
								</div>
							) : null}
						</div>
						<AccordionContent
							panelClassName={
								isActiveFamily
									? "data-[state=closed]:animate-none data-[state=open]:flex data-[state=open]:min-h-0 data-[state=open]:flex-1 data-[state=open]:flex-col data-[state=open]:animate-none"
									: undefined
							}
							className={isActiveFamily ? "flex min-h-0 flex-1 flex-col pb-0 pt-0" : "pb-0 pt-0"}
						>
							{isActiveFamily ? (
								<InspectorCapabilityFamilyList
									family={family}
									state={state}
									disabled={disabled}
									onSelectItem={onSelectItem}
									onList={() => onList(family.value)}
									filterText={capabilitySearch}
									fillHeight
								/>
							) : null}
						</AccordionContent>
					</AccordionItem>
				);
			})}
		</Accordion>
	);
}
