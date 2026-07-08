import { describe, expect, it } from "vitest";

import {
	buildResourceUriFromTemplate,
	missingResourceTemplateVariables,
	resourceTemplateVariables,
	schemaFromResourceTemplateUri,
} from "./inspector-resource-template-uri";

describe("inspector resource template uri", () => {
	it("extracts variables from simple and operator expressions", () => {
		expect(
			resourceTemplateVariables("repo://{owner}/{repo}{/path}{?q,limit}"),
		).toEqual(["owner", "repo", "path", "q", "limit"]);
	});

	it("builds a schema for generated template forms", () => {
		expect(schemaFromResourceTemplateUri("file:///{path}{?line}")).toEqual({
			type: "object",
			properties: {
				path: {
					type: "string",
					description: "Value for path in the resource URI template.",
				},
				line: {
					type: "string",
					description: "Value for line in the resource URI template.",
				},
			},
			required: ["path", "line"],
		});
	});

	it("expands common URI template operators for resource reads", () => {
		expect(
			buildResourceUriFromTemplate("repo://{owner}/{repo}{/path}{?q,limit}", {
				owner: "open ai",
				repo: "mcpmate",
				path: "docs/readme.md",
				q: "server import",
				limit: 10,
			}),
		).toBe(
			"repo://open%20ai/mcpmate/docs%2Freadme.md?q=server%20import&limit=10",
		);
	});

	it("reports missing required template variables", () => {
		expect(
			missingResourceTemplateVariables("repo://{owner}/{repo}{?q}", {
				owner: "openai",
				repo: "",
			}),
		).toEqual(["repo", "q"]);
	});
});
