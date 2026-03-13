import type { JsonObject, JsonSchema, JsonValue } from "../types/json";

const toSingleSchema = (
	schema: JsonSchema | JsonSchema[] | undefined,
): JsonSchema =>
	Array.isArray(schema) ? schema[0] : schema ?? {};

const isJsonObject = (value: unknown): value is JsonObject =>
	Boolean(value) && typeof value === "object" && !Array.isArray(value);

export function defaultFromSchema(schema: JsonSchema | undefined): JsonValue {
	try {
		if (!schema) return {};
		if (schema.default !== undefined) return schema.default;
		const t = Array.isArray(schema.type) ? schema.type[0] : schema.type;
		if (schema.enum && schema.enum.length) return schema.enum[0];
		switch ((t || "object").toLowerCase()) {
			case "string":
				return schema.examples?.[0] ?? "example";
			case "integer":
				return 1;
			case "number":
				return 1;
			case "boolean":
				return true;
			case "array": {
				const item = defaultFromSchema(
					toSingleSchema(schema.items) ?? { type: "string" },
				);
				return [item];
			}
			case "object": {
				const o: JsonObject = {};
				const props = schema.properties || {};
				Object.keys(props).forEach((k) => {
					o[k] = defaultFromSchema(props[k]);
				});
				return o;
			}
			default:
				return "example";
		}
	} catch {
		return {};
	}
}

export const isJsonObjectValue = isJsonObject;
