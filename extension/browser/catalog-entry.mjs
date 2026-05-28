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

function safeSameOriginHttpsUrl(candidate, adminOrigin) {
	if (typeof candidate !== "string" || candidate.trim() === "") {
		return "";
	}
	try {
		const adminUrl = new URL(adminOrigin);
		const url = new URL(candidate.trim(), adminUrl);
		if (adminUrl.protocol !== "https:" || url.protocol !== "https:") {
			return "";
		}
		return url.origin === adminUrl.origin ? url.toString() : "";
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
		entry?.iconUrl,
		entry?.logoUrl,
		officialIcon,
		server?.iconUrl,
		server?.logoUrl,
	];
	for (const candidate of candidates) {
		const safeUrl = safeSameOriginHttpsUrl(candidate, adminOrigin);
		if (safeUrl) return safeUrl;
	}
	return "";
}

export function clientConfigMeta(entry) {
	const config = entry?.config || {};
	const kind = config.kind || "";
	if (!kind) {
		return {
			signal: entry?.signal || entry?.category || "",
			meta: entry?.meta || "",
		};
	}
	if (kind !== "file") {
		return {
			signal: `config.kind=${kind}`,
			meta: entry?.meta || "",
		};
	}

	const paths = config.file?.paths || {};
	const pathPlatforms = Object.keys(paths).filter((platform) => Boolean(paths[platform]));
	const containerKeys = config.file?.container?.keys || [];
	const keys = Array.isArray(containerKeys) ? containerKeys : Object.keys(containerKeys);
	const metaParts = [];
	if (pathPlatforms.length > 0) {
		metaParts.push(`paths: ${pathPlatforms.slice(0, 3).join(", ")}`);
	}
	if (keys.length > 0) {
		metaParts.push(`keys: ${keys.slice(0, 3).join(", ")}`);
	}
	return {
		signal: "config.kind=file",
		meta: metaParts.join("; "),
	};
}
