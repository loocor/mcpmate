import type { JsonObject } from "../../types/json";

const RESERVED_OPERATOR_CHARS = new Set(["+", "#", ".", "/", ";", "?", "&"]);

type TemplateExpression = {
	raw: string;
	operator: string | null;
	variables: string[];
};

function encodedTemplateValue(value: unknown, allowReserved = false): string {
	const text = String(value ?? "");
	return allowReserved ? encodeURI(text) : encodeURIComponent(text);
}

function normalizeTemplateVariable(variable: string): string {
	return variable.trim().replace(/[:*].*$/, "").trim();
}

function parseTemplateExpression(expression: string): TemplateExpression {
	const first = expression[0] ?? "";
	const operator = RESERVED_OPERATOR_CHARS.has(first) ? first : null;
	const body = operator ? expression.slice(1) : expression;
	const variables = body
		.split(",")
		.map(normalizeTemplateVariable)
		.filter((variable) => variable.length > 0);
	return {
		raw: expression,
		operator,
		variables,
	};
}

function templateExpressions(template: string): TemplateExpression[] {
	return Array.from(template.matchAll(/\{([^{}]+)\}/g)).map((match) =>
		parseTemplateExpression(match[1] ?? ""),
	);
}

export function resourceTemplateVariables(template: string): string[] {
	const variables = new Set<string>();
	for (const expression of templateExpressions(template)) {
		for (const variable of expression.variables) {
			variables.add(variable);
		}
	}
	return Array.from(variables);
}

function hasTemplateVariableValue(args: JsonObject, variable: string): boolean {
	if (!Object.hasOwn(args, variable)) return false;
	const value = args[variable];
	if (value == null) return false;
	if (typeof value === "string") return value.trim().length > 0;
	return true;
}

export function missingResourceTemplateVariables(
	template: string,
	args: JsonObject,
): string[] {
	return resourceTemplateVariables(template).filter(
		(variable) => !hasTemplateVariableValue(args, variable),
	);
}

function expandNamedVariables(
	variables: string[],
	args: JsonObject,
	prefix: string,
	joiner: string,
	assignment = "=",
): string {
	const pairs = variables
		.filter((variable) => Object.hasOwn(args, variable))
		.map(
			(variable) =>
				`${encodeURIComponent(variable)}${assignment}${encodedTemplateValue(args[variable])}`,
		);
	return pairs.length > 0 ? `${prefix}${pairs.join(joiner)}` : "";
}

function expandTemplateExpression(
	expression: TemplateExpression,
	args: JsonObject,
): string {
	const values = expression.variables
		.filter((variable) => Object.hasOwn(args, variable))
		.map((variable) => ({
			name: variable,
			value: args[variable],
		}));

	switch (expression.operator) {
		case "?":
			return expandNamedVariables(expression.variables, args, "?", "&");
		case "&":
			return expandNamedVariables(expression.variables, args, "&", "&");
		case ";":
			return expandNamedVariables(expression.variables, args, ";", ";");
		case "/":
			return values.length > 0
				? `/${values.map((entry) => encodedTemplateValue(entry.value)).join("/")}`
				: "";
		case ".":
			return values.length > 0
				? `.${values.map((entry) => encodedTemplateValue(entry.value)).join(".")}`
				: "";
		case "#":
			return values.length > 0
				? `#${values.map((entry) => encodedTemplateValue(entry.value, true)).join(",")}`
				: "";
		case "+":
			return values.map((entry) => encodedTemplateValue(entry.value, true)).join(",");
		default:
			return values.map((entry) => encodedTemplateValue(entry.value)).join(",");
	}
}

export function buildResourceUriFromTemplate(
	template: string,
	args: JsonObject,
): string {
	return template.replace(/\{([^{}]+)\}/g, (_match, expression: string) =>
		expandTemplateExpression(parseTemplateExpression(expression), args),
	);
}

export function schemaFromResourceTemplateUri(
	template: string,
): Record<string, unknown> | undefined {
	const variables = resourceTemplateVariables(template);
	if (variables.length === 0) {
		return undefined;
	}
	return {
		type: "object",
		properties: Object.fromEntries(
			variables.map((variable) => [
				variable,
				{
					type: "string",
					description: `Value for ${variable} in the resource URI template.`,
				},
			]),
		),
		required: variables,
	};
}
