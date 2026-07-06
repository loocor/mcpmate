import type { TFunction } from "i18next";
import { cn } from "../lib/utils";

export const INSPECTOR_RECORD_FILTERABLE_KEYS = new Set(["server_id", "session_id"]);

/** Shared field priority for inspector drawer property rows. */
export const INSPECTOR_RECORD_PROPERTY_FIELD_ORDER = [
	"mode",
	"server_id",
	"session_id",
	"event",
	"scratch_id",
	"server_name",
	"name",
	"method",
	"timeout_ms",
	"call_id",
	"elapsed_ms",
] as const;

function asRecord(value: unknown): Record<string, unknown> | null {
	if (!value || typeof value !== "object" || Array.isArray(value)) {
		return null;
	}
	return value as Record<string, unknown>;
}

export function isFlatInspectorRecord(value: unknown): value is Record<string, unknown> {
	const record = asRecord(value);
	if (!record) {
		return false;
	}
	return Object.values(record).every(
		(entry) =>
			entry === null ||
			entry === undefined ||
			typeof entry === "string" ||
			typeof entry === "number" ||
			typeof entry === "boolean",
	);
}

function humanizeFieldKey(key: string): string {
	return key
		.split("_")
		.map((part) => part.charAt(0).toUpperCase() + part.slice(1))
		.join(" ");
}

export function formatInspectorRecordValue(key: string, value: unknown): string {
	if (key === "mode" && typeof value === "string") {
		return value.charAt(0).toUpperCase() + value.slice(1);
	}
	if (value === null || value === undefined) {
		return "";
	}
	return String(value);
}

export function inspectorRecordFieldLabel(key: string, t: TFunction): string {
	return t(`activity.drawer.contextFields.${key}`, {
		defaultValue: humanizeFieldKey(key),
	});
}

export function buildInspectorRecordPropertyRows(
	record: Record<string, unknown>,
	fieldOrder: readonly string[] = INSPECTOR_RECORD_PROPERTY_FIELD_ORDER,
): Array<{ key: string; value: unknown }> {
	const orderedKeys = new Set<string>();
	const rows: Array<{ key: string; value: unknown }> = [];

	for (const key of fieldOrder) {
		const value = record[key];
		if (value === undefined || value === null || value === "") {
			continue;
		}
		orderedKeys.add(key);
		rows.push({ key, value });
	}

	for (const key of Object.keys(record).sort()) {
		if (orderedKeys.has(key)) {
			continue;
		}
		const value = record[key];
		if (value === undefined || value === null || value === "") {
			continue;
		}
		rows.push({ key, value });
	}

	return rows;
}

export const INSPECTOR_RECORD_PROPERTY_PANEL_CLASSNAME =
	"divide-y divide-border rounded-md border border-border bg-card text-card-foreground";

export function InspectorRecordPropertyPanel({
	record,
	t,
	fieldOrder = INSPECTOR_RECORD_PROPERTY_FIELD_ORDER,
	filterableKeys = INSPECTOR_RECORD_FILTERABLE_KEYS,
	onFilterByServerId,
	onFilterBySessionId,
}: {
	record: Record<string, unknown>;
	t: TFunction;
	fieldOrder?: readonly string[];
	filterableKeys?: ReadonlySet<string>;
	onFilterByServerId?: (serverId: string) => void;
	onFilterBySessionId?: (sessionId: string) => void;
}) {
	const rows = buildInspectorRecordPropertyRows(record, fieldOrder);

	if (rows.length === 0) {
		return null;
	}

	return (
		<div className={INSPECTOR_RECORD_PROPERTY_PANEL_CLASSNAME}>
			{rows.map(({ key, value }) => {
				const displayValue = formatInspectorRecordValue(key, value);
				const isFilterable = filterableKeys.has(key);
				const handleFilter =
					key === "server_id" && typeof value === "string"
						? onFilterByServerId
						: key === "session_id" && typeof value === "string"
							? onFilterBySessionId
							: undefined;

				return (
					<div
						key={key}
						className="grid grid-cols-[minmax(0,6.5rem)_minmax(0,1fr)] items-baseline gap-3 px-3 py-2 text-xs"
					>
						<span className="text-muted-foreground">{inspectorRecordFieldLabel(key, t)}</span>
						{isFilterable && handleFilter ? (
							<button
								type="button"
								className={cn(
									"min-w-0 truncate text-left font-mono text-foreground",
									"rounded-sm hover:underline focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
								)}
								title={t("activity.drawer.filterByField", {
									defaultValue: "Filter activity by {{field}}",
									field: inspectorRecordFieldLabel(key, t),
								})}
								onClick={() => handleFilter(displayValue)}
							>
								{displayValue}
							</button>
						) : (
							<span className="min-w-0 truncate font-mono text-foreground">
								{displayValue}
							</span>
						)}
					</div>
				);
			})}
		</div>
	);
}
