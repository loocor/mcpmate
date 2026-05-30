import { resolveApiUrl } from "../../lib/api";
import type {
	ClientCheckData,
	ConfigSuit,
	ConfigSuitListResponse,
	ServerListResponse,
	ServerSummary,
} from "../../lib/types";

interface ApiWrapper<T> {
	data?: T | null;
	error?: unknown | null;
	success: boolean;
}

interface ProfileListPayload {
	profile: OperatorProfileRow[];
	total: number;
	timestamp: string;
}

interface OperatorProfileRow {
	id: string;
	name: string;
	description?: string | null;
	profile_type: string;
	multi_select: boolean;
	priority: number;
	is_active: boolean;
	is_default: boolean;
	role?: string | null;
	allowed_operations: string[];
}

async function fetchOperatorJson<T>(path: string): Promise<T> {
	const response = await fetch(resolveApiUrl(path), {
		credentials: "include",
	});
	const text = await response.text();
	const payload = text ? (JSON.parse(text) as ApiWrapper<T>) : null;

	if (!response.ok) {
		const message =
			typeof payload?.error === "string" && payload.error.length > 0
				? payload.error
				: `Operator API request failed with status ${response.status}`;
		throw new Error(message);
	}

	if (!payload?.success || !payload.data) {
		throw new Error(
			typeof payload?.error === "string" && payload.error.length > 0
				? payload.error
				: "Operator API request failed",
		);
	}

	return payload.data;
}

function assertRecord(value: unknown, context: string): asserts value is Record<string, unknown> {
	if (!value || typeof value !== "object" || Array.isArray(value)) {
		throw new Error(`${context} is malformed`);
	}
}

function readString(record: Record<string, unknown>, field: string, context: string): string {
	const value = record[field];
	if (typeof value !== "string") {
		throw new Error(`${context} is missing ${field}`);
	}
	return value;
}

function readOptionalString(
	record: Record<string, unknown>,
	field: string,
	context: string,
): string | undefined {
	const value = record[field];
	if (value === undefined || value === null) {
		return undefined;
	}
	if (typeof value !== "string") {
		throw new Error(`${context} has invalid ${field}`);
	}
	return value;
}

function readBoolean(record: Record<string, unknown>, field: string, context: string): boolean {
	const value = record[field];
	if (typeof value !== "boolean") {
		throw new Error(`${context} is missing ${field}`);
	}
	return value;
}

function readNumber(record: Record<string, unknown>, field: string, context: string): number {
	const value = record[field];
	if (typeof value !== "number" || !Number.isFinite(value)) {
		throw new Error(`${context} is missing ${field}`);
	}
	return value;
}

function readStringArray(record: Record<string, unknown>, field: string, context: string): string[] {
	const value = record[field];
	if (!Array.isArray(value) || !value.every((item) => typeof item === "string")) {
		throw new Error(`${context} is missing ${field}`);
	}
	return value;
}

function profileRowToConfigSuit(row: unknown, index: number): ConfigSuit {
	const context = `Operator profile row ${index}`;
	assertRecord(row, context);

	return {
		id: readString(row, "id", context),
		name: readString(row, "name", context),
		description: readOptionalString(row, "description", context),
		suit_type: readString(row, "profile_type", context),
		multi_select: readBoolean(row, "multi_select", context),
		priority: readNumber(row, "priority", context),
		is_active: readBoolean(row, "is_active", context),
		is_default: readBoolean(row, "is_default", context),
		role: readOptionalString(row, "role", context),
		allowed_operations: readStringArray(row, "allowed_operations", context),
	};
}

export async function listOperatorProfiles(): Promise<ConfigSuitListResponse> {
	const data = await fetchOperatorJson<ProfileListPayload>("/api/mcp/profile/list");
	if (!Array.isArray(data.profile)) {
		throw new Error("Operator profiles response is missing profile list");
	}
	return { suits: data.profile.map(profileRowToConfigSuit) };
}

export async function listOperatorServers(): Promise<ServerListResponse> {
	const data = await fetchOperatorJson<{ servers: ServerSummary[] }>("/api/mcp/servers/list");
	if (!Array.isArray(data.servers)) {
		throw new Error("Operator servers response is missing server list");
	}
	return { servers: data.servers };
}

export async function listOperatorClients(): Promise<ClientCheckData> {
	const data = await fetchOperatorJson<ClientCheckData>("/api/client/list?refresh=false");
	if (!Array.isArray(data.client)) {
		throw new Error("Operator clients response is missing client list");
	}
	return data;
}
