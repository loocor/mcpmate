import { describe, expect, test } from "bun:test";

import { requireSelectedCapabilityRevisionSet } from "./profile-capability-revisions";
import { ProfileSyncClientError } from "./profile-sync-error";

function captureError(run: () => void): unknown {
	try {
		run();
	} catch (error) {
		return error;
	}
	return null;
}

describe("requireSelectedCapabilityRevisionSet", () => {
	test("keeps only revisions for Servers that own the selected CapabilityRefs", () => {
		expect(
			requireSelectedCapabilityRevisionSet([
				{
					selectedIds: ["tool-a"],
					items: [
						{ id: "tool-a", server_id: "server-a" },
						{ id: "tool-b", server_id: "server-b" },
					],
					sourceRevisionSet: {
						"server-a": 4,
						"server-b": 9,
					},
				},
			]),
		).toEqual({ "server-a": 4 });
	});

	test("unions exact revisions across capability kinds", () => {
		expect(
			requireSelectedCapabilityRevisionSet([
				{
					selectedIds: ["tool-a"],
					items: [{ id: "tool-a", server_id: "server-a" }],
					sourceRevisionSet: { "server-a": 4, "server-z": 12 },
				},
				{
					selectedIds: ["resource-b"],
					items: [{ id: "resource-b", server_id: "server-b" }],
					sourceRevisionSet: { "server-b": 7 },
				},
			]),
		).toEqual({ "server-a": 4, "server-b": 7 });
	});

	test("rejects missing selected CapabilityRef revision evidence", () => {
		for (const source of [
			{
				selectedIds: ["missing-tool"],
				items: [{ id: "tool-a", server_id: "server-a" }],
				sourceRevisionSet: { "server-a": 4 },
			},
			{
				selectedIds: ["tool-a"],
				items: [{ id: "tool-a", server_id: "server-a" }],
				sourceRevisionSet: undefined,
			},
			{
				selectedIds: ["tool-a"],
				items: [{ id: "tool-a", server_id: "server-a" }],
				sourceRevisionSet: {},
			},
		]) {
			const error = captureError(() =>
				requireSelectedCapabilityRevisionSet([source]),
			);
			expect(error).toBeInstanceOf(ProfileSyncClientError);
			expect(error).toMatchObject({ code: "catalog_snapshot_missing" });
		}
	});

	test("rejects inconsistent revisions for the same selected Server", () => {
		const error = captureError(() =>
			requireSelectedCapabilityRevisionSet([
				{
					selectedIds: ["tool-a"],
					items: [{ id: "tool-a", server_id: "server-a" }],
					sourceRevisionSet: { "server-a": 4 },
				},
				{
					selectedIds: ["resource-a"],
					items: [{ id: "resource-a", server_id: "server-a" }],
					sourceRevisionSet: { "server-a": 5 },
				},
			]),
		);
		expect(error).toBeInstanceOf(ProfileSyncClientError);
		expect(error).toMatchObject({ code: "catalog_snapshot_mismatch" });
	});
});
