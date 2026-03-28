import { countTokens } from "./token-utils";
import type { CapabilityTokenLedgerRow } from "./types";

function extractLedgerPayloadBody(payloadJson: string): Record<string, unknown> | null {
	try {
		const parsed = JSON.parse(payloadJson) as unknown;
		if (!parsed || typeof parsed !== "object") {
			return null;
		}
		const record = parsed as Record<string, unknown>;
		if (typeof record.enabled === "boolean") {
			return record;
		}
		for (const key of ["Tool", "Prompt", "Resource", "ResourceTemplate"]) {
			const nested = record[key];
			if (nested && typeof nested === "object") {
				return nested as Record<string, unknown>;
			}
		}
		return record;
	} catch {
		return null;
	}
}

function isLedgerRowEnabled(row: CapabilityTokenLedgerRow): boolean {
	const payload = extractLedgerPayloadBody(row.payload_json);
	return typeof payload?.enabled === "boolean" ? payload.enabled : false;
}

function computeLedgerTokens(
	ledger: CapabilityTokenLedgerRow[] | undefined,
	isRowVisible: (row: CapabilityTokenLedgerRow) => boolean,
): { totalTokens: number; visibleTokens: number } {
	if (!ledger?.length) {
		return { totalTokens: 0, visibleTokens: 0 };
	}

	let totalTokens = 0;
	let visibleTokens = 0;

	for (const row of ledger) {
		const rowTokens = countTokens(row.payload_json);
		totalTokens += rowTokens;

		if (isRowVisible(row)) {
			visibleTokens += rowTokens;
		}
	}

	return { totalTokens, visibleTokens };
}

/**
 * Aggregate cl100k token counts directly from a profile ledger.
 * Useful when the enabled state is already embedded in the serialized payloads.
 */
export function computeProfileLedgerTokens(
	ledger: CapabilityTokenLedgerRow[] | undefined,
): { totalTokens: number; visibleTokens: number } {
	return computeLedgerTokens(
		ledger,
		(row) => row.server_enabled_in_profile && isLedgerRowEnabled(row),
	);
}

/**
 * Aggregate cl100k token counts for profile trimming: sums payload_json per row,
 * applying server + per-component enable flags from the live dashboard state.
 */
export function computeProfileTrimTokens(
	ledger: CapabilityTokenLedgerRow[] | undefined,
	enabledByComponentId: ReadonlyMap<string, boolean>,
): { totalTokens: number; visibleTokens: number } {
	return computeLedgerTokens(ledger, (row) => {
		if (!row.server_enabled_in_profile) {
			return false;
		}
		return enabledByComponentId.get(row.profile_row_id) ?? false;
	});
}
