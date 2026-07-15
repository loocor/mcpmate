import { describe, expect, test } from "bun:test";

import { editServerSchema, manualServerSchema } from "./types";

const stdioForm = {
	kind: "stdio" as const,
	command: "server-command",
};

describe("server install namespace validation", () => {
	test("requires a canonical namespace before creation", () => {
		const result = manualServerSchema.safeParse({
			...stdioForm,
			name: "Legacy Server-v2",
		});

		expect(result.success).toBe(false);
		if (!result.success) {
			expect(result.error.issues).toContainEqual(
				expect.objectContaining({
					path: ["name"],
					message: "manual.errors.namespaceInvalid",
				}),
			);
		}
	});

	test("accepts a canonical namespace before creation", () => {
		expect(
			manualServerSchema.safeParse({
				...stdioForm,
				name: "legacy_server_v2",
			}).success,
		).toBe(true);
	});

	test("allows editing other fields on a legacy immutable namespace", () => {
		expect(
			editServerSchema.safeParse({
				...stdioForm,
				name: "legacy-server",
			}).success,
		).toBe(true);
	});
});
