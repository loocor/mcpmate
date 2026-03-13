export type JsonValue =
	| string
	| number
	| boolean
	| null
	| JsonObject
	| JsonArray;
export type JsonObject = { [key: string]: JsonValue };
export type JsonArray = JsonValue[];

export interface JsonSchema {
	type?: string | string[];
	properties?: Record<string, JsonSchema>;
	items?: JsonSchema | JsonSchema[];
	required?: string[];
	description?: string;
	default?: JsonValue;
	examples?: JsonValue[];
	enum?: JsonValue[];
}
