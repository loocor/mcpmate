export type InspectorResourceTemplateMode = "native" | "proxy";

function stringValue(value: unknown): string | undefined {
	return typeof value === "string" && value.length > 0 ? value : undefined;
}

function templateVariableNames(expression: string): string[] {
	const body = expression.replace(/^[+#./;?&]/, "");
	return body
		.split(",")
		.map((variable) => variable.replace(/\*$/, "").replace(/:\d+$/, ""))
		.filter(Boolean);
}

export function pickInspectorResourceTemplateForMode(
	source: Record<string, unknown> | null,
	mode: InspectorResourceTemplateMode,
): string {
	if (!source) return "";
	if (mode === "proxy") {
		return (
			stringValue(source.unique_uri_template) ??
			stringValue(source.unique_name) ??
			""
		);
	}
	return (
		stringValue(source.uri_template) ??
		stringValue(source.uriTemplate) ??
		""
	);
}

export function extractInspectorResourceTemplateParameters(
	template: string,
): string[] {
	const seen = new Set<string>();
	const parameters: string[] = [];
	for (const match of template.matchAll(/\{([^}]+)\}/g)) {
		for (const name of templateVariableNames(match[1])) {
			if (!seen.has(name)) {
				seen.add(name);
				parameters.push(name);
			}
		}
	}
	return parameters;
}
