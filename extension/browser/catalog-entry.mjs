function discoveryMeta(entry) {
	return (
		entry?._meta?.["ai.mcpmate/discovery"] ||
		entry?.metadata?.discovery ||
		entry?.server?._meta?.["ai.mcpmate/discovery"] ||
		{}
	);
}

const DEFAULT_ENTRY_URL = "https://mcp.umate.ai";

function safeHttpUrl(candidate) {
	if (typeof candidate !== "string" || candidate.trim() === "") {
		return "";
	}
	try {
		const url = new URL(candidate.trim());
		return url.protocol === "http:" || url.protocol === "https:" ? url.toString() : "";
	} catch {
		return "";
	}
}

const RasterDataImagePattern = /^data:image\/(?:png|jpe?g|webp|gif);base64,[A-Za-z0-9+/=]+$/i;

function safeImageUrl(candidate, adminOrigin) {
	if (typeof candidate !== "string" || candidate.trim() === "") {
		return "";
	}
	const trimmed = candidate.trim();
	if (RasterDataImagePattern.test(trimmed)) {
		return trimmed;
	}
	try {
		const adminUrl = new URL(adminOrigin);
		const url = new URL(trimmed, adminUrl);
		if (url.protocol !== "https:") {
			return "";
		}
		return url.toString();
	} catch {
		return "";
	}
}

export function entryUrl(entry, fallbackUrl = DEFAULT_ENTRY_URL) {
	const server = entry?.server || entry;
	const links = entry?.links || server?.links || {};
	const official = entry?.official || server?.official || {};
	const curated = entry?.curated || server?.curated || {};
	const candidates = [
		links.homepage,
		links.docs,
		links.support,
		entry?.url,
		entry?.homepageUrl,
		curated.docsUrl,
		curated.supportUrl,
		official.websiteUrl,
		official.repository?.url,
		official.docsUrl,
		server?.websiteUrl,
		server?.homepageUrl,
		server?.repository?.url,
		server?.docsUrl,
	];
	for (const candidate of candidates) {
		const safeUrl = safeHttpUrl(candidate);
		if (safeUrl) return safeUrl;
	}
	if (fallbackUrl === DEFAULT_ENTRY_URL) {
		return DEFAULT_ENTRY_URL;
	}
	return safeHttpUrl(fallbackUrl) || DEFAULT_ENTRY_URL;
}

export function iconUrl(entry, adminOrigin) {
	const server = entry?.server || entry;
	const official = entry?.official || server?.official || {};
	const meta = discoveryMeta(entry);
	const officialIcon = Array.isArray(official.icons) ? official.icons[0]?.src : "";
	const candidates = [
		entry?.icon?.url,
		server?.icon?.url,
		meta.iconUrl,
		meta.brandIconUrl,
		typeof entry?.icon === "string" ? entry.icon : "",
		typeof server?.icon === "string" ? server.icon : "",
		entry?.iconUrl,
		entry?.logoUrl,
		officialIcon,
		server?.iconUrl,
		server?.logoUrl,
	];
	for (const candidate of candidates) {
		const safeUrl = safeImageUrl(candidate, adminOrigin);
		if (safeUrl) return safeUrl;
	}
	return "";
}

function firstTextValue(values) {
	for (const value of values) {
		if (typeof value === "string" && value.trim()) {
			return value.trim();
		}
	}
	return "";
}

function firstArrayValue(values) {
	for (const value of values) {
		if (Array.isArray(value)) {
			const text = firstTextValue(value);
			if (text) return text;
		}
	}
	return "";
}

export function clientCatalogMeta(entry) {
	const meta = discoveryMeta(entry);
	return {
		signal:
			firstArrayValue([entry?.tags, entry?.categories, meta.categories]) ||
			firstTextValue([entry?.category, entry?.type, meta.category]),
		meta: "",
	};
}
