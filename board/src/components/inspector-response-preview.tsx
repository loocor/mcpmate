import { ChevronRight } from "lucide-react";
import {
	forwardRef,
	useEffect,
	useImperativeHandle,
	useMemo,
	useState,
	type KeyboardEvent,
} from "react";
import { useTranslation } from "react-i18next";
import type { TFunction } from "i18next";
import type { InspectorCapabilityKind } from "../lib/inspector-capability";
import {
	buildInspectorJsonOutline,
	parseInspectorResponsePreview,
	type InspectorJsonOutlineRow,
	type InspectorJsonOutlineSummaryMeta,
} from "../lib/inspector-response-preview";
import { cn } from "../lib/utils";
import { JsonCodeBlock } from "./json-code-block";
import { LazyImage } from "./lazy-image";

export const INSPECTOR_PAYLOAD_SURFACE_CLASSNAME =
	"min-w-0 overflow-auto rounded-md bg-slate-50 font-mono text-xs dark:bg-slate-900";

const JSON_OUTLINE_INDENT_PX = 12;
const JSON_OUTLINE_BASE_PADDING_PX = 8;

const JSON_OUTLINE_TYPE_COLUMN_CLASSNAME =
	"w-[4.5rem] shrink-0 text-right opacity-50 transition-opacity group-hover/outline-row:opacity-100 group-focus-within/outline-row:opacity-100";

const JSON_OUTLINE_TYPE_CLASSNAMES: Record<InspectorJsonOutlineRow["type"], string> = {
	array: "border-blue-200 bg-blue-50 text-blue-700 dark:border-blue-900/70 dark:bg-blue-950/40 dark:text-blue-300",
	boolean: "border-amber-200 bg-amber-50 text-amber-700 dark:border-amber-900/70 dark:bg-amber-950/40 dark:text-amber-300",
	null: "border-slate-200 bg-slate-50 text-slate-500 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-400",
	number: "border-emerald-200 bg-emerald-50 text-emerald-700 dark:border-emerald-900/70 dark:bg-emerald-950/40 dark:text-emerald-300",
	object: "border-indigo-200 bg-indigo-50 text-indigo-700 dark:border-indigo-900/70 dark:bg-indigo-950/40 dark:text-indigo-300",
	string: "border-rose-200 bg-rose-50 text-rose-700 dark:border-rose-900/70 dark:bg-rose-950/40 dark:text-rose-300",
	truncated: "border-slate-200 bg-slate-50 text-slate-500 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-400",
	undefined: "border-slate-200 bg-slate-50 text-slate-500 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-400",
	unknown: "border-slate-200 bg-slate-50 text-slate-500 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-400",
};

const JSON_OUTLINE_CONTAINER_TYPES = new Set<InspectorJsonOutlineRow["type"]>(["array", "object"]);

const JSON_OUTLINE_VALUE_CLASSNAMES: Partial<Record<InspectorJsonOutlineRow["type"], string>> = {
	boolean: "text-amber-700 dark:text-amber-300",
	null: "text-slate-500 dark:text-slate-400",
	number: "text-emerald-700 dark:text-emerald-300",
	string: "text-rose-700 dark:text-rose-400",
	truncated: "italic text-slate-500 dark:text-slate-400",
	undefined: "text-slate-500 dark:text-slate-400",
	unknown: "text-slate-500 dark:text-slate-400",
};

function formatJsonOutlineSummary(
	row: InspectorJsonOutlineRow,
	t: TFunction,
): string {
	const meta = row.summaryMeta;
	if (!meta) {
		return row.summary;
	}
	return formatJsonOutlineSummaryMeta(meta, t, row.summary);
}

function formatJsonOutlineSummaryMeta(
	meta: InspectorJsonOutlineSummaryMeta,
	t: TFunction,
	fallback: string,
): string {
	switch (meta.kind) {
		case "keys":
			return t(meta.count === 1 ? "jsonOutline.summary.key" : "jsonOutline.summary.keys", {
				defaultValue: meta.count === 1 ? "{{count}} key" : "{{count}} keys",
				count: meta.count,
			});
		case "items":
			return t(meta.count === 1 ? "jsonOutline.summary.item" : "jsonOutline.summary.items", {
				defaultValue: meta.count === 1 ? "{{count}} item" : "{{count}} items",
				count: meta.count,
			});
		case "emptyObject":
			return t("jsonOutline.summary.emptyObject", { defaultValue: "{}" });
		case "emptyArray":
			return t("jsonOutline.summary.emptyArray", { defaultValue: "[]" });
		case "truncatedRows":
			return t("jsonOutline.summary.maxRows", {
				defaultValue: "Additional entries hidden after max rows",
			});
		case "truncatedDepth":
			return t("jsonOutline.summary.maxDepth", {
				defaultValue: "Nested entries hidden after max depth",
			});
		default:
			return fallback;
	}
}

function buildDefaultCollapsedRowIds(rows: InspectorJsonOutlineRow[]): Set<string> {
	const collapsed = new Set<string>();
	for (const row of rows) {
		if (row.hasChildren && row.depth >= 1) {
			collapsed.add(row.id);
		}
	}
	return collapsed;
}

function buildAllCollapsedRowIds(rows: InspectorJsonOutlineRow[]): Set<string> {
	const collapsed = new Set<string>();
	for (const row of rows) {
		if (row.hasChildren) {
			collapsed.add(row.id);
		}
	}
	return collapsed;
}

export type InspectorJsonOutlineHandle = {
	expandAll: () => void;
	collapseAll: () => void;
};

export const InspectorJsonOutline = forwardRef<
	InspectorJsonOutlineHandle,
	{
		value: unknown;
		className?: string;
	}
>(function InspectorJsonOutline({ value, className }, ref) {
	const { t } = useTranslation("inspector");
	const rows = useMemo(() => buildInspectorJsonOutline(value), [value]);
	const [collapsedRowIds, setCollapsedRowIds] = useState<Set<string>>(() => new Set());

	useEffect(() => {
		setCollapsedRowIds(buildDefaultCollapsedRowIds(rows));
	}, [rows, value]);

	useImperativeHandle(
		ref,
		() => ({
			expandAll: () => setCollapsedRowIds(new Set()),
			collapseAll: () => setCollapsedRowIds(buildAllCollapsedRowIds(rows)),
		}),
		[rows],
	);

	const visibleRows = useMemo(() => {
		const nextRows: InspectorJsonOutlineRow[] = [];
		let hiddenDepth: number | null = null;
		for (const row of rows) {
			if (hiddenDepth !== null) {
				if (row.depth > hiddenDepth) {
					continue;
				}
				hiddenDepth = null;
			}
			nextRows.push(row);
			if (collapsedRowIds.has(row.id) && row.hasChildren) {
				hiddenDepth = row.depth;
			}
		}
		return nextRows;
	}, [collapsedRowIds, rows]);

	function toggleRow(rowId: string): void {
		setCollapsedRowIds((current) => {
			const next = new Set(current);
			if (next.has(rowId)) {
				next.delete(rowId);
			} else {
				next.add(rowId);
			}
			return next;
		});
	}

	function handleRowKeyDown(event: KeyboardEvent<HTMLDivElement>, rowId: string): void {
		if (event.key === "Enter" || event.key === " ") {
			event.preventDefault();
			toggleRow(rowId);
		}
	}

	return (
		<div className={cn(INSPECTOR_PAYLOAD_SURFACE_CLASSNAME, "flex min-h-0 flex-col", className)}>
			<div className="divide-y divide-slate-200/70 dark:divide-slate-800/80">
				{visibleRows.map((row) => {
					const isExpandable = row.hasChildren;
					const isCollapsed = collapsedRowIds.has(row.id);
					const isContainer = JSON_OUTLINE_CONTAINER_TYPES.has(row.type);
					const summary = formatJsonOutlineSummary(row, t);
					const rowLabel =
						row.label === "$"
							? t("jsonOutline.root", { defaultValue: "root" })
							: row.label;

					return (
						<div
							key={row.id}
							role={isExpandable ? "button" : undefined}
							tabIndex={isExpandable ? 0 : undefined}
							aria-expanded={isExpandable ? !isCollapsed : undefined}
							aria-label={
								isExpandable
									? isCollapsed
										? t("jsonOutline.expandNode", { defaultValue: "Expand JSON node" })
										: t("jsonOutline.collapseNode", { defaultValue: "Collapse JSON node" })
									: undefined
							}
							className={cn(
								"group/outline-row grid grid-cols-[0.875rem_minmax(0,1fr)_4.5rem] items-center gap-2 py-1.5 pr-2",
								isExpandable &&
								"cursor-pointer hover:bg-slate-100/80 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring dark:hover:bg-slate-800/50",
							)}
							style={{
								paddingLeft: JSON_OUTLINE_BASE_PADDING_PX + row.depth * JSON_OUTLINE_INDENT_PX,
							}}
							onClick={isExpandable ? () => toggleRow(row.id) : undefined}
							onKeyDown={
								isExpandable ? (event) => handleRowKeyDown(event, row.id) : undefined
							}
						>
							<span className="flex h-3.5 w-3.5 shrink-0 items-center justify-center">
								{isExpandable ? (
									<ChevronRight
										className={cn(
											"h-3.5 w-3.5 text-muted-foreground transition-transform",
											!isCollapsed && "rotate-90",
										)}
										aria-hidden
									/>
								) : null}
							</span>
							<span
								className="min-w-0 truncate text-left"
								title={`${rowLabel}: ${summary}`}
							>
								<span
									className={cn(
										"font-medium text-slate-800 dark:text-slate-200",
										row.type === "truncated" && "text-muted-foreground",
									)}
								>
									{rowLabel}
								</span>
								<span className="text-muted-foreground">: </span>
								<span
									className={cn(
										isContainer
											? "text-muted-foreground"
											: JSON_OUTLINE_VALUE_CLASSNAMES[row.type],
									)}
								>
									{summary}
								</span>
							</span>
							<span className={JSON_OUTLINE_TYPE_COLUMN_CLASSNAME}>
								<span
									className={cn(
										"inline-block rounded border px-1 py-px font-mono text-[10px] uppercase leading-none",
										JSON_OUTLINE_TYPE_CLASSNAMES[row.type],
									)}
								>
									{t(`jsonOutline.types.${row.type}`, { defaultValue: row.type })}
								</span>
							</span>
						</div>
					);
				})}
			</div>
		</div>
	);
});

export function InspectorResponsePreview({
	result,
	kind,
	className,
}: {
	result: unknown;
	kind: InspectorCapabilityKind;
	className?: string;
}) {
	const blocks = parseInspectorResponsePreview(result, kind);

	if (blocks.length === 0) {
		return (
			<div className={cn("text-sm text-muted-foreground", className)}>
				No previewable MCP content.
			</div>
		);
	}

	return (
		<div className={cn("min-w-0 space-y-3 overflow-y-auto", className)}>
			{blocks.map((block, index) => {
				if (block.kind === "image") {
					return (
						<div
							key={`image-${index}`}
							className="rounded-md bg-slate-50 p-2 dark:bg-slate-900"
						>
							<LazyImage
								src={block.src}
								alt={block.alt ?? ""}
								cacheKey={block.src.slice(0, 128)}
								className="block max-w-full"
								imgClassName="block h-auto max-w-full rounded-md border border-slate-200 object-contain dark:border-slate-800"
								fallback={
									<div className="rounded-md border border-dashed border-slate-300 px-3 py-6 text-center text-xs text-muted-foreground dark:border-slate-700">
										{block.mimeType ?? "image"}
									</div>
								}
							/>
						</div>
					);
				}

				if (block.format === "markdown") {
					return (
						<JsonCodeBlock
							key={`markdown-${index}`}
							code={block.text}
							language="markdown"
						/>
					);
				}

				return (
					<pre
						key={`text-${index}`}
						className="m-0 whitespace-pre-wrap break-words rounded-md bg-slate-50 p-3 text-sm text-slate-700 dark:bg-slate-900 dark:text-slate-200"
					>
						{block.text}
					</pre>
				);
			})}
		</div>
	);
}
