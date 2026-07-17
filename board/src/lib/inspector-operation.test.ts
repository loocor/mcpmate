import { describe, expect, test } from "bun:test";

import {
	getInspectorModeIdentity,
	getInspectorOperationLabelKey,
	getInspectorPrimaryActionKey,
	normalizeInspectorCapabilityOption,
	resolveInspectorCounterpartIdentity,
	shouldAutoLoadInspectorOptions,
	shouldOfferCustomCapabilityValue,
	switchInspectorOperationSnapshot,
} from "./inspector-operation";

describe("Inspector operation helpers", () => {
	test("auto-loads each available Inspector capability list once", () => {
		expect(
			shouldAutoLoadInspectorOptions({
				canUseCurrentMode: true,
				hasAttemptedAutoLoad: false,
				hasListedOptions: false,
				isDrawerInitialized: true,
				isProxyChecking: false,
				open: true,
			}),
		).toBe(true);
		expect(
			shouldAutoLoadInspectorOptions({
				canUseCurrentMode: true,
				hasAttemptedAutoLoad: true,
				hasListedOptions: false,
				isDrawerInitialized: true,
				isProxyChecking: false,
				open: true,
			}),
		).toBe(false);
		expect(
			shouldAutoLoadInspectorOptions({
				canUseCurrentMode: true,
				hasAttemptedAutoLoad: false,
				hasListedOptions: true,
				isDrawerInitialized: true,
				isProxyChecking: false,
				open: true,
			}),
		).toBe(false);
		expect(
			shouldAutoLoadInspectorOptions({
				canUseCurrentMode: false,
				hasAttemptedAutoLoad: false,
				hasListedOptions: false,
				isDrawerInitialized: true,
				isProxyChecking: false,
				open: true,
			}),
		).toBe(false);
		expect(
			shouldAutoLoadInspectorOptions({
				canUseCurrentMode: true,
				hasAttemptedAutoLoad: false,
				hasListedOptions: false,
				isDrawerInitialized: true,
				isProxyChecking: true,
				open: true,
			}),
		).toBe(false);
	});

	test("offers a trimmed custom value only when it is not already listed", () => {
		const listed = [
			"mcpmate://resources/everything/demo/static/document/architecture.md",
		];

		expect(shouldOfferCustomCapabilityValue("  ", listed)).toBe(false);
		expect(
			shouldOfferCustomCapabilityValue(` ${listed[0]} `, listed),
		).toBe(false);
		expect(
			shouldOfferCustomCapabilityValue(
				" mcpmate://resources/everything/demo/dynamic/1 ",
				listed,
			),
		).toBe(true);
	});

	test("maps capability kinds to protocol operation labels", () => {
		expect(getInspectorOperationLabelKey("tool")).toBe("modes.toolCall");
		expect(getInspectorOperationLabelKey("prompt")).toBe("modes.getPrompt");
		expect(getInspectorOperationLabelKey("resource")).toBe(
			"modes.readResource",
		);
		expect(getInspectorOperationLabelKey("template")).toBe(
			"modes.readTemplate",
		);

		expect(getInspectorPrimaryActionKey("tool")).toBe("actions.call");
		expect(getInspectorPrimaryActionKey("prompt")).toBe("actions.get");
		expect(getInspectorPrimaryActionKey("resource")).toBe("actions.read");
		expect(getInspectorPrimaryActionKey("template")).toBe("actions.read");
	});

	test("restores operation state without crossing mode boundaries", () => {
		const snapshots = new Map<string, { selected: string }>([
			["native:prompt", { selected: "native-prompt" }],
			["proxy:prompt", { selected: "proxy-prompt" }],
		]);

		expect(
			switchInspectorOperationSnapshot(
				snapshots,
				"native",
				"tool",
				"prompt",
				{ selected: "native-tool" },
			),
		).toEqual({ selected: "native-prompt" });
		expect(snapshots.get("native:tool")).toEqual({
			selected: "native-tool",
		});
		expect(snapshots.get("proxy:prompt")).toEqual({
			selected: "proxy-prompt",
		});
	});

	test("projects standard Proxy list identities into explicit canonical fields", () => {
		expect(
			normalizeInspectorCapabilityOption("tool", "proxy", {
				name: "everything_get-resource-links",
			}),
		).toEqual({
			name: "everything_get-resource-links",
			unique_name: "everything_get-resource-links",
		});
		expect(
			normalizeInspectorCapabilityOption("resource", "proxy", {
				uri: "mcpmate://resources/everything/demo/static/document/a.md",
			}),
		).toEqual({
			uri: "mcpmate://resources/everything/demo/static/document/a.md",
			unique_uri:
				"mcpmate://resources/everything/demo/static/document/a.md",
		});
		expect(
			normalizeInspectorCapabilityOption("template", "proxy", {
				uriTemplate:
					"mcpmate://resources/template/everything/demo/dynamic/text/{resourceId}",
			}),
		).toEqual({
			uriTemplate:
				"mcpmate://resources/template/everything/demo/dynamic/text/{resourceId}",
			unique_uri_template:
				"mcpmate://resources/template/everything/demo/dynamic/text/{resourceId}",
		});
	});

	test("does not relabel Native list identities as canonical", () => {
		expect(
			normalizeInspectorCapabilityOption("tool", "native", {
				name: "get-resource-links",
			}),
		).toEqual({ name: "get-resource-links" });
	});

	test("does not use Resource Template display metadata as a Proxy identity", () => {
		expect(
			normalizeInspectorCapabilityOption("template", "proxy", {
				name: "Dynamic Text Resource",
			}),
		).toEqual({ name: "Dynamic Text Resource" });
	});

	test("selects capability identities from the active Inspector mode", () => {
		const tool = {
			unique_name: "everything_echo",
			tool_name: "echo",
			name: "everything_echo",
		};
		const prompt = {
			unique_name: "everything_review",
			prompt_name: "review",
			name: "everything_review",
		};
		const resource = {
			unique_uri: "mcpmate://resources/everything/demo/static/a.md",
			resource_uri: "demo://resource/static/a.md",
			uri: "mcpmate://resources/everything/demo/static/a.md",
		};
		const template = {
			unique_uri_template:
				"mcpmate://resources/template/everything/demo/dynamic/{resourceId}",
			uri_template: "demo://resource/dynamic/{resourceId}",
			uriTemplate:
				"mcpmate://resources/template/everything/demo/dynamic/{resourceId}",
		};

		expect(getInspectorModeIdentity("tool", "native", tool)).toBe("echo");
		expect(getInspectorModeIdentity("tool", "proxy", tool)).toBe(
			"everything_echo",
		);
		expect(getInspectorModeIdentity("prompt", "native", prompt)).toBe(
			"review",
		);
		expect(getInspectorModeIdentity("prompt", "proxy", prompt)).toBe(
			"everything_review",
		);
		expect(getInspectorModeIdentity("resource", "native", resource)).toBe(
			"demo://resource/static/a.md",
		);
		expect(getInspectorModeIdentity("resource", "proxy", resource)).toBe(
			"mcpmate://resources/everything/demo/static/a.md",
		);
		expect(getInspectorModeIdentity("template", "native", template)).toBe(
			"demo://resource/dynamic/{resourceId}",
		);
		expect(getInspectorModeIdentity("template", "proxy", template)).toBe(
			"mcpmate://resources/template/everything/demo/dynamic/{resourceId}",
		);
	});

	test("resolves the exact counterpart identity across Inspector modes", () => {
		const toolMappings = [
			{
				unique_name: "everything_echo",
				tool_name: "echo",
			},
			{
				unique_name: "everything_get-env",
				tool_name: "get-env",
			},
		];

		expect(
			resolveInspectorCounterpartIdentity(
				"tool",
				"native",
				"proxy",
				"echo",
				toolMappings,
			),
		).toBe("everything_echo");
		expect(
			resolveInspectorCounterpartIdentity(
				"tool",
				"proxy",
				"native",
				"everything_get-env",
				toolMappings,
			),
		).toBe("get-env");

		expect(
			resolveInspectorCounterpartIdentity(
				"prompt",
				"native",
				"proxy",
				"simple-prompt",
				[
					{
						prompt_name: "simple-prompt",
						unique_name: "everything_simple-prompt",
					},
				],
			),
		).toBe("everything_simple-prompt");

		expect(
			resolveInspectorCounterpartIdentity(
				"resource",
				"native",
				"proxy",
				"demo://resource/static/document/a.md",
				[
					{
						resource_uri: "demo://resource/static/document/a.md",
						unique_uri:
							"mcpmate://resources/everything/demo/static/document/a.md",
					},
				],
			),
		).toBe("mcpmate://resources/everything/demo/static/document/a.md");

		expect(
			resolveInspectorCounterpartIdentity(
				"template",
				"proxy",
				"native",
				"mcpmate://resources/template/everything/demo/dynamic/text/{resourceId}",
				[
					{
						uri_template: "demo://resource/dynamic/text/{resourceId}",
						unique_uri_template:
							"mcpmate://resources/template/everything/demo/dynamic/text/{resourceId}",
					},
				],
			),
		).toBe("demo://resource/dynamic/text/{resourceId}");
	});

	test("does not guess a counterpart identity from naming conventions", () => {
		expect(
			resolveInspectorCounterpartIdentity(
				"resource",
				"native",
				"proxy",
				"demo://resource/dynamic/unlisted",
				[
					{
						unique_uri:
							"mcpmate://resources/everything/demo/static/document/a.md",
						resource_uri: "demo://resource/static/document/a.md",
					},
				],
			),
		).toBe("");
	});
});
