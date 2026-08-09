import { describe, expect, test } from "bun:test";

import {
	editServerSchema,
	manualServerSchema,
	requireExplicitTransportSelection,
	transportDraftToFormFields,
} from "./types";
import { resolveTransportFocusField } from "../../lib/types";

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

describe("transport repair focus", () => {
	test("keeps an unrecognized transport out of the form until a user selects one", () => {
		const unrecognizedTransport = {
			kind: "unrecognized" as const,
			declared_type: "websocket",
		};

		expect(transportDraftToFormFields(unrecognizedTransport)).toBeNull();
		expect(() => requireExplicitTransportSelection(unrecognizedTransport)).toThrow(
			"websocket",
		);
		expect(
			requireExplicitTransportSelection(unrecognizedTransport, "sse"),
		).toBe("sse");
	});

	test("maps transport diagnostics to the editable form fields", () => {
		expect(resolveTransportFocusField("command", "stdio")).toBe("command");
		expect(resolveTransportFocusField("endpoint", "streamable_http")).toBe("url");
		expect(resolveTransportFocusField(undefined, "stdio")).toBe("command");
	});

	test("converts secret references into exact editable placeholders", () => {
		expect(
			transportDraftToFormFields({
				kind: "http",
				protocol: "streamable_http",
				endpoint: "https://example.com/mcp?mode=read",
				headers: {
					Authorization: { kind: "secret_ref", alias: "api-token" },
				},
			}),
		).toEqual({
			kind: "streamable_http",
			url: "https://example.com/mcp",
			urlParams: { mode: "read" },
			headers: { Authorization: "[[secret:api-token]]" },
		});
	});

	test("retains existing stdio literal environment values behind redacted drafts", () => {
		expect(
			transportDraftToFormFields(
				{
					kind: "stdio",
					command: null,
					args: [],
					env: { TOKEN: { kind: "literal", value: "***REDACTED***" } },
				},
				{ TOKEN: "actual-token" },
			),
		).toMatchObject({ env: { TOKEN: "actual-token" } });
	});
});
