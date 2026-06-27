import {
	Database,
	LayoutTemplate,
	MessageSquare,
	Wrench,
	type LucideIcon,
} from "lucide-react";
import {
	type KeyboardEvent,
	type MouseEvent,
	type ReactNode,
	useEffect,
	useMemo,
	useState,
} from "react";
import { useTranslation } from "react-i18next";
import { useAppStore } from "../lib/store";
import { cn } from "../lib/utils";
import { BulkSelectionCheckbox } from "./bulk-selection";
import type {
	CapabilityArgument,
	CapabilityMapItem,
	CapabilityRecord,
} from "../types/capabilities";
import type { JsonSchema } from "../types/json";
import { CapabilityListSkeleton } from "./capability-list-skeleton";
import { CardListScrollBody } from "./card-list-scroll-body";
import { JsonCodeBlock } from "./json-code-block";
import { CachedAvatar } from "./cached-avatar";
import {
	CapsuleStripeList,
	CapsuleStripeListItem,
} from "./capsule-stripe-list";
import {
	CAPABILITY_SCROLL_CARD_CLASS,
	CapabilityScrollCardContent,
} from "./capability-scroll-card-layout";
import { Card, CardHeader, CardTitle } from "./ui/card";
import { Input } from "./ui/input";
import { Switch } from "./ui/switch";
import {
	TooltipProvider,
} from "./ui/tooltip";
import { SchemaTable } from "./schema-table";
import { TruncatedText } from "./truncated-text";
import {
	CAPABILITY_DETAILS_CLASS,
	CAPABILITY_SUMMARY_CLASS,
} from "./capability-disclosure-classes";

export type CapabilityKind = "tools" | "resources" | "prompts" | "templates";
type ContextType = "server" | "profile";
type CapabilityDetailsLoader<T> = (
	item: T,
	kind: CapabilityKind,
) => Promise<CapabilityRecord | null | undefined>;

const CAPABILITY_KIND_ICONS: Record<CapabilityKind, LucideIcon> = {
	tools: Wrench,
	resources: Database,
	prompts: MessageSquare,
	templates: LayoutTemplate,
};

export interface CapabilityListProps<T = CapabilityRecord> {
	title?: string;
	kind: CapabilityKind;
	getKind?: (item: T) => CapabilityKind;
	context?: ContextType;
	items: T[];
	loading?: boolean;
	filterText?: string;
	onFilterTextChange?: (text: string) => void;
	emptyText?: string;
	enableToggle?: boolean;
	getId?: (item: T) => string;
	getEnabled?: (item: T) => boolean;
	onToggle?: (id: string, next: boolean, item: T) => void;
	selectable?: boolean;
	selectedIds?: string[];
	onSelectToggle?: (id: string, item: T) => void;
	asCard?: boolean;
	leadingIcon?: "source" | "kind";
	/** Dense spacing between list items (space-y-2). */
	dense?: boolean;
	/** Render using CapsuleStripeList visual style. */
	capsule?: boolean;
	/** Hide actions until item hover. */
	hoverActions?: boolean;
	/** Clicking an item toggles the details block (if present). */
	clickToToggleDetails?: boolean;
	loadDetails?: CapabilityDetailsLoader<T>;
	detailsCacheScope?: string | number | null;
	renderAction?: (mapped: CapabilityMapItem<T>, item: T) => ReactNode;
	scrollBodyClassName?: string;
	/**
	 * When `asCard` is false and the list sits in a flex `Card` body, scroll inside this component so
	 * the host card keeps rounded corners (see `CardListScrollBody`).
	 */
	scrollContainedBody?: boolean;
}

function asString(v: unknown): string | undefined {
	if (v == null) return undefined;
	if (typeof v === "string") return v;
	if (typeof v === "number" || typeof v === "boolean") return String(v);
	return undefined;
}

function normalizeMultiline(text?: string): string | undefined {
	if (!text) return text;
	try {
		return text.replace(/\\r\\n/g, "\n").replace(/\\n/g, "\n");
	} catch {
		return text;
	}
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
	Boolean(value) && typeof value === "object" && !Array.isArray(value);

const toCapabilityRecord = (value: unknown): CapabilityRecord | null =>
	isRecord(value) ? (value as CapabilityRecord) : null;

const toIconArray = (value: unknown): unknown[] => {
	if (!value) return [];
	if (Array.isArray(value)) return value;
	return [value];
};

const extractIconSrc = (item: CapabilityRecord | null): string | undefined => {
	if (!item) return undefined;
	const meta = toCapabilityRecord(item.meta);
	const candidate = item.icons ?? item.icon ?? meta?.icons;
	const icons = toIconArray(candidate);
	for (const icon of icons) {
		if (isRecord(icon) && typeof icon.src === "string") {
			return icon.src;
		}
	}
	return undefined;
};

const toSchema = (value: unknown): JsonSchema | undefined => {
	if (!value) return undefined;
	const record = toCapabilityRecord(value);
	if (!record) return undefined;
	const nested = toCapabilityRecord(record.schema);
	if (nested) return nested as JsonSchema;
	return record as JsonSchema;
};

const toArguments = (value: unknown): CapabilityArgument[] | undefined => {
	if (!Array.isArray(value)) return undefined;
	return value.map((entry, index) => {
		if (!isRecord(entry)) {
			return { name: `arg_${index}` };
		}
		const name = asString(entry.name) ?? `arg_${index}`;
		const type = asString(entry.type);
		const description = asString(entry.description);
		const required =
			typeof entry.required === "boolean" ? entry.required : undefined;
		return {
			name,
			type: type ?? undefined,
			description: description ?? undefined,
			required,
		};
	});
};

const resolveInputSchema = (
	source: CapabilityRecord,
): JsonSchema | undefined => {
	const candidates = [source.input_schema, source.inputSchema, source.schema];
	for (const candidate of candidates) {
		const schema = toSchema(candidate);
		if (schema) {
			if (!schema.type && schema.properties) {
				schema.type = "object";
			}
			return schema;
		}
	}
	return undefined;
};

const resolveOutputSchema = (
	source: CapabilityRecord,
): JsonSchema | undefined => {
	const candidates = [source.output_schema, source.outputSchema];
	for (const candidate of candidates) {
		const schema = toSchema(candidate);
		if (schema) {
			if (!schema.type && schema.properties) {
				schema.type = "object";
			}
			return schema;
		}
	}
	return undefined;
};

function mapItem<T>(kind: CapabilityKind, item: T): CapabilityMapItem<T> {
	const record = toCapabilityRecord(item) ?? ({} as CapabilityRecord);
	if (kind === "tools") {
		const unique = asString(record.unique_name);
		const name = asString(record.tool_name) || asString(record.name);
		const title = unique || name || asString(record.id) || "Untitled Tool";
		const description = normalizeMultiline(asString(record.description));
		const schema = resolveInputSchema(record);
		const outputSchema = resolveOutputSchema(record);
		const args = toArguments(record.arguments);
		return {
			title,
			subtitle: undefined, // Remove original tool name display
			description,
			server: asString(record.server_name),
			raw: item,
			schema,
			outputSchema,
			args,
			icon: extractIconSrc(record),
		};
	}

	if (kind === "resources") {
		const title =
			asString(record.resource_uri) ||
			asString(record.uri) ||
			asString(record.name) ||
			"Resource";
		const description = normalizeMultiline(asString(record.description));
		return {
			title,
			subtitle: asString(record.name),
			server: asString(record.server_name),
			mime: asString(record.mime_type),
			description,
			raw: item,
			icon: extractIconSrc(record),
		};
	}

	if (kind === "prompts") {
		const title =
			asString(record.prompt_name) || asString(record.name) || "Prompt";
		const description = normalizeMultiline(asString(record.description));
		const args = toArguments(record.arguments);
		return {
			title,
			server: asString(record.server_name),
			description,
			args,
			raw: item,
			icon: extractIconSrc(record),
		};
	}

	// Templates: show concise row like other capabilities, with server only
	const uriTemplate = asString(record.uriTemplate) ?? asString(record.uri_template);
	const title = uriTemplate || asString(record.name) || "Template";
	const description = normalizeMultiline(asString(record.description));
	return {
		title,
		subtitle: undefined,
		server: asString(record.server_name),
		description,
		raw: item,
		icon: extractIconSrc(record),
	};
}

function matchText<T>(obj: CapabilityMapItem<T>, needle: string): boolean {
	if (!needle) return true;
	const lower = needle.toLowerCase();
	try {
		const fields = [obj.title, obj.subtitle, obj.server, obj.description]
			.filter(Boolean)
			.join(" \n ")
			.toLowerCase();
		if (fields.includes(lower)) return true;
		return JSON.stringify(obj.raw).toLowerCase().includes(lower);
	} catch {
		return false;
	}
}

const INTERACTIVE_TARGET_SELECTOR =
	"button, a, input, textarea, select, [role=button]";

function hasActiveTextSelection(): boolean {
	const selection = typeof window !== "undefined" ? window.getSelection() : null;
	return Boolean(selection?.toString().trim());
}

function isNestedInteractiveTarget(
	target: HTMLElement | null,
	boundary: HTMLElement,
): boolean {
	let current: HTMLElement | null = target;
	while (current && current !== boundary) {
		if (current.matches?.(INTERACTIVE_TARGET_SELECTOR)) {
			return true;
		}
		current = current.parentElement;
	}
	return false;
}

function isDetailsTarget(target: HTMLElement): boolean {
	return Boolean(target.closest("summary") || target.closest("details"));
}

export function CapabilityList<T = CapabilityRecord>({
	title,
	kind,
	getKind,
	context = "server",
	items,
	loading,
	filterText,
	onFilterTextChange,
	emptyText,
	enableToggle,
	getId,
	getEnabled,
	onToggle,
	selectable,
	selectedIds,
	onSelectToggle,
	asCard,
	leadingIcon = "source",
	dense,
	capsule,
	hoverActions,
	clickToToggleDetails,
	loadDetails,
	detailsCacheScope,
	renderAction,
	scrollBodyClassName,
	scrollContainedBody,
}: CapabilityListProps<T>) {
	const [internalFilter, setInternalFilter] = useState("");
	const [openMap, setOpenMap] = useState<Record<string, boolean>>({});
	const [lazyDetailsById, setLazyDetailsById] = useState<Record<string, CapabilityRecord | null>>({});
	const [lazyDetailsLoadingById, setLazyDetailsLoadingById] = useState<Record<string, boolean>>({});
	const [lazyDetailsErrorById, setLazyDetailsErrorById] = useState<Record<string, string | null>>({});
	const search = filterText ?? internalFilter;
	const showRawJson = useAppStore(
		(state) => state.dashboardSettings.showRawCapabilityJson,
	);
	const { t } = useTranslation();

	const useCapsule = capsule ?? (context === "server");
	const shouldClickToToggleDetails = clickToToggleDetails ?? true;

	useEffect(() => {
		setOpenMap({});
		setLazyDetailsById({});
		setLazyDetailsLoadingById({});
		setLazyDetailsErrorById({});
	}, [detailsCacheScope]);

	const mappedItems = useMemo(
		() =>
			(items || []).map((it) => {
				const itemKind = getKind ? getKind(it) : kind;
				return {
					kind: itemKind,
					mapped: mapItem(itemKind, it),
				};
			}),
		[items, kind, getKind],
	);
	const data = useMemo(
		() => mappedItems
			.filter(({ mapped }) => matchText(mapped, search))
			.sort((a, b) => a.mapped.title.localeCompare(b.mapped.title)),
		[mappedItems, search],
	);
	const selectedIdSet = useMemo(
		() => (selectedIds ? new Set(selectedIds) : null),
		[selectedIds],
	);

	const useStripeSkeleton = context === "profile" || useCapsule;
	const skeleton = useStripeSkeleton ? (
		<CapabilityListSkeleton
			className={scrollContainedBody ? "p-0" : undefined}
			fillContainer={scrollContainedBody}
		/>
	) : (
		<div className="space-y-2">
			{[1, 2, 3].map((i) => (
				<div
					key={i}
					className="h-10 animate-pulse rounded bg-slate-200 dark:bg-slate-800"
				/>
			))}
		</div>
	);

	const renderedItems = data.map(({ kind: itemKind, mapped }, idx) => {
		const item = mapped.raw;
		const id = getId ? getId(item) : String(idx);
		const isSelected = !!(selectable && selectedIdSet?.has(id));
		const isEnabled = getEnabled ? !!getEnabled(item) : undefined;
		const hasLazyDetails = Object.prototype.hasOwnProperty.call(
			lazyDetailsById,
			id,
		);
		const lazyDetails = lazyDetailsById[id];
		const lazyDetailsLoading = !!lazyDetailsLoadingById[id];
		const lazyDetailsError = lazyDetailsErrorById[id];
		const mergedDetailsRecord = lazyDetails
			? ({
				...(toCapabilityRecord(item) ?? {}),
				...lazyDetails,
			} as CapabilityRecord)
			: null;
		const detailsMapped = mergedDetailsRecord
			? mapItem(itemKind, mergedDetailsRecord as T)
			: mapped;

		const ensureLazyDetails = () => {
			if (!loadDetails || hasLazyDetails || lazyDetailsLoading) {
				return;
			}
			setLazyDetailsLoadingById((prev) => ({ ...prev, [id]: true }));
			setLazyDetailsErrorById((prev) => ({ ...prev, [id]: null }));
			void loadDetails(item, itemKind)
				.then((details) => {
					setLazyDetailsById((prev) => ({ ...prev, [id]: details ?? null }));
				})
				.catch((error: unknown) => {
					setLazyDetailsErrorById((prev) => ({
						...prev,
						[id]: error instanceof Error ? error.message : String(error),
					}));
				})
				.finally(() => {
					setLazyDetailsLoadingById((prev) => ({ ...prev, [id]: false }));
				});
		};

		const handleSelect = () => {
			if (selectable && onSelectToggle) onSelectToggle(id, item);
		};

		const handleKeyDown = (event: KeyboardEvent<HTMLElement>) => {
			if (!selectable || !onSelectToggle) return;
			if (event.key === "Enter" || event.key === " ") {
				event.preventDefault();
				onSelectToggle(id, item);
			}
		};

		const schemaEntries = detailsMapped.schema?.properties
			? Object.entries(detailsMapped.schema.properties)
			: [];
		const outputSchemaEntries = detailsMapped.outputSchema?.properties
			? Object.entries(detailsMapped.outputSchema.properties)
			: [];
		const hasArgs = Boolean(detailsMapped.args?.length);
		const hasSchema = schemaEntries.length > 0;
		const hasOutSchema = outputSchemaEntries.length > 0;
		const hasRaw = showRawJson && detailsMapped.raw != null;
		const hasLoadedDetails = hasArgs || hasSchema || hasOutSchema || hasRaw;
		const hasDetails = hasLoadedDetails || Boolean(loadDetails);
		const isBulkSelectionMode = !!(selectable && onSelectToggle);

		const toggleDetails = () => {
			if (!shouldClickToToggleDetails || !hasDetails) return;
			const nextOpen = !openMap[id];
			setOpenMap((prev) => ({ ...prev, [id]: nextOpen }));
			if (nextOpen) {
				ensureLazyDetails();
			}
		};

		const handleItemClick = (e: MouseEvent<HTMLElement>) => {
			const target = e.target as HTMLElement;
			if (hasActiveTextSelection()) return;
			if (isNestedInteractiveTarget(target, e.currentTarget)) return;
			if (isDetailsTarget(target)) return;
			if (isBulkSelectionMode) {
				handleSelect();
				return;
			}
			if (!shouldClickToToggleDetails || !hasDetails) return;
			toggleDetails();
		};

		const handleDetailsKeyDown = (e: KeyboardEvent<HTMLElement>) => {
			const target = e.target as HTMLElement;
			if (isNestedInteractiveTarget(target, e.currentTarget)) return;
			if (isBulkSelectionMode) {
				handleKeyDown(e);
				return;
			}
			if (!shouldClickToToggleDetails || !hasDetails) return;
			if (e.key === "Enter" || e.key === " ") {
				e.preventDefault();
				toggleDetails();
			}
		};
		const isRowInteractive = hasDetails || isBulkSelectionMode;

		const titleClasses =
			context === "profile" && isSelected
				? "font-medium text-primary"
				: "font-medium";

		const KindIcon = CAPABILITY_KIND_ICONS[itemKind];
		const leadingNode =
			context === "server" || leadingIcon === "kind" ? (
				<KindIcon
					className={cn(
						"h-4 w-4 shrink-0 text-slate-500 dark:text-slate-400",
						onSelectToggle == null && "mt-0.5",
					)}
					aria-hidden
				/>
			) : (
				<CachedAvatar
					src={mapped.icon}
					fallback={mapped.title || itemKind}
					size="sm"
					className="border border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-900/40"
				/>
			);

		const descriptionBlock = mapped.description ? (
			<TruncatedText className="mt-1 text-xs text-slate-600 dark:text-slate-300">
				{mapped.description}
			</TruncatedText>
		) : null;

		const detailsBlock = hasDetails ? (
			<details
				className={CAPABILITY_DETAILS_CLASS}
				open={!!openMap[id]}
				onToggle={(e) => {
					const isOpen = (e.currentTarget as HTMLDetailsElement).open;
					setOpenMap((prev) => ({ ...prev, [id]: isOpen }));
					if (isOpen) {
						ensureLazyDetails();
					}
				}}
			>
				<summary className={CAPABILITY_SUMMARY_CLASS}>
					{t("servers:capabilityList.detailsToggle", {
						defaultValue: "Details",
					})}
				</summary>
				<div className="mt-2 space-y-2">
					{hasArgs ? (
						<div className="overflow-x-auto">
							<table className="w-full border-collapse text-xs">
								<thead>
									<tr className="text-left text-slate-500">
										<th className="border-b py-1 pr-2">
											{t("servers:capabilityList.table.argument", {
												defaultValue: "Argument",
											})}
										</th>
										<th className="border-b py-1 pr-2">
											{t("servers:capabilityList.table.required", {
												defaultValue: "Required",
											})}
										</th>
										<th className="border-b py-1 pr-2">
											{t("servers:capabilityList.table.description", {
												defaultValue: "Description",
											})}
										</th>
									</tr>
								</thead>
								<tbody>
									{detailsMapped.args?.map((arg, argIdx) => (
										<tr key={`${arg.name ?? `arg_${argIdx}`}-${argIdx}`}>
											<td className="border-b py-1 pr-2 font-mono">
												{arg.name ?? `arg_${argIdx}`}
											</td>
											<td className="border-b py-1 pr-2">
												{arg.required
													? t("servers:capabilityList.table.requiredYes", {
														defaultValue: "Yes",
													})
													: t("servers:capabilityList.table.requiredNo", {
														defaultValue: "No",
													})}
											</td>
											<td className="border-b py-1 pr-2">
												{arg.description ?? ""}
											</td>
										</tr>
									))}
								</tbody>
							</table>
						</div>
					) : null}

					{hasSchema ? (
						<div className="overflow-x-auto">
							<div className="mb-1 text-xs text-slate-500">
								{t("servers:capabilityList.inputSchemaTitle", {
									defaultValue: "Input Schema",
								})}
							</div>
							<SchemaTable schema={detailsMapped.schema as JsonSchema} />
						</div>
					) : null}

					{hasOutSchema ? (
						<div className="overflow-x-auto">
							<div className="mb-1 text-xs text-slate-500">
								{t("servers:capabilityList.outputSchemaTitle", {
									defaultValue: "Output Schema",
								})}
							</div>
							<SchemaTable schema={detailsMapped.outputSchema as JsonSchema} />
						</div>
					) : null}

					{hasRaw ? (
						<JsonCodeBlock code={JSON.stringify(detailsMapped.raw, null, 2)} />
					) : null}
					{lazyDetailsLoading ? (
						<div className="text-xs text-slate-500 dark:text-slate-400">
							{t("servers:capabilityList.loadingDetails", {
								defaultValue: "Loading details...",
							})}
						</div>
					) : null}
					{lazyDetailsError ? (
						<div className="text-xs text-red-600 dark:text-red-400">
							{t("servers:capabilityList.detailsLoadFailed", {
								defaultValue: "Failed to load details.",
							})}{" "}
							{lazyDetailsError}
						</div>
					) : null}
					{loadDetails &&
					hasLazyDetails &&
					!lazyDetails &&
					!lazyDetailsLoading &&
					!hasLoadedDetails ? (
						<div className="text-xs text-slate-500 dark:text-slate-400">
							{t("servers:capabilityList.noDetails", {
								defaultValue: "No additional details available.",
							})}
						</div>
					) : null}
				</div>
			</details>
		) : null;

		const infoBlock = (
			<div className="min-w-0 flex-1 overflow-hidden select-text">
				<div
					className={titleClasses}
					title={
						mapped.server
							? t("profiles:detail.labels.capabilityServerTooltip", {
								server: mapped.server,
								defaultValue: "Server: {{server}}",
							})
							: undefined
					}
				>
					{mapped.title}
					{mapped.subtitle ? (
						<span className="ml-2 text-xs text-slate-500">
							{mapped.subtitle}
						</span>
					) : null}
				</div>
				{mapped.mime ? (
					<div className="text-sm text-slate-500">Mime: {mapped.mime}</div>
				) : null}
				{descriptionBlock}
				{detailsBlock}
			</div>
		);

		const selectionNode = onSelectToggle ? (
			<BulkSelectionCheckbox
				visible={!!selectable}
				checked={isSelected}
				onToggle={handleSelect}
				ariaLabel={t("profiles:detail.bulk.selectItem", {
					name: mapped.title,
					defaultValue: "Select {{name}}",
				})}
			/>
		) : null;

		const leadGroup =
			onSelectToggle != null ? (
				<div
					className={`flex shrink-0 items-center ${selectable ? "gap-3" : "gap-0"}`}
				>
					{selectionNode}
					{leadingNode}
				</div>
			) : (
				leadingNode
			);

		const leftSection = (
			<div className="flex min-w-0 flex-1 items-start gap-3">
				{leadGroup}
				{infoBlock}
			</div>
		);

		const actions = (
			<>
				{context === "profile" && enableToggle && getEnabled && onToggle ? (
					<Switch
						checked={!!isEnabled}
						onCheckedChange={(next) => onToggle(id, next, item)}
						onClick={(e) => e.stopPropagation()}
					/>
				) : null}
				{renderAction ? (
					<div
						role="presentation"
						onClick={(e) => e.stopPropagation()}
						onKeyDown={(e) => {
							if (e.key === "Enter" || e.key === " ") {
								e.preventDefault();
								e.stopPropagation();
							}
						}}
					>
						{renderAction(mapped, item)}
					</div>
				) : null}
			</>
		);

		const actionSection = (
			<div
				className={`ml-auto flex items-start gap-2 ${hoverActions ? "opacity-0 group-hover:opacity-100 transition-opacity" : ""
					}`}
			>
				{actions}
			</div>
		);

		if (context === "profile") {
			return (
				<CapsuleStripeListItem
					key={id}
					interactive={isRowInteractive}
					className={`group relative px-3 transition-colors ${isSelected
						? "bg-primary/10 ring-1 ring-slate-200/80 dark:ring-slate-700/60"
						: ""
						}`}
					onClick={handleItemClick}
					onKeyDown={handleDetailsKeyDown}
				>
					<div className="flex w-full min-w-0 items-start justify-between gap-3 overflow-hidden">
						{leftSection}
						{actionSection}
					</div>
				</CapsuleStripeListItem>
			);
		}

		if (useCapsule) {
			return (
				<CapsuleStripeListItem
					key={id}
					interactive={isRowInteractive}
					className="group"
					onClick={handleItemClick}
					onKeyDown={handleDetailsKeyDown}
				>
					<div className="flex w-full min-w-0 items-start justify-between gap-3 overflow-hidden">
						{leftSection}
						{actionSection}
					</div>
				</CapsuleStripeListItem>
			);
		}

		return (
			<li
				key={id}
				className={`rounded border p-3 ${isSelected ? "bg-accent/50 ring-1 ring-slate-200/80 dark:ring-slate-700/60" : ""}`}
				role={isRowInteractive ? "button" : undefined}
				tabIndex={isRowInteractive ? 0 : undefined}
				onClick={handleItemClick}
				onKeyDown={handleDetailsKeyDown}
			>
				<div className="flex items-start justify-between gap-3">
					{leftSection}
					{actionSection}
				</div>
			</li>
		);
	});

	const listContent = (
		<TooltipProvider delayDuration={200} disableHoverableContent={false}>
			{context === "profile" || useCapsule ? (
				<CapsuleStripeList
					className={
						scrollContainedBody
							? "rounded-none border-0 overflow-visible"
							: undefined
					}
				>
					{renderedItems}
				</CapsuleStripeList>
			) : (
				<ul
					className={`${dense || asCard === false ? "space-y-2" : "space-y-4"} text-sm`}
				>
					{renderedItems}
				</ul>
			)}
		</TooltipProvider>
	);

	const isEmpty = !loading && renderedItems.length === 0;

	const list = (
		<div
			className={
				isEmpty
					? "flex min-h-full w-full flex-col items-center justify-center px-4 py-8 text-center"
					: loading && scrollContainedBody
						? "min-h-full"
						: undefined
			}
		>
			{loading ? (
				skeleton
			) : renderedItems.length ? (
				listContent
			) : (
				<p className="text-sm text-slate-500 dark:text-slate-400">
					{emptyText ||
						t("servers:capabilityList.emptyFallback", {
							defaultValue: "No data.",
						})}
				</p>
			)}
		</div>
	);

	if (asCard === false) {
		if (scrollContainedBody) {
			return (
				<CardListScrollBody
					className={scrollBodyClassName}
					scrollLocked={loading}
				>
					{list}
				</CardListScrollBody>
			);
		}
		return list;
	}

	const showSearch =
		typeof onFilterTextChange === "function" || filterText === undefined;
	const kindLabel = t(`servers:detail.capabilityList.labels.${kind}`, {
		defaultValue: kind[0].toUpperCase() + kind.slice(1),
	});
	const searchPlaceholder = t("servers:capabilityList.searchPlaceholder", {
		label: kindLabel,
		defaultValue: `Search ${kind}...`,
	});

	const body = scrollContainedBody ? (
		<CardListScrollBody
			className={scrollBodyClassName}
			scrollLocked={loading}
		>
			{list}
		</CardListScrollBody>
	) : (
		list
	);

	return (
		<Card
			className={
				scrollContainedBody
					? CAPABILITY_SCROLL_CARD_CLASS
					: undefined
			}
		>
			<CardHeader
				className={scrollContainedBody ? "shrink-0" : undefined}
			>
				<div className="flex items-center justify-between gap-2">
					<CardTitle>{title ?? kindLabel}</CardTitle>
					{showSearch ? (
						<Input
							placeholder={searchPlaceholder}
							className="w-56"
							value={search}
							onChange={(event) => {
								if (onFilterTextChange) onFilterTextChange(event.target.value);
								else setInternalFilter(event.target.value);
							}}
						/>
					) : null}
				</div>
			</CardHeader>
			{scrollContainedBody ? (
				<CapabilityScrollCardContent>{body}</CapabilityScrollCardContent>
			) : (
				body
			)}
		</Card>
	);
}

export default CapabilityList;
