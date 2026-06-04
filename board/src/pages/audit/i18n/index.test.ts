import { readFileSync } from "node:fs";
import { describe, expect, test } from "bun:test";

import { auditTranslations } from ".";

const locales = ["en", "zh-CN", "ja-JP"] as const;

const frontendOnlyActions = new Set([
	"client_manage_enable",
	"client_manage_disable",
	"client_config_import",
]);

function auditActionsFromTypes(): string[] {
	const source = readFileSync(new URL("../../../lib/types.ts", import.meta.url), "utf8");
	const match = source.match(/export type AuditAction =([\s\S]*?);/);
	if (!match) {
		throw new Error("AuditAction union not found.");
	}

	return [...match[1].matchAll(/"([^"]+)"/g)].map((item) => item[1]);
}

function snakeCase(value: string): string {
	return value.replace(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase();
}

function backendAuditActions(): string[] {
	const source = readFileSync(
		new URL("../../../../../backend/src/audit/types.rs", import.meta.url),
		"utf8",
	);
	const match = source.match(/pub enum AuditAction \{([\s\S]*?)\n\}/);
	if (!match) {
		throw new Error("Backend AuditAction enum not found.");
	}

	return [...match[1].matchAll(/^\s*([A-Z][A-Za-z0-9]*),/gm)].map((item) =>
		snakeCase(item[1]),
	);
}

describe("audit translations", () => {
	test("defines labels for every AuditAction in all supported locales", () => {
		const actions = auditActionsFromTypes();

		for (const locale of locales) {
			const values = auditTranslations[locale].actionValues;
			const missing = actions.filter((action) => !(action in values));
			expect(missing).toEqual([]);
		}
	});

	test("covers backend audit actions without introducing unused action keys", () => {
		const frontendActions = auditActionsFromTypes();
		const backendActions = backendAuditActions();
		const missingBackendActions = backendActions.filter(
			(action) => !frontendActions.includes(action),
		);
		const unexpectedFrontendActions = frontendActions.filter(
			(action) => !backendActions.includes(action) && !frontendOnlyActions.has(action),
		);

		expect(missingBackendActions).toEqual([]);
		expect(unexpectedFrontendActions).toEqual([]);
	});
});
