import { ChevronDown, ChevronRight, GripVertical } from "lucide-react";
import {
	Fragment,
	type MouseEvent as ReactMouseEvent,
	type ReactNode,
	type RefObject,
	useCallback,
	useEffect,
	useLayoutEffect,
	useRef,
	useState,
} from "react";
import type {
	ActivityLogDataColumnId,
	ActivityLogTableLayout,
} from "../lib/activity-log-table-layout";
import {
	ACTIVITY_LOG_DATA_COLUMN_IDS,
	ACTIVITY_LOG_COLUMN_AUTOSIZE_PADDING_PX,
	ACTIVITY_LOG_MIN_COLUMN_WIDTH,
	DEFAULT_ACTIVITY_LOG_TABLE_LAYOUT,
	INSPECTOR_ACTIVITY_TABLE_LAYOUT_STORAGE_KEY,
	loadActivityLogTableLayout,
	reorderActivityLogColumns,
	resizeActivityLogColumnWidth,
	saveActivityLogTableLayout,
	setActivityLogColumnWidth,
} from "../lib/activity-log-table-layout";
import type {
	ActivityLogRow,
	ActivityLogTableHeaders,
	ActivityLogTableSize,
} from "../lib/activity-log-row";
import { cn, formatLocalDateTime } from "../lib/utils";

/** Fixed-width gutter so ChevronRight/ChevronDown swaps do not shift the layout */
const EXPAND_COL_WIDTH_PX = 40;
const EXPAND_COL_BASE = "box-border w-10 min-w-10 max-w-10 px-0 pl-1 pr-2";
const EXPAND_COL_SPACER_CLASS = `${EXPAND_COL_BASE} border-b-0 p-0 align-middle`;

const ACTIVITY_LOG_ROW_CLASSNAME =
	"group/row border-b align-middle opacity-75 transition-opacity hover:opacity-100";
const ACTIVITY_LOG_ROW_EXPANDABLE_CLASSNAME =
	"cursor-pointer";
const ACTIVITY_LOG_ROW_SURFACE_ODD = "bg-background";
const ACTIVITY_LOG_ROW_SURFACE_EVEN = "bg-slate-50/50 dark:bg-slate-900/40";
const ACTIVITY_LOG_ROW_SURFACE_HOVER =
	"group-hover/row:bg-slate-100/70 dark:group-hover/row:bg-slate-800/60";
const ACTIVITY_LOG_TABLE_RULE_CLASSNAME = "bg-slate-200 dark:bg-slate-700";

const COLUMN_STACK_Z_CLASS = [
	"z-[1]",
	"z-[2]",
	"z-[3]",
	"z-[4]",
	"z-[5]",
	"z-[6]",
	"z-[7]",
	"z-[8]",
] as const;

function columnStackZClass(stackIndex: number): string {
	const index = Math.max(1, Math.min(stackIndex, COLUMN_STACK_Z_CLASS.length)) - 1;
	return COLUMN_STACK_Z_CLASS[index] ?? "z-[1]";
}

function activityLogRowSurface(rowIndex: number): string {
	return rowIndex % 2 === 0 ? ACTIVITY_LOG_ROW_SURFACE_ODD : ACTIVITY_LOG_ROW_SURFACE_EVEN;
}

function activityLogCellStackClass({
	stackIndex,
	rowSurface,
	isRowHoverable,
}: {
	stackIndex: number;
	rowSurface: string;
	isRowHoverable: boolean;
}): string {
	return cn(
		"relative overflow-hidden",
		columnStackZClass(stackIndex),
		rowSurface,
		isRowHoverable && ACTIVITY_LOG_ROW_SURFACE_HOVER,
	);
}

const HEADER_KEY_BY_COLUMN: Record<ActivityLogDataColumnId, keyof ActivityLogTableHeaders> = {
	timestamp: "timestamp",
	action: "action",
	category: "category",
	status: "status",
	target: "target",
	duration: "duration",
};

function getColumnNthChildIndex(
	columnOrder: ActivityLogDataColumnId[],
	columnId: ActivityLogDataColumnId,
): number {
	const index = columnOrder.indexOf(columnId);
	return index < 0 ? -1 : index + 2;
}

function measureActivityLogColumnNaturalWidth(
	table: HTMLTableElement,
	columnId: ActivityLogDataColumnId,
	columnOrder: ActivityLogDataColumnId[],
): number {
	const nthChild = getColumnNthChildIndex(columnOrder, columnId);
	if (nthChild < 0) {
		return ACTIVITY_LOG_MIN_COLUMN_WIDTH;
	}

	const measurer = document.createElement("div");
	measurer.style.position = "absolute";
	measurer.style.visibility = "hidden";
	measurer.style.pointerEvents = "none";
	measurer.style.left = "-9999px";
	measurer.style.top = "0";
	measurer.style.display = "inline-block";
	measurer.style.whiteSpace = "nowrap";
	document.body.appendChild(measurer);

	const tableStyles = window.getComputedStyle(table);
	measurer.style.font = tableStyles.font;
	measurer.style.letterSpacing = tableStyles.letterSpacing;

	let maxWidth = ACTIVITY_LOG_MIN_COLUMN_WIDTH;
	const cells = table.querySelectorAll(
		`thead tr > :nth-child(${nthChild}), tbody tr > :nth-child(${nthChild})`,
	);

	for (const cell of cells) {
		if (!(cell instanceof HTMLElement)) {
			continue;
		}

		measurer.replaceChildren(cell.cloneNode(true));
		const clone = measurer.firstElementChild;
		if (!(clone instanceof HTMLElement)) {
			continue;
		}

		clone.style.width = "auto";
		clone.style.maxWidth = "none";
		clone.style.overflow = "visible";
		clone.style.whiteSpace = "nowrap";

		for (const truncated of clone.querySelectorAll(".truncate")) {
			if (truncated instanceof HTMLElement) {
				truncated.classList.remove("truncate");
				truncated.style.overflow = "visible";
				truncated.style.textOverflow = "clip";
				truncated.style.whiteSpace = "nowrap";
			}
		}

		maxWidth = Math.max(maxWidth, measurer.getBoundingClientRect().width);
	}

	document.body.removeChild(measurer);
	return Math.ceil(maxWidth) + ACTIVITY_LOG_COLUMN_AUTOSIZE_PADDING_PX;
}

/** Distance from the column's right edge to the resize guide and hit target. */
const RESIZE_GUIDE_OFFSET_PX = 4;

function measureColumnResizeGuideLeft(
	scrollContainer: HTMLDivElement,
	table: HTMLTableElement,
	columnId: ActivityLogDataColumnId,
): number | null {
	const headerCell = table.querySelector(`thead th[data-activity-log-column="${columnId}"]`);
	if (!(headerCell instanceof HTMLElement)) {
		return null;
	}

	const containerRect = scrollContainer.getBoundingClientRect();
	const cellRect = headerCell.getBoundingClientRect();
	return cellRect.right - containerRect.left + scrollContainer.scrollLeft - RESIZE_GUIDE_OFFSET_PX;
}

type ColumnResizeRailPosition = {
	columnId: ActivityLogDataColumnId;
	left: number;
};

function measureColumnResizeRailPositions(
	scrollContainer: HTMLDivElement,
	table: HTMLTableElement,
	columnOrder: ActivityLogDataColumnId[],
): ColumnResizeRailPosition[] {
	return columnOrder
		.map((columnId) => {
			const left = measureColumnResizeGuideLeft(scrollContainer, table, columnId);
			return left == null ? null : { columnId, left };
		})
		.filter((rail): rail is ColumnResizeRailPosition => rail != null);
}

function ActivityLogColumnResizeRail({
	columnId,
	left,
	onResize,
	onAutoSize,
	onGuideEnter,
	onGuideLeave,
	onResizeStart,
	onResizeEnd,
	getResizeStartWidth,
}: {
	columnId: ActivityLogDataColumnId;
	left: number;
	onResize: (columnId: ActivityLogDataColumnId, nextWidth: number) => void;
	onAutoSize: (columnId: ActivityLogDataColumnId) => void;
	onGuideEnter?: (columnId: ActivityLogDataColumnId) => void;
	onGuideLeave?: (columnId: ActivityLogDataColumnId) => void;
	onResizeStart?: (columnId: ActivityLogDataColumnId) => void;
	onResizeEnd?: () => void;
	getResizeStartWidth: (columnId: ActivityLogDataColumnId) => number;
}) {
	const resizeStateRef = useRef<{ startX: number; startWidth: number } | null>(null);
	const hoveredRef = useRef(false);

	const handleMouseDown = useCallback(
		(event: ReactMouseEvent<HTMLDivElement>) => {
			event.preventDefault();
			event.stopPropagation();

			onResizeStart?.(columnId);
			resizeStateRef.current = {
				startX: event.clientX,
				startWidth: getResizeStartWidth(columnId),
			};

			const handleMouseMove = (moveEvent: MouseEvent) => {
				const state = resizeStateRef.current;
				if (!state) {
					return;
				}
				onResize(columnId, state.startWidth + (moveEvent.clientX - state.startX));
			};

			const handleMouseUp = () => {
				resizeStateRef.current = null;
				onResizeEnd?.();
				if (!hoveredRef.current) {
					onGuideLeave?.(columnId);
				}
				window.removeEventListener("mousemove", handleMouseMove);
				window.removeEventListener("mouseup", handleMouseUp);
			};

			window.addEventListener("mousemove", handleMouseMove);
			window.addEventListener("mouseup", handleMouseUp);
		},
		[columnId, getResizeStartWidth, onGuideLeave, onResize, onResizeEnd, onResizeStart],
	);

	const handleDoubleClick = useCallback(
		(event: ReactMouseEvent<HTMLDivElement>) => {
			event.preventDefault();
			event.stopPropagation();
			onAutoSize(columnId);
		},
		[columnId, onAutoSize],
	);

	return (
		<div
			role="separator"
			aria-orientation="vertical"
			aria-label="Resize column"
			className="absolute top-0 bottom-0 z-[30] w-2 -translate-x-1/2 cursor-col-resize touch-none select-none"
			style={{ left }}
			onMouseEnter={() => {
				hoveredRef.current = true;
				onGuideEnter?.(columnId);
			}}
			onMouseLeave={() => {
				hoveredRef.current = false;
				onGuideLeave?.(columnId);
			}}
			onMouseDown={handleMouseDown}
			onDoubleClick={handleDoubleClick}
		/>
	);
}

function ActivityLogTableScrollContainer({
	className,
	scrollContainerRef,
	resizeGuide,
	columnResizeRails,
	interactiveColumns = false,
	onColumnResize,
	onColumnAutoSize,
	onResizeGuideEnter,
	onResizeGuideLeave,
	onResizeStart,
	onResizeEnd,
	getColumnResizeStartWidth,
	children,
}: {
	className?: string;
	scrollContainerRef: RefObject<HTMLDivElement | null>;
	resizeGuide: { columnId: ActivityLogDataColumnId; left: number } | null;
	columnResizeRails: ColumnResizeRailPosition[];
	interactiveColumns?: boolean;
	onColumnResize?: (columnId: ActivityLogDataColumnId, nextWidth: number) => void;
	onColumnAutoSize?: (columnId: ActivityLogDataColumnId) => void;
	onResizeGuideEnter?: (columnId: ActivityLogDataColumnId) => void;
	onResizeGuideLeave?: (columnId: ActivityLogDataColumnId) => void;
	onResizeStart?: (columnId: ActivityLogDataColumnId) => void;
	onResizeEnd?: () => void;
	getColumnResizeStartWidth?: (columnId: ActivityLogDataColumnId) => number;
	children: ReactNode;
}) {
	const canResize =
		interactiveColumns &&
		onColumnResize &&
		onColumnAutoSize &&
		onResizeGuideEnter &&
		onResizeGuideLeave &&
		onResizeStart &&
		onResizeEnd &&
		getColumnResizeStartWidth;

	return (
		<div ref={scrollContainerRef} className={cn("relative", className)}>
			{resizeGuide ? (
				<div
					className={cn(
						"pointer-events-none absolute top-0 bottom-0 z-[3] w-px -translate-x-1/2",
						ACTIVITY_LOG_TABLE_RULE_CLASSNAME,
					)}
					style={{ left: resizeGuide.left }}
					aria-hidden
				/>
			) : null}
			{canResize
				? columnResizeRails.map(({ columnId, left }) => (
					<ActivityLogColumnResizeRail
						key={columnId}
						columnId={columnId}
						left={left}
						onResize={onColumnResize}
						onAutoSize={onColumnAutoSize}
						onGuideEnter={onResizeGuideEnter}
						onGuideLeave={onResizeGuideLeave}
						onResizeStart={onResizeStart}
						onResizeEnd={onResizeEnd}
						getResizeStartWidth={getColumnResizeStartWidth}
					/>
				))
				: null}
			{children}
		</div>
	);
}

type ActivityLogTableSizeStyles = {
	table: string;
	thExpand: string;
	thCell: string;
	tdExpand: string;
	tdCell: string;
	tdDuration: string;
	detailCell: string;
	detailContent: string;
	expandIconBox: string;
	expandIcon: string;
	skeletonBar: string;
	skeletonBadge: string;
};

const SIZE_STYLES: Record<ActivityLogTableSize, ActivityLogTableSizeStyles> = {
	big: {
		table: "text-sm",
		thExpand: "py-2",
		thCell: "py-2 pr-4",
		tdExpand: "py-3",
		tdCell: "py-3 pr-4",
		tdDuration: "py-3 pl-2 pr-4",
		detailCell: "py-4 pr-4",
		detailContent: "text-sm",
		expandIconBox: "h-8 w-8",
		expandIcon: "h-4 w-4",
		skeletonBar: "h-4",
		skeletonBadge: "h-5",
	},
	middle: {
		table: "text-sm",
		thExpand: "py-1.5",
		thCell: "py-1.5 pr-4",
		tdExpand: "py-2",
		tdCell: "py-2 pr-4",
		tdDuration: "py-2 pl-2 pr-4",
		detailCell: "py-3 pr-4",
		detailContent: "text-sm",
		expandIconBox: "h-7 w-7",
		expandIcon: "h-4 w-4",
		skeletonBar: "h-3.5",
		skeletonBadge: "h-4",
	},
	small: {
		table: "text-xs",
		thExpand: "py-1",
		thCell: "py-1 pr-3",
		tdExpand: "py-1.5",
		tdCell: "py-1.5 pr-3",
		tdDuration: "py-1.5 pl-1.5 pr-3",
		detailCell: "py-2 pr-3",
		detailContent: "text-xs",
		expandIconBox: "h-6 w-6",
		expandIcon: "h-3.5 w-3.5",
		skeletonBar: "h-3",
		skeletonBadge: "h-3.5",
	},
};

type ActivityLogTableProps = {
	rows: ActivityLogRow[];
	headers: ActivityLogTableHeaders;
	emptyState: ReactNode;
	size?: ActivityLogTableSize;
	isLoading?: boolean;
	loadingRowCount?: number;
	expandedRowKey?: string | null;
	onExpandedRowKeyChange?: (key: string | null) => void;
	headerSurfaceClassName?: string;
	/** Constrain table scroll area to fill a flex parent (e.g. inspector bottom panel). */
	fillContainer?: boolean;
	/**
	 * @deprecated Use `interactiveColumns` for inspector-style layouts. Kept for callers that
	 * still pass the flag; column widths are always synchronized via a single table layout.
	 */
	autoSizeDataColumns?: boolean;
	/** Enable drag-to-resize column widths and drag-to-reorder headers. */
	interactiveColumns?: boolean;
	/** Persist interactive column layout to localStorage when set. */
	tableLayoutStorageKey?: string;
	/** Open a detail surface instead of inline row expansion. */
	onRowClick?: (row: ActivityLogRow) => void;
	className?: string;
};

function ActivityLogTableColgroup({
	columnOrder,
	columnWidths,
}: {
	columnOrder: ActivityLogDataColumnId[];
	columnWidths: Record<ActivityLogDataColumnId, number | null>;
}) {
	return (
		<colgroup>
			<col style={{ width: EXPAND_COL_WIDTH_PX }} />
			{columnOrder.map((columnId) => {
				const width = columnWidths[columnId];
				return (
					<col
						key={columnId}
						style={width == null ? undefined : { width }}
					/>
				);
			})}
		</colgroup>
	);
}

function ActivityLogColumnDragHandle({
	columnId,
	isDragging = false,
	onColumnDragStart,
	onColumnDragEnd,
}: {
	columnId: ActivityLogDataColumnId;
	isDragging?: boolean;
	onColumnDragStart?: (columnId: ActivityLogDataColumnId) => void;
	onColumnDragEnd?: () => void;
}) {
	return (
		<span
			draggable
			role="button"
			tabIndex={-1}
			aria-label="Reorder column"
			className={cn(
				"drag-handle absolute left-0 top-1/2 z-[1] flex h-4 w-3 -translate-y-1/2",
				"cursor-grab touch-none select-none active:cursor-grabbing",
			)}
			onDragStart={(event) => {
				event.stopPropagation();
				event.dataTransfer.effectAllowed = "move";
				event.dataTransfer.setData("text/plain", columnId);
				onColumnDragStart?.(columnId);
			}}
			onDragEnd={(event) => {
				event.stopPropagation();
				onColumnDragEnd?.();
			}}
		>
			<GripVertical
				className={cn(
					"drag-grip h-3 w-3 text-muted-foreground opacity-0 transition-opacity",
					isDragging && "opacity-100",
				)}
				aria-hidden
			/>
		</span>
	);
}

function ActivityLogRowSkeleton({
	styles,
	columnOrder,
	rowIndex,
}: {
	styles: ActivityLogTableSizeStyles;
	columnOrder: ActivityLogDataColumnId[];
	rowIndex: number;
}) {
	const expandColTdClass = cn(EXPAND_COL_BASE, styles.tdExpand, "align-middle");
	const cellClass = cn(styles.tdCell, "align-middle");
	const durationClass = cn(styles.tdDuration, "align-middle text-right tabular-nums");
	const rowSurface = activityLogRowSurface(rowIndex);

	return (
		<tr className="border-b align-middle">
			<td
				className={cn(
					expandColTdClass,
					activityLogCellStackClass({
						stackIndex: 1,
						rowSurface,
						isRowHoverable: false,
					}),
				)}
			>
				<span
					className={cn(
						"inline-flex shrink-0 items-center justify-center",
						styles.expandIconBox,
					)}
				>
					<div
						className={cn(
							styles.skeletonBar,
							"w-4 animate-pulse rounded bg-slate-200 dark:bg-slate-800",
						)}
					/>
				</span>
			</td>
			{columnOrder.map((columnId, columnIndex) => {
				const stackClass = activityLogCellStackClass({
					stackIndex: columnIndex + 2,
					rowSurface,
					isRowHoverable: false,
				});

				if (columnId === "duration") {
					return (
						<td key={columnId} className={cn(durationClass, stackClass)}>
							<div
								className={cn(
									styles.skeletonBar,
									"ml-auto w-12 animate-pulse rounded bg-slate-200 dark:bg-slate-800",
								)}
							/>
						</td>
					);
				}

				if (columnId === "status") {
					return (
						<td key={columnId} className={cn(cellClass, stackClass)}>
							<div
								className={cn(
									styles.skeletonBadge,
									"w-16 animate-pulse rounded bg-slate-200 dark:bg-slate-800",
								)}
							/>
						</td>
					);
				}

				if (columnId === "target") {
					return (
						<td key={columnId} className={cn("min-w-0", cellClass, stackClass)}>
							<div
								className={cn(
									styles.skeletonBar,
									"w-full max-w-full animate-pulse rounded bg-slate-200 dark:bg-slate-800",
								)}
							/>
						</td>
					);
				}

				return (
					<td key={columnId} className={cn(cellClass, stackClass)}>
						<div
							className={cn(
								styles.skeletonBar,
								columnId === "timestamp" ? "w-32" : "w-20",
								"animate-pulse rounded bg-slate-200 dark:bg-slate-800",
							)}
						/>
					</td>
				);
			})}
		</tr>
	);
}

function ActivityLogTableHead({
	headers,
	styles,
	headerSurfaceClassName = "bg-white dark:bg-slate-900",
	sticky = true,
	columnOrder,
	interactiveColumns = false,
	draggingColumnId,
	dropTargetColumnId,
	onColumnDragStart,
	onColumnDragOver,
	onColumnDrop,
	onColumnDragEnd,
}: {
	headers: ActivityLogTableHeaders;
	styles: ActivityLogTableSizeStyles;
	headerSurfaceClassName?: string;
	sticky?: boolean;
	columnOrder: ActivityLogDataColumnId[];
	interactiveColumns?: boolean;
	draggingColumnId: ActivityLogDataColumnId | null;
	dropTargetColumnId: ActivityLogDataColumnId | null;
	onColumnDragStart?: (columnId: ActivityLogDataColumnId) => void;
	onColumnDragOver?: (columnId: ActivityLogDataColumnId) => void;
	onColumnDrop?: (columnId: ActivityLogDataColumnId) => void;
	onColumnDragEnd?: () => void;
}) {
	const expandColThClass = cn(EXPAND_COL_BASE, styles.thExpand, "align-middle font-normal");

	return (
		<thead className={cn(sticky && "sticky top-0 z-20", headerSurfaceClassName)}>
			<tr className="border-b border-slate-200 text-left text-muted-foreground dark:border-slate-700">
				<th
					scope="col"
					className={cn(
						expandColThClass,
						"relative overflow-hidden",
						headerSurfaceClassName,
						columnStackZClass(1),
					)}
				>
					<span className="sr-only">{headers.expandColumn}</span>
				</th>
				{columnOrder.map((columnId, columnIndex) => {
					const isDuration = columnId === "duration";
					const isTarget = columnId === "target";
					const isDropTarget = dropTargetColumnId === columnId;
					const isDragging = draggingColumnId === columnId;
					const columnStackIndex = columnIndex + 2;

					return (
						<th
							key={columnId}
							scope="col"
							data-activity-log-column={columnId}
							onDragOver={
								interactiveColumns
									? (event) => {
										event.preventDefault();
										event.dataTransfer.dropEffect = "move";
										onColumnDragOver?.(columnId);
									}
									: undefined
							}
							onDrop={
								interactiveColumns
									? (event) => {
										event.preventDefault();
										onColumnDrop?.(columnId);
									}
									: undefined
							}
							className={cn(
								"relative overflow-hidden align-middle font-normal",
								headerSurfaceClassName,
								columnStackZClass(columnStackIndex),
								isDuration ? "text-right" : "text-left",
								isTarget ? "min-w-0" : "whitespace-nowrap",
								styles.thCell,
								isDragging && "opacity-50",
								isDropTarget && "bg-primary/10",
								interactiveColumns && [
									"[&:has(.drag-handle:hover)_.header-label]:pl-4",
									"[&:has(.drag-handle:hover)_.drag-grip]:opacity-100",
									isDragging && "[&_.header-label]:pl-4 [&_.drag-grip]:opacity-100",
								],
							)}
						>
							{interactiveColumns ? (
								<ActivityLogColumnDragHandle
									columnId={columnId}
									isDragging={isDragging}
									onColumnDragStart={onColumnDragStart}
									onColumnDragEnd={onColumnDragEnd}
								/>
							) : null}
							<span
								className={cn(
									"header-label block min-w-0 transition-[padding]",
									isTarget && "truncate",
									isDuration && "w-full",
								)}
							>
								{headers[HEADER_KEY_BY_COLUMN[columnId]]}
							</span>
						</th>
					);
				})}
			</tr>
		</thead>
	);
}

type ActivityLogTableRowsProps = {
	rows: ActivityLogRow[];
	styles: ActivityLogTableSizeStyles;
	expandColTdClass: string;
	cellClass: string;
	durationClass: string;
	expandedRowKey: string | null;
	onRowToggle: (event: ReactMouseEvent<HTMLTableRowElement>, row: ActivityLogRow) => void;
	columnOrder: ActivityLogDataColumnId[];
	onRowClick?: (row: ActivityLogRow) => void;
};

function renderDataCell(
	columnId: ActivityLogDataColumnId,
	row: ActivityLogRow,
	cellClass: string,
	durationClass: string,
	cellStackClass: string,
): ReactNode {
	switch (columnId) {
		case "timestamp":
			return (
				<td key={columnId} className={cn("whitespace-nowrap", cellClass, cellStackClass)}>
					{formatLocalDateTime(row.timestampMs)}
				</td>
			);
		case "action":
			return (
				<td key={columnId} className={cn("min-w-0", cellClass, cellStackClass)}>
					<div className="truncate">{row.action}</div>
				</td>
			);
		case "category":
			return (
				<td key={columnId} className={cn("min-w-0", cellClass, cellStackClass)}>
					<div className="truncate">{row.category}</div>
				</td>
			);
		case "status":
			return (
				<td key={columnId} className={cn("whitespace-nowrap", cellClass, cellStackClass)}>
					{row.status}
				</td>
			);
		case "target":
			return (
				<td key={columnId} className={cn("min-w-0", cellClass, cellStackClass)}>
					<div className="truncate">{row.target}</div>
				</td>
			);
		case "duration":
			return (
				<td key={columnId} className={cn(durationClass, cellStackClass)}>
					{row.durationMs != null ? String(row.durationMs) : "—"}
				</td>
			);
	}
}

function ActivityLogTableRows({
	rows,
	styles,
	expandColTdClass,
	cellClass,
	durationClass,
	expandedRowKey,
	onRowToggle,
	columnOrder,
	onRowClick,
}: ActivityLogTableRowsProps) {
	return (
		<tbody>
			{rows.map((row, rowIndex) => {
				const expanded = expandedRowKey === row.key;
				const expandable =
					!onRowClick && row.expandable !== false && Boolean(row.details);
				const clickable = Boolean(onRowClick && row.eventId) || expandable;
				const rowSurface = activityLogRowSurface(rowIndex);
				const isRowHoverable = clickable;
				return (
					<Fragment key={row.key}>
						<tr
							className={cn(
								ACTIVITY_LOG_ROW_CLASSNAME,
								clickable && ACTIVITY_LOG_ROW_EXPANDABLE_CLASSNAME,
								expanded && "opacity-100",
							)}
							onClick={clickable ? (event) => onRowToggle(event, row) : undefined}
						>
							<td
								className={cn(
									expandColTdClass,
									"text-muted-foreground",
									activityLogCellStackClass({
										stackIndex: 1,
										rowSurface,
										isRowHoverable,
									}),
								)}
							>
								{expandable ? (
									<span
										className={cn(
											"inline-flex shrink-0 items-center justify-center",
											styles.expandIconBox,
										)}
									>
										{expanded ? (
											<ChevronDown
												className={cn(styles.expandIcon, "shrink-0")}
												aria-hidden
											/>
										) : (
											<ChevronRight
												className={cn(styles.expandIcon, "shrink-0")}
												aria-hidden
											/>
										)}
									</span>
								) : null}
							</td>
							{columnOrder.map((columnId, columnIndex) =>
								renderDataCell(
									columnId,
									row,
									cellClass,
									durationClass,
									activityLogCellStackClass({
										stackIndex: columnIndex + 2,
										rowSurface,
										isRowHoverable,
									}),
								),
							)}
						</tr>
						{expanded && row.details ? (
							<tr className="border-b last:border-0">
								<td className={EXPAND_COL_SPACER_CLASS} />
								<td
									colSpan={columnOrder.length}
									className={cn(
										styles.detailCell,
										styles.detailContent,
										"relative z-[8] overflow-hidden bg-slate-100/80 pl-0 align-top dark:bg-slate-900/70",
									)}
								>
									{row.details}
								</td>
							</tr>
						) : null}
					</Fragment>
				);
			})}
		</tbody>
	);
}

function ActivityLogTableSkeletonBody({
	loadingRowCount,
	styles,
	columnOrder,
}: {
	loadingRowCount: number;
	styles: ActivityLogTableSizeStyles;
	columnOrder: ActivityLogDataColumnId[];
}) {
	return (
		<tbody>
			{Array.from({ length: loadingRowCount }).map((_, index) => (
				<ActivityLogRowSkeleton
					key={index}
					styles={styles}
					columnOrder={columnOrder}
					rowIndex={index}
				/>
			))}
		</tbody>
	);
}

function useActivityLogTableLayoutState({
	interactiveColumns,
	tableLayoutStorageKey,
}: {
	interactiveColumns: boolean;
	tableLayoutStorageKey?: string;
}) {
	const [layout, setLayout] = useState<ActivityLogTableLayout>(() =>
		interactiveColumns && tableLayoutStorageKey
			? loadActivityLogTableLayout(tableLayoutStorageKey)
			: {
				...DEFAULT_ACTIVITY_LOG_TABLE_LAYOUT,
				columnOrder: [...ACTIVITY_LOG_DATA_COLUMN_IDS],
			},
	);
	const [draggingColumnId, setDraggingColumnId] = useState<ActivityLogDataColumnId | null>(null);
	const [dropTargetColumnId, setDropTargetColumnId] = useState<ActivityLogDataColumnId | null>(
		null,
	);

	useEffect(() => {
		if (!interactiveColumns || !tableLayoutStorageKey) {
			return;
		}
		saveActivityLogTableLayout(tableLayoutStorageKey, layout);
	}, [interactiveColumns, layout, tableLayoutStorageKey]);

	const handleColumnResize = useCallback(
		(columnId: ActivityLogDataColumnId, nextWidth: number) => {
			if (!interactiveColumns) {
				return;
			}
			setLayout((current) => resizeActivityLogColumnWidth(current, columnId, nextWidth));
		},
		[interactiveColumns],
	);

	const handleColumnDragStart = useCallback((columnId: ActivityLogDataColumnId) => {
		setDraggingColumnId(columnId);
	}, []);

	const handleColumnDragOver = useCallback((columnId: ActivityLogDataColumnId) => {
		setDropTargetColumnId(columnId);
	}, []);

	const handleColumnDrop = useCallback(
		(targetColumnId: ActivityLogDataColumnId) => {
			if (!draggingColumnId) {
				return;
			}
			setLayout((current) =>
				reorderActivityLogColumns(current, draggingColumnId, targetColumnId),
			);
			setDraggingColumnId(null);
			setDropTargetColumnId(null);
		},
		[draggingColumnId],
	);

	const handleColumnDragEnd = useCallback(() => {
		setDraggingColumnId(null);
		setDropTargetColumnId(null);
	}, []);

	const handleColumnAutoSize = useCallback(
		(columnId: ActivityLogDataColumnId, measuredWidth: number) => {
			if (!interactiveColumns) {
				return;
			}
			setLayout((current) => setActivityLogColumnWidth(current, columnId, measuredWidth));
		},
		[interactiveColumns],
	);

	return {
		layout,
		draggingColumnId,
		dropTargetColumnId,
		handleColumnResize,
		handleColumnAutoSize,
		handleColumnDragStart,
		handleColumnDragOver,
		handleColumnDrop,
		handleColumnDragEnd,
	};
}

export function ActivityLogTable({
	rows,
	headers,
	emptyState,
	size = "big",
	isLoading = false,
	loadingRowCount = 8,
	expandedRowKey = null,
	onExpandedRowKeyChange,
	headerSurfaceClassName,
	fillContainer = false,
	interactiveColumns = false,
	tableLayoutStorageKey = INSPECTOR_ACTIVITY_TABLE_LAYOUT_STORAGE_KEY,
	onRowClick,
	className,
}: ActivityLogTableProps) {
	const styles = SIZE_STYLES[size];
	const tableRef = useRef<HTMLTableElement>(null);
	const scrollContainerRef = useRef<HTMLDivElement>(null);
	const resizingColumnRef = useRef<ActivityLogDataColumnId | null>(null);
	const [resizeGuide, setResizeGuide] = useState<{
		columnId: ActivityLogDataColumnId;
		left: number;
	} | null>(null);
	const [columnResizeRails, setColumnResizeRails] = useState<ColumnResizeRailPosition[]>([]);
	const expandColTdClass = cn(EXPAND_COL_BASE, styles.tdExpand, "align-middle");
	const cellClass = cn(styles.tdCell, "align-middle");
	const durationClass = cn(styles.tdDuration, "align-middle text-right tabular-nums");

	const {
		layout,
		draggingColumnId,
		dropTargetColumnId,
		handleColumnResize,
		handleColumnAutoSize,
		handleColumnDragStart,
		handleColumnDragOver,
		handleColumnDrop,
		handleColumnDragEnd,
	} = useActivityLogTableLayoutState({
		interactiveColumns,
		tableLayoutStorageKey: interactiveColumns ? tableLayoutStorageKey : undefined,
	});

	const handleColumnAutoSizeRequest = useCallback(
		(columnId: ActivityLogDataColumnId) => {
			if (!tableRef.current) {
				return;
			}
			const measuredWidth = measureActivityLogColumnNaturalWidth(
				tableRef.current,
				columnId,
				layout.columnOrder,
			);
			handleColumnAutoSize(columnId, measuredWidth);
		},
		[handleColumnAutoSize, layout.columnOrder],
	);

	const syncColumnResizeRails = useCallback(() => {
		const container = scrollContainerRef.current;
		const table = tableRef.current;
		if (!container || !table || !interactiveColumns) {
			setColumnResizeRails([]);
			return;
		}

		const rails = measureColumnResizeRailPositions(container, table, layout.columnOrder);
		setColumnResizeRails(rails);
		setResizeGuide((current) => {
			if (!current) {
				return null;
			}
			const rail = rails.find((entry) => entry.columnId === current.columnId);
			return rail ? { columnId: current.columnId, left: rail.left } : null;
		});
	}, [interactiveColumns, layout.columnOrder]);

	const syncResizeGuide = useCallback(
		(columnId: ActivityLogDataColumnId) => {
			const container = scrollContainerRef.current;
			const table = tableRef.current;
			if (!container || !table || !interactiveColumns) {
				return;
			}
			const left = measureColumnResizeGuideLeft(container, table, columnId);
			if (left == null) {
				return;
			}
			setResizeGuide({ columnId, left });
			syncColumnResizeRails();
		},
		[interactiveColumns, syncColumnResizeRails],
	);

	const getColumnResizeStartWidth = useCallback((columnId: ActivityLogDataColumnId) => {
		const table = tableRef.current;
		if (!table) {
			return ACTIVITY_LOG_MIN_COLUMN_WIDTH;
		}
		const headerCell = table.querySelector(`thead th[data-activity-log-column="${columnId}"]`);
		if (!(headerCell instanceof HTMLElement)) {
			return ACTIVITY_LOG_MIN_COLUMN_WIDTH;
		}
		return headerCell.getBoundingClientRect().width;
	}, []);

	useLayoutEffect(() => {
		syncColumnResizeRails();
	}, [
		syncColumnResizeRails,
		layout.columnWidths,
		rows.length,
		isLoading,
		loadingRowCount,
		expandedRowKey,
	]);

	useEffect(() => {
		if (!interactiveColumns) {
			return;
		}

		const syncActiveRails = () => {
			syncColumnResizeRails();
		};

		window.addEventListener("resize", syncActiveRails);
		const container = scrollContainerRef.current;
		container?.addEventListener("scroll", syncActiveRails);

		return () => {
			window.removeEventListener("resize", syncActiveRails);
			container?.removeEventListener("scroll", syncActiveRails);
		};
	}, [interactiveColumns, syncColumnResizeRails]);

	const handleResizeGuideEnter = useCallback(
		(columnId: ActivityLogDataColumnId) => {
			syncResizeGuide(columnId);
		},
		[syncResizeGuide],
	);

	const handleResizeGuideLeave = useCallback((columnId: ActivityLogDataColumnId) => {
		if (resizingColumnRef.current === columnId) {
			return;
		}
		setResizeGuide((current) => (current?.columnId === columnId ? null : current));
	}, []);

	const handleResizeStart = useCallback(
		(columnId: ActivityLogDataColumnId) => {
			resizingColumnRef.current = columnId;
			syncResizeGuide(columnId);
		},
		[syncResizeGuide],
	);

	const handleResizeEnd = useCallback(() => {
		resizingColumnRef.current = null;
	}, []);

	const handleColumnResizeWithGuide = useCallback(
		(columnId: ActivityLogDataColumnId, nextWidth: number) => {
			handleColumnResize(columnId, nextWidth);
			requestAnimationFrame(() => {
				syncResizeGuide(columnId);
			});
		},
		[handleColumnResize, syncResizeGuide],
	);

	useEffect(() => {
		if (!resizeGuide) {
			return;
		}

		const syncActiveGuide = () => {
			syncResizeGuide(resizeGuide.columnId);
		};

		window.addEventListener("resize", syncActiveGuide);

		return () => {
			window.removeEventListener("resize", syncActiveGuide);
		};
	}, [resizeGuide, syncResizeGuide]);

	const handleColumnAutoSizeRequestWithGuide = useCallback(
		(columnId: ActivityLogDataColumnId) => {
			handleColumnAutoSizeRequest(columnId);
			requestAnimationFrame(() => {
				syncResizeGuide(columnId);
			});
		},
		[handleColumnAutoSizeRequest, syncResizeGuide],
	);

	const handleRowToggle = useCallback(
		(event: ReactMouseEvent<HTMLTableRowElement>, row: ActivityLogRow) => {
			const target = event.target as HTMLElement;
			if (target.closest("a, button, summary")) {
				return;
			}
			if (onRowClick && row.eventId) {
				onRowClick(row);
				return;
			}
			if (!onExpandedRowKeyChange || row.expandable === false) {
				return;
			}
			onExpandedRowKeyChange(expandedRowKey === row.key ? null : row.key);
		},
		[expandedRowKey, onExpandedRowKeyChange, onRowClick],
	);

	const scrollContainerClassName = cn(
		fillContainer ? "min-h-0 flex-1 overflow-auto overscroll-contain" : "overflow-y-auto overscroll-contain",
		className,
	);

	const rowProps = {
		rows,
		styles,
		expandColTdClass,
		cellClass,
		durationClass,
		expandedRowKey,
		onRowToggle: handleRowToggle,
		columnOrder: layout.columnOrder,
		onRowClick,
	};

	const tableClassName = cn("w-full table-fixed", styles.table);

	const headProps = {
		headers,
		styles,
		headerSurfaceClassName,
		sticky: true,
		columnOrder: layout.columnOrder,
		interactiveColumns,
		draggingColumnId,
		dropTargetColumnId,
		onColumnDragStart: interactiveColumns ? handleColumnDragStart : undefined,
		onColumnDragOver: interactiveColumns ? handleColumnDragOver : undefined,
		onColumnDrop: interactiveColumns ? handleColumnDrop : undefined,
		onColumnDragEnd: interactiveColumns ? handleColumnDragEnd : undefined,
	};

	const scrollContainerProps = {
		scrollContainerRef,
		resizeGuide,
		columnResizeRails,
		interactiveColumns,
		onColumnResize: interactiveColumns ? handleColumnResizeWithGuide : undefined,
		onColumnAutoSize: interactiveColumns ? handleColumnAutoSizeRequestWithGuide : undefined,
		onResizeGuideEnter: interactiveColumns ? handleResizeGuideEnter : undefined,
		onResizeGuideLeave: interactiveColumns ? handleResizeGuideLeave : undefined,
		onResizeStart: interactiveColumns ? handleResizeStart : undefined,
		onResizeEnd: interactiveColumns ? handleResizeEnd : undefined,
		getColumnResizeStartWidth,
	};

	const colgroup = (
		<ActivityLogTableColgroup
			columnOrder={layout.columnOrder}
			columnWidths={layout.columnWidths}
		/>
	);

	if (isLoading && rows.length === 0) {
		return (
			<ActivityLogTableScrollContainer
				className={scrollContainerClassName}
				{...scrollContainerProps}
			>
				<table ref={tableRef} className={tableClassName}>
					{colgroup}
					<ActivityLogTableHead {...headProps} />
					<ActivityLogTableSkeletonBody
						loadingRowCount={loadingRowCount}
						styles={styles}
						columnOrder={layout.columnOrder}
					/>
				</table>
			</ActivityLogTableScrollContainer>
		);
	}

	if (rows.length === 0) {
		return (
			<div
				className={cn(
					fillContainer && "flex min-h-0 flex-1 flex-col overflow-hidden",
					!fillContainer && "flex flex-col items-center justify-center",
					scrollContainerClassName,
				)}
			>
				{emptyState}
			</div>
		);
	}

	return (
		<ActivityLogTableScrollContainer
			className={scrollContainerClassName}
			{...scrollContainerProps}
		>
			<table ref={tableRef} className={tableClassName}>
				{colgroup}
				<ActivityLogTableHead {...headProps} />
				<ActivityLogTableRows {...rowProps} />
			</table>
		</ActivityLogTableScrollContainer>
	);
}
