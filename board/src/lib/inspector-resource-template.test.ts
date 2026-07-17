import { describe, expect, test } from "bun:test";

import * as resourceTemplate from "./inspector-resource-template";

const template = {
	unique_uri_template:
		"mcpmate://resources/template/everything/demo/dynamic/text/{resourceId}",
	uriTemplate:
		"mcpmate://resources/template/everything/demo/dynamic/text/{resourceId}",
	uri_template: "demo://resource/dynamic/text/{resourceId}",
};

describe("Inspector resource template projection", () => {
	test("uses exact upstream templates in Native mode and canonical templates in Proxy mode", () => {
		expect(resourceTemplate.pickInspectorResourceTemplateForMode(template, "native")).toBe(
			"demo://resource/dynamic/text/{resourceId}",
		);
		expect(resourceTemplate.pickInspectorResourceTemplateForMode(template, "proxy")).toBe(
			"mcpmate://resources/template/everything/demo/dynamic/text/{resourceId}",
		);
	});

	test("extracts variables without treating the RFC 6570 query operator as part of the name", () => {
		expect(
			resourceTemplate.extractInspectorResourceTemplateParameters(
				"demo://resource/dynamic/text/{resourceId}",
			),
		).toEqual(["resourceId"]);
		expect(
			resourceTemplate.extractInspectorResourceTemplateParameters(
				"mcpmate://resources/template/everything/demo/report{?year,month}",
			),
		).toEqual(["year", "month"]);
	});

	test("does not expose a client-side URI expansion path", () => {
		expect(resourceTemplate).not.toHaveProperty(
			"expandInspectorResourceTemplate",
		);
	});
});
