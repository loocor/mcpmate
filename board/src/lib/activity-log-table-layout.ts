export type ActivityLogDataColumnId =
	| "timestamp"
	| "action"
	| "category"
	| "status"
	| "target"
	| "duration";

export const ACTIVITY_LOG_DATA_COLUMN_IDS: ActivityLogDataColumnId[] = [
	"timestamp",
	"action",
	"category",
	"status",
	"target",
	"duration",
];

export type ActivityLogTableLayout = {
	columnOrder: ActivityLogDataColumnId[];
	columnWidths: Record<ActivityLogDataColumnId, number | null>;
};

export const DEFAULT_ACTIVITY_LOG_COLUMN_WIDTHS: Record<
	ActivityLogDataColumnId,
	number | null
> = {
	timestamp: 176,
	action: 128,
	category: 104,
	status: 92,
	target: null,
	duration: 96,
};

function createDefaultActivityLogTableLayout(): ActivityLogTableLayout {
	return {
		columnOrder: [...ACTIVITY_LOG_DATA_COLUMN_IDS],
		columnWidths: { ...DEFAULT_ACTIVITY_LOG_COLUMN_WIDTHS },
	};
}

export const DEFAULT_ACTIVITY_LOG_TABLE_LAYOUT: ActivityLogTableLayout =
	createDefaultActivityLogTableLayout();

export const ACTIVITY_LOG_MIN_COLUMN_WIDTH = 56;

export const INSPECTOR_ACTIVITY_TABLE_LAYOUT_STORAGE_KEY =
	"mcpmate:inspector:activity-table-layout";

export const AUDIT_ACTIVITY_TABLE_LAYOUT_STORAGE_KEY =
	"mcpmate:audit:activity-table-layout";

export const ACTIVITY_LOG_COLUMN_AUTOSIZE_PADDING_PX = 16;

function isActivityLogDataColumnId(value: unknown): value is ActivityLogDataColumnId {
	return (
		typeof value === "string" &&
		(ACTIVITY_LOG_DATA_COLUMN_IDS as string[]).includes(value)
	);
}

function normalizeColumnOrder(order: unknown): ActivityLogDataColumnId[] {
	if (!Array.isArray(order)) {
		return [...ACTIVITY_LOG_DATA_COLUMN_IDS];
	}

	const seen = new Set<ActivityLogDataColumnId>();
	const normalized: ActivityLogDataColumnId[] = [];

	for (const entry of order) {
		if (!isActivityLogDataColumnId(entry) || seen.has(entry)) {
			continue;
		}
		seen.add(entry);
		normalized.push(entry);
	}

	for (const columnId of ACTIVITY_LOG_DATA_COLUMN_IDS) {
		if (!seen.has(columnId)) {
			normalized.push(columnId);
		}
	}

	return normalized;
}

function normalizeColumnWidths(
	widths: unknown,
): Record<ActivityLogDataColumnId, number | null> {
	const normalized = { ...DEFAULT_ACTIVITY_LOG_COLUMN_WIDTHS };

	if (!widths || typeof widths !== "object") {
		return normalized;
	}

	for (const columnId of ACTIVITY_LOG_DATA_COLUMN_IDS) {
		const value = (widths as Record<string, unknown>)[columnId];
		if (value === null) {
			normalized[columnId] = null;
			continue;
		}
		if (typeof value === "number" && Number.isFinite(value) && value >= ACTIVITY_LOG_MIN_COLUMN_WIDTH) {
			normalized[columnId] = Math.round(value);
		}
	}

	return normalized;
}

export function parseActivityLogTableLayout(raw: unknown): ActivityLogTableLayout {
	if (!raw || typeof raw !== "object") {
		return createDefaultActivityLogTableLayout();
	}

	const record = raw as Record<string, unknown>;
	return {
		columnOrder: normalizeColumnOrder(record.columnOrder),
		columnWidths: normalizeColumnWidths(record.columnWidths),
	};
}

export function loadActivityLogTableLayout(storageKey: string): ActivityLogTableLayout {
	if (typeof window === "undefined") {
		return createDefaultActivityLogTableLayout();
	}

	try {
		const raw = window.localStorage.getItem(storageKey);
		if (!raw) {
			return createDefaultActivityLogTableLayout();
		}
		return parseActivityLogTableLayout(JSON.parse(raw));
	} catch {
		return createDefaultActivityLogTableLayout();
	}
}

export function saveActivityLogTableLayout(
	storageKey: string,
	layout: ActivityLogTableLayout,
): void {
	if (typeof window === "undefined") {
		return;
	}

	try {
		window.localStorage.setItem(storageKey, JSON.stringify(layout));
	} catch {
		// Ignore quota or private-mode failures.
	}
}

export function setActivityLogColumnWidth(
	layout: ActivityLogTableLayout,
	columnId: ActivityLogDataColumnId,
	nextWidth: number,
): ActivityLogTableLayout {
	return {
		...layout,
		columnWidths: {
			...layout.columnWidths,
			[columnId]: Math.max(ACTIVITY_LOG_MIN_COLUMN_WIDTH, Math.round(nextWidth)),
		},
	};
}

export function resizeActivityLogColumnWidth(
	layout: ActivityLogTableLayout,
	columnId: ActivityLogDataColumnId,
	nextWidth: number,
): ActivityLogTableLayout {
	return setActivityLogColumnWidth(layout, columnId, nextWidth);
}

export function reorderActivityLogColumns(
	layout: ActivityLogTableLayout,
	sourceColumnId: ActivityLogDataColumnId,
	targetColumnId: ActivityLogDataColumnId,
): ActivityLogTableLayout {
	if (sourceColumnId === targetColumnId) {
		return layout;
	}

	const nextOrder = layout.columnOrder.filter((columnId) => columnId !== sourceColumnId);
	const targetIndex = nextOrder.indexOf(targetColumnId);
	if (targetIndex < 0) {
		return layout;
	}

	nextOrder.splice(targetIndex, 0, sourceColumnId);
	return {
		...layout,
		columnOrder: nextOrder,
	};
}
