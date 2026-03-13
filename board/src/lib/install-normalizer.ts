import JSON5 from "json5";
import JSZip from "jszip";
import * as TOML from "toml";
import type { ServerInstallDraft } from "../hooks/use-server-install-pipeline";
import type {
	RegistryMetaPayload,
	RegistryRepositoryInfo,
	ServerIcon,
	ServerMetaInfo,
} from "./types";

const normalizeKind = (value: unknown): ServerInstallDraft["kind"] => {
	const token = typeof value === "string" ? value.trim().toLowerCase() : "";
	switch (token) {
		case "stdio":
		case "command":
		case "process":
			return "stdio";
		case "sse":
		case "server-sent-events":
			return "sse";
		case "streamable_http":
		case "streamable-http":
		case "http":
		case "http_stream":
			return "streamable_http";
		default:
			return "stdio";
	}
};

const toStringArray = (value: unknown): string[] | undefined => {
	if (!value) return undefined;
	if (Array.isArray(value)) {
		const filtered = value
			.map((entry) => (typeof entry === "string" ? entry.trim() : ""))
			.filter(Boolean);
		return filtered.length ? filtered : undefined;
	}
	if (typeof value === "string" && value.trim()) {
		return value
			.split(/\s+/)
			.map((part) => part.trim())
			.filter(Boolean);
	}
	return undefined;
};

const toEnvRecord = (value: unknown): Record<string, string> | undefined => {
	if (!value || typeof value !== "object" || value === null) return undefined;
	const entries: Record<string, string> = {};
	for (const [key, raw] of Object.entries(value as Record<string, unknown>)) {
		if (!key) continue;
		const trimmed = typeof raw === "string" ? raw.trim() : undefined;
		entries[key] = trimmed ?? String(raw);
	}
	return Object.keys(entries).length ? entries : undefined;
};

const normalizeRepository = (
	input: unknown,
): RegistryRepositoryInfo | undefined => {
	if (!input) return undefined;
	if (typeof input === "string") {
		const url = input.trim();
		return url ? { url } : undefined;
	}
	if (typeof input !== "object" || input === null) return undefined;
	const repo = input as Record<string, unknown>;
	const payload: RegistryRepositoryInfo = {};
	const url = typeof repo.url === "string" ? repo.url.trim() : undefined;
	const source =
		typeof repo.source === "string" ? repo.source.trim() : undefined;
	const subfolder =
		typeof repo.subfolder === "string" ? repo.subfolder.trim() : undefined;
	const id = typeof repo.id === "string" ? repo.id.trim() : undefined;
	if (url) payload.url = url;
	if (source) payload.source = source;
	if (subfolder) payload.subfolder = subfolder;
	if (id) payload.id = id;
	return Object.keys(payload).length ? payload : undefined;
};

const normalizeServerIcon = (icon: unknown): ServerIcon | null => {
	if (!icon || typeof icon !== "object") return null;
	const obj = icon as Record<string, unknown>;
	const src =
		typedefString(obj.src) || typedefString(obj.url) || typedefString(obj.href);
	if (!src) return null;
	const mimeType =
		typedefString(obj.mime_type) || typedefString(obj.mimeType) || undefined;
	const sizes =
		typedefString(obj.sizes) || typedefString(obj.size) || undefined;
	const normalized: ServerIcon = { src };
	if (mimeType) normalized.mimeType = mimeType;
	if (sizes) normalized.sizes = sizes;
	return normalized;
};

const normalizeIconList = (value: unknown): ServerIcon[] => {
	if (!value) return [];
	const array = Array.isArray(value) ? value : [value];
	return array
		.map((entry) => normalizeServerIcon(entry))
		.filter((x): x is ServerIcon => Boolean(x));
};

const normalizeMeta = (metaLike: unknown): ServerMetaInfo | undefined => {
	if (!metaLike || typeof metaLike !== "object" || metaLike === null)
		return undefined;
	const raw = metaLike as Record<string, unknown>;
	const meta: ServerMetaInfo = {};

	const description =
		typeof raw.description === "string" ? raw.description.trim() : undefined;
	const version =
		typeof raw.version === "string" ? raw.version.trim() : undefined;
	const website =
		typedefString(raw.websiteUrl) ||
		typedefString(raw.website_url) ||
		typedefString(raw.website);
	const repository = normalizeRepository(raw.repository);
	const icons = normalizeIconList(raw.icons);

	if (description) meta.description = description;
	if (version) meta.version = version;
	if (website) meta.websiteUrl = website;
	if (repository) meta.repository = repository;
	if (icons.length) meta.icons = icons;

	if (raw._meta && typeof raw._meta === "object") {
		meta._meta = raw._meta as RegistryMetaPayload;
	}

	if (raw.extras && typeof raw.extras === "object") {
		meta.extras = raw.extras as Record<string, unknown>;
	}

	return Object.keys(meta).length ? meta : undefined;
};

const typedefString = (value: unknown): string | undefined => {
	if (typeof value !== "string") return undefined;
	const trimmed = value.trim();
	return trimmed.length ? trimmed : undefined;
};

const hydrateMeta = (
	base: ServerMetaInfo | undefined,
	extra: Record<string, unknown> | undefined,
): ServerMetaInfo | undefined => {
	if (!base && !extra) return undefined;
	const next: ServerMetaInfo = base ? { ...base } : {};
	if (extra) {
		next.extras = {
			...(next.extras ?? {}),
			...extra,
		};
	}
	return Object.keys(next).length ? next : undefined;
};

export const parseJsonDrafts = (text: string): ServerInstallDraft[] => {
	const trimmed = text.trim();
	if (!trimmed) return [];
	const json = JSON.parse(trimmed);
	return draftFromJson(json);
};

// --- TOML helpers -----------------------------------------------------------

/** Try to parse a TOML string. Returns object or null. */
const tryParseToml = (source: string): unknown | null => {
	try {
		return TOML.parse(source);
	} catch {
		return null;
	}
};

/**
 * Extract a TOML object from arbitrary text using a few heuristics:
 * 1) Parse whole text as TOML
 * 2) Parse fenced ```toml code blocks
 * 3) Parse from a section header like [mcp_servers.*] or [[servers]] to the next blank-blank boundary
 * 4) Parse a compact key-value window when we detect lines like `command =`, `args =`, `env =`
 */
export const extractTomlFromText = (text: string): unknown | null => {
	const trimmed = text.trim();
	if (!trimmed) return null;

	// 1) Whole-text parse
	const whole = tryParseToml(trimmed);
	if (whole) return whole;

	// 2) Fenced code blocks ```toml
	const fenceRegex = /```\s*toml\s*([\s\S]*?)```/gi;
	for (const m of trimmed.matchAll(fenceRegex)) {
		const block = m[1];
		const parsed = tryParseToml(block);
		if (parsed) return parsed;
	}

	// 3) Section header window: [mcp_servers.*] or [[servers]] or [servers]
	const lines = trimmed.split(/\r?\n/);
	const isTomlish = (line: string) =>
		/^\s*\[.*\]\s*$/.test(line) || // section
		/^\s*[A-Za-z0-9_.-]+\s*=/.test(line) || // key = value
		/^\s*#/.test(line); // comment

	let start = -1;
	for (let i = 0; i < lines.length; i++) {
		const l = lines[i];
		if (
			/^\s*\[(?:mcp_servers(?:\.[^\]]+)?)\]\s*$/.test(l) ||
			/^\s*\[\[servers\]\]\s*$/.test(l) ||
			/^\s*\[servers\]\s*$/.test(l)
		) {
			start = i;
			break;
		}
	}
	if (start >= 0) {
		// extend until we hit two consecutive non-toml-ish lines or end
		let end = lines.length;
		let nonTomlCount = 0;
		for (let j = start; j < lines.length; j++) {
			if (isTomlish(lines[j]) || lines[j].trim() === "") {
				nonTomlCount = 0;
				continue;
			}
			nonTomlCount++;
			if (nonTomlCount >= 2) {
				end = j - 1;
				break;
			}
		}
		const slice = lines.slice(start, end).join("\n");
		const parsed = tryParseToml(slice);
		if (parsed) return parsed;
	}

	// 4) Key window: look for cluster containing typical keys
	const keyHints = ["command", "args", "env", "type", "kind"];
	const keyLine = (s: string) => /^\s*[A-Za-z0-9_.-]+\s*=/.test(s);
	let firstHint = -1;
	for (let i = 0; i < lines.length; i++) {
		const l = lines[i];
		if (keyHints.some((k) => new RegExp(`^\\s*${k}\\s*=`).test(l))) {
			firstHint = i;
			break;
		}
	}
	if (firstHint >= 0) {
		// expand window upward to previous blank or section, downward to next blank
		let top = firstHint;
		while (top > 0 && (keyLine(lines[top - 1]) || lines[top - 1].trim() === ""))
			top--;
		let bottom = firstHint;
		while (
			bottom + 1 < lines.length &&
			(keyLine(lines[bottom + 1]) || lines[bottom + 1].trim() === "")
		)
			bottom++;
		const slice = lines.slice(top, bottom + 1).join("\n");
		const parsed = tryParseToml(slice);
		if (parsed) return parsed;
	}

	return null;
};

/** Convert TOML root keys to the JSON shapes our JSON parser expects. */
const normalizeTomlRoot = (obj: unknown): unknown => {
	if (!obj || typeof obj !== "object" || obj === null) return obj;
	const clone: Record<string, unknown> = {
		...(obj as Record<string, unknown>),
	};
	if (clone.mcp_servers && !clone.mcpServers) {
		clone.mcpServers = clone.mcp_servers as unknown;
	}
	return clone;
};

export const parseTomlDrafts = (text: string): ServerInstallDraft[] => {
	const parsed = extractTomlFromText(text);
	if (!parsed) return [];
	try {
		const normalized = normalizeTomlRoot(parsed);
		return draftFromJson(normalized);
	} catch {
		return [];
	}
};

const draftFromJson = (value: unknown): ServerInstallDraft[] => {
	if (!value || typeof value !== "object" || value === null) return [];
	const raw = value as Record<string, unknown>;

	if (
		raw.mcpServers &&
		typeof raw.mcpServers === "object" &&
		raw.mcpServers !== null
	) {
		return Object.entries(raw.mcpServers as Record<string, unknown>)
			.map(([name, config]) => buildDraft(name, config))
			.filter((draft): draft is ServerInstallDraft => Boolean(draft));
	}

	if (
		raw.servers &&
		typeof raw.servers === "object" &&
		!Array.isArray(raw.servers)
	) {
		const container = raw.servers as Record<string, unknown>;
		const drafts = Object.entries(container)
			.map(([name, config]) => buildDraft(name, config))
			.filter((draft): draft is ServerInstallDraft => Boolean(draft));
		if (drafts.length) {
			return drafts;
		}
	}

	if (Array.isArray(raw.servers)) {
		return raw.servers
			.map((entry: unknown) =>
				buildDraft((entry as { name?: unknown })?.name, entry),
			)
			.filter((draft): draft is ServerInstallDraft => Boolean(draft));
	}

	// TODO: Support secure `inputs` (e.g., VS Code promptString) for credential handling.

	// Single server object fallback
	const draft = buildDraft(
		(raw as { name?: unknown })?.name ?? "imported-server",
		raw,
	);
	return draft ? [draft] : [];
};

const buildDraft = (
	name: unknown,
	config: unknown,
): ServerInstallDraft | undefined => {
	if (!config || typeof config !== "object" || config === null)
		return undefined;
	const raw = config as Record<string, unknown>;
	const draftName =
		typeof name === "string" && name.trim() ? name.trim() : undefined;
	const rawKind = raw.kind ?? raw.type ?? raw.server_type;
	let kind = normalizeKind(rawKind);
	const command = typedefString(raw.command ?? raw.command_path ?? raw.launch);
	let url = typedefString(
		raw.url ??
			(raw as any)?.httpUrl ??
			(raw as any)?.http_url ??
			raw.endpoint ??
			raw.baseUrl,
	);
	// Extract URL parameters from query string if present
	let urlParams: Record<string, string> | undefined;
	if (url?.includes("?")) {
		try {
			const u = new URL(url, "http://dummy.local");
			const base = `${u.protocol}//${u.host}${u.pathname}`;
			const params: Record<string, string> = {};
			u.searchParams.forEach((v, k) => {
				params[k] = v;
			});
			urlParams = Object.keys(params).length ? params : undefined;
			// If original had no protocol (relative), keep original before '?'
			if (/^https?:/i.test(url)) {
				url = base;
			} else {
				url = url.split("?")[0];
			}
		} catch {}
	}
	// Merge explicit url params fields
	const explicitParams =
		(raw as any)?.urlParams ||
		(raw as any)?.url_params ||
		(raw as any)?.url_parameters;
	if (explicitParams && typeof explicitParams === "object") {
		urlParams = {
			...(urlParams ?? {}),
			...(explicitParams as Record<string, string>),
		};
	}
	const args = toStringArray(raw.args);
	const env = toEnvRecord(raw.env);
	const headers =
		toEnvRecord(raw.headers) ||
		toEnvRecord((raw as any)?.httpHeaders) ||
		toEnvRecord((raw as any)?.requestHeaders);
	const registryServerId = typedefString(
		raw.registry_server_id ?? raw.registryServerId,
	);

	const baseMeta = normalizeMeta(raw.meta);
	let extras: Record<string, unknown> | undefined;
	if (raw.extras && typeof raw.extras === "object") {
		extras = raw.extras as Record<string, unknown>;
	}
	if (raw.manifest && typeof raw.manifest === "object") {
		extras = {
			...(extras ?? {}),
			manifest: raw.manifest,
		};
	}

	const meta = hydrateMeta(baseMeta, extras);

	const envForDraft = env && Object.keys(env).length ? env : undefined;
	const headersForDraft = headers && Object.keys(headers).length ? headers : undefined;

	// If no kind was provided but we do have a URL, assume streamable_http
	if ((rawKind === undefined || rawKind === null) && url) {
		kind = "streamable_http";
	}

	return {
		name: draftName ?? generateServerName(kind),
		kind,
		command: kind === "stdio" ? command : undefined,
		url: kind !== "stdio" ? url : undefined,
		args: args,
		env: envForDraft,
		headers: headersForDraft ?? undefined,
		...(kind !== "stdio" && urlParams ? { urlParams } : {}),
		registryServerId,
		meta,
	};
};

const generateServerName = (kind: ServerInstallDraft["kind"]): string => {
	const base =
		kind === "stdio"
			? "stdio-server"
			: kind === "sse"
				? "sse-server"
				: "http-server";
	return `${base}-${Math.random().toString(36).slice(2, 8)}`;
};

export async function parseMcpbBundle(
	buffer: ArrayBuffer,
): Promise<ServerInstallDraft[]> {
	const zip = await JSZip.loadAsync(buffer);
	const manifestEntry = zip.file("manifest.json");
	if (!manifestEntry) {
		throw new Error("MCPB bundle missing manifest.json");
	}
	const manifestText = await manifestEntry.async("string");
	const manifest = JSON.parse(manifestText);
	const serverConfig = manifest.server?.mcp_config ?? {};
	const env = toEnvRecord(serverConfig.env);
	const args = toStringArray(serverConfig.args);
	const command = typedefString(serverConfig.command);

	const metaExtras = {
		"anthropic.mcpb/manifest": manifest,
	};
	const metaIcons = normalizeIconList(
		(manifest as any)?.icons || (manifest as any)?.icon,
	);
	const baseMeta = hydrateMeta(
		normalizeMeta({
			description: manifest.description,
			version: manifest.version,
			website: manifest.homepage ?? manifest.documentation,
			repository: manifest.repository,
			icons: metaIcons,
		}),
		metaExtras,
	);

	const envForDraft = env && Object.keys(env).length ? env : undefined;
	const draft: ServerInstallDraft = {
		name:
			typedefString(manifest.name) ||
			typedefString(manifest.display_name) ||
			generateServerName("stdio"),
		kind: "stdio",
		command: command,
		args,
		env: envForDraft,
		meta: baseMeta,
	};

	return [draft];
}

export const extractJsonFromText = (
	text: string,
	depth = 0,
): unknown | null => {
	const trimmed = text.trim();
	if (!trimmed) return null;
	if (depth > 3) return null; // prevent runaway recursion on malformed input

	// 1) Direct parse (JSON → JSON5)
	try {
		return JSON.parse(trimmed);
	} catch {}
	try {
		return JSON5.parse(trimmed);
	} catch {}

	// 2) Fenced code block: ```json ... ```
	const fenceMatch =
		trimmed.match(/```\s*json\s*([\s\S]+?)```/i) ||
		trimmed.match(/```([\s\S]+?)```/i);
	if (fenceMatch?.[1]) {
		const inner = fenceMatch[1];
		const parsed = extractJsonFromText(inner, depth + 1);
		if (parsed) return parsed;
	}

	// 3) Heuristic: missing outer braces around top-level property list
	//    Example: \"Context7\": { ... }  OR  \"A\": {...}, \"B\": {...}
	if (trimmed.startsWith('"') && !trimmed.startsWith('{"')) {
		const candidates: string[] = [];
		// try as-is wrapped
		candidates.push(`{${trimmed}}`);
		// also try trimming a trailing comma then wrap
		if (/,\s*$/.test(trimmed)) {
			candidates.push(`{${trimmed.replace(/,\s*$/, "")}}`);
		}
		for (const cand of candidates) {
			try {
				const parsed = JSON.parse(cand);
				if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
					// If values are objects (likely name→server), hand back as mcpServers to preserve names
					const values = Object.values(parsed as Record<string, unknown>);
					const isDictOfObjects =
						values.length > 0 &&
						values.every(
							(v) => typeof v === "object" && v !== null && !Array.isArray(v),
						);
					return isDictOfObjects ? { mcpServers: parsed } : parsed;
				}
			} catch {}
			try {
				const parsed5 = JSON5.parse(cand);
				if (parsed5 && typeof parsed5 === "object" && !Array.isArray(parsed5)) {
					const values = Object.values(parsed5 as Record<string, unknown>);
					const isDictOfObjects =
						values.length > 0 &&
						values.every(
							(v) => typeof v === "object" && v !== null && !Array.isArray(v),
						);
					return isDictOfObjects ? { mcpServers: parsed5 } : parsed5;
				}
			} catch {}
		}
	}

	// 4) Fallback: extract the broadest {...} slice and retry
	const start = trimmed.indexOf("{");
	const end = trimmed.lastIndexOf("}");
	if (start >= 0 && end > start) {
		const snippet = trimmed.slice(start, end + 1);
		if (snippet !== trimmed) {
			return extractJsonFromText(snippet, depth + 1);
		}
	}

	return null;
};

export const draftFromText = (text: string): ServerInstallDraft[] => {
	const json = extractJsonFromText(text);
	if (!json) return [];
	try {
		return draftFromJson(json);
	} catch {
		return [];
	}
};

export const normalizeIngestResult = async (payload: {
	text?: string;
	buffer?: ArrayBuffer;
	fileName?: string;
}): Promise<ServerInstallDraft[]> => {
	if (payload.buffer) {
		return parseMcpbBundle(payload.buffer);
	}
	if (payload.text) {
		// Try JSON first for best fidelity, then TOML
		const asJson = draftFromText(payload.text);
		if (asJson.length) return asJson;
		const asToml = parseTomlDrafts(payload.text);
		if (asToml.length) return asToml;
		return [];
	}
	return [];
};
