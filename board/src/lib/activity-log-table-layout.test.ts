import { describe, expect, it } from "vitest";
import {
	DEFAULT_ACTIVITY_LOG_TABLE_LAYOUT,
	parseActivityLogTableLayout,
	reorderActivityLogColumns,
	resizeActivityLogColumnWidth,
	setActivityLogColumnWidth,
} from "./activity-log-table-layout";

describe("activity-log-table-layout", () => {
	it("fills missing columns when parsing partial order", () => {
		const parsed = parseActivityLogTableLayout({
			columnOrder: ["duration", "timestamp"],
			columnWidths: { timestamp: 200 },
		});

		expect(parsed.columnOrder[0]).toBe("duration");
		expect(parsed.columnOrder[1]).toBe("timestamp");
		expect(parsed.columnOrder).toContain("target");
		expect(parsed.columnWidths.timestamp).toBe(200);
	});

	it("clamps resized widths to the minimum", () => {
		const resized = resizeActivityLogColumnWidth(
			DEFAULT_ACTIVITY_LOG_TABLE_LAYOUT,
			"action",
			12,
		);

		expect(resized.columnWidths.action).toBe(56);
	});

	it("reorders columns without dropping entries", () => {
		const reordered = reorderActivityLogColumns(
			DEFAULT_ACTIVITY_LOG_TABLE_LAYOUT,
			"duration",
			"timestamp",
		);

		expect(reordered.columnOrder.indexOf("duration")).toBeLessThan(
			reordered.columnOrder.indexOf("timestamp"),
		);
		expect(reordered.columnOrder).toHaveLength(
			DEFAULT_ACTIVITY_LOG_TABLE_LAYOUT.columnOrder.length,
		);
	});

	it("assigns a fixed width to flexible columns during autosize", () => {
		const autosized = setActivityLogColumnWidth(
			DEFAULT_ACTIVITY_LOG_TABLE_LAYOUT,
			"target",
			240,
		);

		expect(autosized.columnWidths.target).toBe(240);
	});
});
