import type {
	InspectorSessionHandshakeData,
	ServerIcon,
	ServerMetaInfo,
	ServerSummary,
} from "./types";

const isRecord = (value: unknown): value is Record<string, unknown> =>
	Boolean(value) && typeof value === "object" && !Array.isArray(value);

const toTrimmedString = (value: unknown): string | undefined => {
	if (typeof value !== "string") return undefined;
	const trimmed = value.trim();
	return trimmed.length > 0 ? trimmed : undefined;
};

const firstIconSrcFromList = (icons: unknown): string | undefined => {
	if (!icons) return undefined;
	const entries = Array.isArray(icons) ? icons : [icons];
	for (const entry of entries) {
		if (!isRecord(entry)) continue;
		const src =
			toTrimmedString(entry.src) ||
			toTrimmedString(entry.url) ||
			toTrimmedString(entry.href);
		if (src) return src;
	}
	return undefined;
};

const recordedIconSrcFromMeta = (meta?: ServerMetaInfo | null): string | undefined => {
	if (!meta) return undefined;

	const fromIcons = firstIconSrcFromList(meta.icons);
	if (fromIcons) return fromIcons;

	const extras = meta.extras;
	if (isRecord(extras)) {
		const fromExtras =
			toTrimmedString(extras.iconUrl) ||
			toTrimmedString(extras.brandIconUrl) ||
			toTrimmedString(extras.icon_url);
		if (fromExtras) return fromExtras;
	}

	const registryMeta = meta._meta;
	if (isRecord(registryMeta)) {
		const official = registryMeta["io.modelcontextprotocol.registry/official"];
		const publisherProvided =
			registryMeta["io.modelcontextprotocol.registry/publisher-provided"];
		return (
			firstIconSrcFromList(isRecord(official) ? official.icons : undefined) ||
			firstIconSrcFromList(
				isRecord(publisherProvided) ? publisherProvided.icons : undefined,
			)
		);
	}

	return undefined;
};

export function extractHandshakeServerIconSrc(
	handshake: InspectorSessionHandshakeData | null | undefined,
): string | undefined {
	if (!handshake?.messages.length) return undefined;

	const initializeResponse = handshake.messages.find(
		(message) => message.direction === "inbound" && message.method === "initialize",
	);
	if (!initializeResponse?.payload || !isRecord(initializeResponse.payload)) {
		return undefined;
	}

	const result = initializeResponse.payload.result;
	if (!isRecord(result)) return undefined;

	const serverInfo = result.serverInfo;
	if (!isRecord(serverInfo)) return undefined;

	return firstIconSrcFromList(serverInfo.icons);
}

export type ResolveInspectorServerIconSrcOptions = {
	runtimeIconSrc?: string | null;
};

export function resolveInspectorServerIconSrc(
	server: Pick<ServerSummary, "icons" | "meta" | "name">,
	options?: ResolveInspectorServerIconSrcOptions,
): string | undefined {
	const runtimeIconSrc = toTrimmedString(options?.runtimeIconSrc);
	if (runtimeIconSrc) return runtimeIconSrc;

	const builtinIconSrc = firstIconSrcFromList(server.icons);
	if (builtinIconSrc) return builtinIconSrc;

	return recordedIconSrcFromMeta(server.meta);
}

export function resolveInspectorServerIcons(
	server: Pick<ServerSummary, "icons" | "meta" | "name">,
	options?: ResolveInspectorServerIconSrcOptions,
): ServerIcon[] {
	const src = resolveInspectorServerIconSrc(server, options);
	return src ? [{ src }] : [];
}
