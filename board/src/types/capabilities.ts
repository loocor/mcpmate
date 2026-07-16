import type { JsonSchema, JsonValue } from "./json";

export interface CapabilityArgument {
	name?: string;
	type?: string;
	description?: string;
	required?: boolean;
	default?: JsonValue;
}

export type CapabilityRecord = Record<string, unknown> & {
	arguments?: unknown;
	meta?: unknown;
	icon?: unknown;
	icons?: unknown;
	input_schema?: unknown;
	inputSchema?: unknown;
	output_schema?: unknown;
	outputSchema?: unknown;
	resource_uri?: unknown;
	uri?: unknown;
	mime_type?: unknown;
	description?: unknown;
	server_name?: unknown;
	unique_name?: unknown;
  unique_uri?: unknown;
  unique_uri_template?: unknown;
	tool_name?: unknown;
	prompt_name?: unknown;
	name?: unknown;
	id?: unknown;
};

export interface CapabilityMapItem<T> {
	title: string;
	upstreamIdentifier?: string;
	subtitle?: string;
	description?: string;
	server?: string;
	raw: T;
	icon?: string;
	schema?: JsonSchema;
	outputSchema?: JsonSchema;
	args?: CapabilityArgument[];
	mime?: string;
}
