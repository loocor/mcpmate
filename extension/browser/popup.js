const ADMIN_ORIGIN = globalThis.MCPMATE_EXTENSION_CONFIG.adminApiOrigin;
const DISCOVERY_MODE = globalThis.MCPMATE_EXTENSION_CONFIG.discoveryMode || "account";
const DISCOVERY_ENDPOINTS = {
	portals: `${ADMIN_ORIGIN}/discovery/portals`,
	servers: `${ADMIN_ORIGIN}/discovery/servers`,
	clients: `${ADMIN_ORIGIN}/discovery/clients`,
};
const MOCK_DISCOVERY_ENDPOINTS = {
	portals: chrome.runtime.getURL("mock/portals.json"),
	servers: chrome.runtime.getURL("mock/servers.json"),
	clients: chrome.runtime.getURL("mock/clients.json"),
};
const SETTINGS_KEY = "mcpmate.discovery.settings";
const DEFAULT_SETTINGS = {
	language: "en",
	theme: "system",
};
const COPY = {
	en: {
		title: "MCPMate",
		subtitle: "Curated MCP resources from MCPMate.",
		tabs: {
			portals: "Portals",
			servers: "Servers",
			clients: "Clients",
		},
		footer: {
			github: "GitHub",
			website: "Website",
			settings: "Settings",
			discord: "Discord",
		},
		settings: {
			language: "Language",
			theme: "Theme",
			system: "System",
			light: "Light",
			dark: "Dark",
		},
		loading: {
			portals: "Loading portal recommendations...",
			servers: "Loading server recommendations...",
			clients: "Loading client recommendations...",
		},
		empty: {
			portals: "No portal recommendations are published yet.",
			servers: "No server recommendations are published yet.",
			clients: "No client recommendations are published yet.",
		},
		unavailable: {
			portals: "Portal catalog is unavailable right now.",
			servers: "Server catalog is unavailable right now.",
			clients: "Client catalog is unavailable right now.",
		},
		mockUnavailable: {
			portals: "Mock portal catalog is unavailable right now.",
			servers: "Mock server catalog is unavailable right now.",
			clients: "Mock client catalog is unavailable right now.",
		},
		source: {
			account: "MCPMate catalog",
			mock: "Mock catalog",
		},
		visit: "Open link",
	},
	"zh-cn": {
		title: "MCPMate",
		subtitle: "来自 MCPMate 的精选 MCP 资源。",
		tabs: {
			portals: "入口",
			servers: "服务",
			clients: "客户端",
		},
		footer: {
			github: "GitHub",
			website: "官网",
			settings: "设置",
			discord: "Discord",
		},
		settings: {
			language: "语言",
			theme: "主题",
			system: "跟随系统",
			light: "浅色",
			dark: "深色",
		},
		loading: {
			portals: "正在加载入口推荐...",
			servers: "正在加载服务推荐...",
			clients: "正在加载客户端推荐...",
		},
		empty: {
			portals: "暂未发布入口推荐。",
			servers: "暂未发布服务推荐。",
			clients: "暂未发布客户端推荐。",
		},
		unavailable: {
			portals: "当前未提供入口目录。",
			servers: "当前未提供服务目录。",
			clients: "当前未提供客户端目录。",
		},
		mockUnavailable: {
			portals: "当前未提供模拟入口目录。",
			servers: "当前未提供模拟服务目录。",
			clients: "当前未提供模拟客户端目录。",
		},
		source: {
			account: "MCPMate 目录",
			mock: "模拟目录",
		},
		visit: "打开链接",
	},
	ja: {
		title: "MCPMate",
		subtitle: "MCPMate の厳選 MCP リソース。",
		tabs: {
			portals: "ポータル",
			servers: "サーバー",
			clients: "クライアント",
		},
		footer: {
			github: "GitHub",
			website: "Web サイト",
			settings: "設定",
			discord: "Discord",
		},
		settings: {
			language: "言語",
			theme: "テーマ",
			system: "システム",
			light: "ライト",
			dark: "ダーク",
		},
		loading: {
			portals: "ポータルのおすすめを読み込み中...",
			servers: "サーバーのおすすめを読み込み中...",
			clients: "クライアントのおすすめを読み込み中...",
		},
		empty: {
			portals: "公開済みのポータルおすすめはまだありません。",
			servers: "公開済みのサーバーおすすめはまだありません。",
			clients: "公開済みのクライアントおすすめはまだありません。",
		},
		unavailable: {
			portals: "ポータルカタログは現在利用できません。",
			servers: "サーバーカタログは現在利用できません。",
			clients: "クライアントカタログは現在利用できません。",
		},
		mockUnavailable: {
			portals: "モックのポータルカタログは現在利用できません。",
			servers: "モックのサーバーカタログは現在利用できません。",
			clients: "モックのクライアントカタログは現在利用できません。",
		},
		source: {
			account: "MCPMate カタログ",
			mock: "モックカタログ",
		},
		visit: "リンクを開く",
	},
};

let activeCopy = COPY.en;

function openExternalUrl(url) {
	if (chrome?.tabs?.create) {
		chrome.tabs.create({ url });
		return;
	}
	window.open(url, "_blank", "noopener,noreferrer");
}

const ICONS = {
	external:
		'<svg viewBox="0 0 24 24" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M7 17 17 7"/><path d="M7 7h10v10"/></svg>',
	github:
		'<svg viewBox="0 0 24 24" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.4 5.4 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65S8.93 17.38 9 18v4"/><path d="M9 18c-4.51 2-5-2-7-2"/></svg>',
	globe:
		'<svg viewBox="0 0 24 24" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="10"/><path d="M2 12h20"/><path d="M12 2a15.3 15.3 0 0 1 0 20"/><path d="M12 2a15.3 15.3 0 0 0 0 20"/></svg>',
	discord:
		'<svg viewBox="0 0 24 24" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M8 12.5h.01"/><path d="M16 12.5h.01"/><path d="M7.5 8.5c3-1 6-1 9 0"/><path d="M8 17c-1.3-.4-2.5-1-3.5-1.8.2-3.2 1-6.3 2.4-9.2A14 14 0 0 1 10 5l.6 1.2a12.5 12.5 0 0 1 2.8 0L14 5a14 14 0 0 1 3.1 1c1.4 2.9 2.2 6 2.4 9.2A13 13 0 0 1 16 17l-.8-1.2a9 9 0 0 1-6.4 0z"/></svg>',
	settings:
		'<svg viewBox="0 0 24 24" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 15.5A3.5 3.5 0 1 0 12 8a3.5 3.5 0 0 0 0 7.5Z"/><path d="M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06A1.7 1.7 0 0 0 15 19.4a1.7 1.7 0 0 0-1 .6V20a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1-.6 1.7 1.7 0 0 0-1.88.34l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.7 1.7 0 0 0 4.6 15a1.7 1.7 0 0 0-.6-1H4a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 .6-1 1.7 1.7 0 0 0-.34-1.88l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.7 1.7 0 0 0 9 4.6a1.7 1.7 0 0 0 1-.6V4a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 .6 1.7 1.7 0 0 0 1.88-.34l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.7 1.7 0 0 0 19.4 9c.12.36.33.69.6 1H20a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-.5 1Z"/></svg>',
};

function renderInlineIcons() {
	for (const el of document.querySelectorAll("[data-icon]")) {
		el.innerHTML = ICONS[el.dataset.icon] || "";
	}
}

function discoveryEndpoints() {
	return DISCOVERY_MODE === "mock" ? MOCK_DISCOVERY_ENDPOINTS : DISCOVERY_ENDPOINTS;
}

function discoverySourceLabel() {
	return activeCopy.source[DISCOVERY_MODE] || activeCopy.source.account;
}

function unavailableMessage(kind) {
	return DISCOVERY_MODE === "mock"
		? activeCopy.mockUnavailable[kind]
		: activeCopy.unavailable[kind];
}

function normalizeLanguage(language) {
	if (language === "zh" || language === "zh-cn") {
		return "zh-cn";
	}
	if (language === "ja") {
		return "ja";
	}
	return "en";
}

function normalizeTheme(theme) {
	if (theme === "light" || theme === "dark" || theme === "system") {
		return theme;
	}
	return "system";
}

function normalizeSettings(candidate) {
	return {
		language: normalizeLanguage(candidate?.language),
		theme: normalizeTheme(candidate?.theme),
	};
}

function storageArea() {
	return chrome?.storage?.sync || chrome?.storage?.local || null;
}

async function readSettings() {
	const area = storageArea();
	if (area) {
		const stored = await area.get(SETTINGS_KEY);
		return normalizeSettings(stored[SETTINGS_KEY] || DEFAULT_SETTINGS);
	}
	try {
		return normalizeSettings(JSON.parse(localStorage.getItem(SETTINGS_KEY) || "{}"));
	} catch {
		return DEFAULT_SETTINGS;
	}
}

async function writeSettings(settings) {
	const normalized = normalizeSettings(settings);
	const area = storageArea();
	if (area) {
		await area.set({ [SETTINGS_KEY]: normalized });
		return normalized;
	}
	localStorage.setItem(SETTINGS_KEY, JSON.stringify(normalized));
	return normalized;
}

function resolvedTheme(theme) {
	if (theme === "system") {
		return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
	}
	return theme;
}

function toolbarIconPaths(theme) {
	const suffix = resolvedTheme(theme) === "dark" ? "-dark" : "";
	return {
		16: `icons/icon${suffix}-16.png`,
		32: `icons/icon${suffix}-32.png`,
		48: `icons/icon${suffix}-48.png`,
		128: `icons/icon${suffix}-128.png`,
	};
}

function applyToolbarIcon(theme) {
	if (!chrome?.action?.setIcon) return;
	chrome.action.setIcon({ path: toolbarIconPaths(theme) });
}

function applyTheme(theme) {
	document.documentElement.dataset.theme = resolvedTheme(theme);
	applyToolbarIcon(theme);
}

function setText(id, value) {
	const el = document.getElementById(id);
	if (el) el.textContent = value;
}

function documentLanguage(language) {
	if (language === "zh-cn") {
		return "zh-CN";
	}
	if (language === "ja") {
		return "ja";
	}
	return "en";
}

function applyCopy(language) {
	activeCopy = COPY[language] || COPY.en;
	document.documentElement.lang = documentLanguage(language);
	setText("popup-title", activeCopy.title);
	setText("popup-copy", activeCopy.subtitle);
	setText("tab-portals", activeCopy.tabs.portals);
	setText("tab-servers", activeCopy.tabs.servers);
	setText("tab-clients", activeCopy.tabs.clients);
	setText("language-label", activeCopy.settings.language);
	setText("theme-label", activeCopy.settings.theme);
	setText("theme-system-option", activeCopy.settings.system);
	setText("theme-light-option", activeCopy.settings.light);
	setText("theme-dark-option", activeCopy.settings.dark);
	setButtonLabel("github-button", activeCopy.footer.github);
	setButtonLabel("website-button", activeCopy.footer.website);
	setButtonLabel("settings-button", activeCopy.footer.settings);
	setButtonLabel("discord-button", activeCopy.footer.discord);
	setText("discord-label", activeCopy.footer.discord);
	for (const button of document.querySelectorAll(".open-button")) {
		button.setAttribute("aria-label", activeCopy.visit);
		button.title = activeCopy.visit;
	}
}

function setButtonLabel(id, label) {
	const button = document.getElementById(id);
	if (!button) return;
	button.setAttribute("aria-label", label);
	button.title = label;
}

function initialBadge(name) {
	return String(name || "?")
		.trim()
		.slice(0, 2)
		.toUpperCase();
}

function renderIcon(name, iconUrl) {
	const badge = document.createElement("span");
	badge.className = "icon-badge";
	if (iconUrl) {
		const img = document.createElement("img");
		img.src = iconUrl;
		img.alt = "";
		badge.appendChild(img);
		return badge;
	}
	badge.textContent = initialBadge(name);
	return badge;
}

function card({ name, description, url, source, signal, meta, iconUrl }) {
	const el = document.createElement("article");
	el.className = "card";
	const title = document.createElement("div");
	title.className = "card-title";

	const heading = document.createElement("div");
	heading.className = "card-heading";
	heading.appendChild(renderIcon(name, iconUrl));

	const headingText = document.createElement("div");
	headingText.className = "card-heading-text";
	const label = document.createElement("span");
	label.textContent = name;
	headingText.appendChild(label);
	heading.appendChild(headingText);

	const openButton = document.createElement("button");
	openButton.type = "button";
	openButton.className = "open-button";
	openButton.setAttribute("aria-label", activeCopy.visit);
	openButton.title = activeCopy.visit;
	const icon = document.createElement("span");
	icon.className = "button-icon";
	icon.dataset.icon = "external";
	icon.innerHTML = ICONS.external;
	openButton.appendChild(icon);
	openButton.addEventListener("click", () => openExternalUrl(url));
	title.appendChild(heading);
	title.appendChild(openButton);

	const body = document.createElement("p");
	body.textContent = description;

	const metaEl = document.createElement("div");
	metaEl.className = "card-meta";
	for (const item of [signal, meta, source].filter(Boolean)) {
		const pill = document.createElement("span");
		pill.className = "pill";
		pill.textContent = item;
		metaEl.appendChild(pill);
	}

	el.appendChild(title);
	el.appendChild(body);
	el.appendChild(metaEl);
	return el;
}

function endpointStatusId(kind) {
	return `${kind.slice(0, -1)}-status`;
}

function endpointListId(kind) {
	return `${kind.slice(0, -1)}-list`;
}

function setSectionStatus(kind, text) {
	document.getElementById(endpointStatusId(kind)).textContent = text;
}

function setSectionEntries(kind, entries) {
	document.getElementById(endpointListId(kind)).replaceChildren(...entries);
}

function discoveryMeta(entry) {
	return (
		entry?._meta?.["ai.mcpmate/discovery"] ||
		entry?.metadata?.discovery ||
		entry?.server?._meta?.["ai.mcpmate/discovery"] ||
		{}
	);
}

function firstTransport(server) {
	const remote = Array.isArray(server?.remotes) ? server.remotes[0] : null;
	if (remote?.type) return remote.type;
	const pkg = Array.isArray(server?.packages) ? server.packages[0] : null;
	if (pkg?.transport?.type) return pkg.transport.type;
	return "";
}

function entryUrl(entry) {
	const server = entry?.server || entry;
	const official = entry?.official || server?.official || {};
	const curated = entry?.curated || server?.curated || {};
	return (
		entry?.url ||
		entry?.homepageUrl ||
		curated.docsUrl ||
		curated.supportUrl ||
		official.websiteUrl ||
		official.repository?.url ||
		official.docsUrl ||
		server?.websiteUrl ||
		server?.homepageUrl ||
		server?.repository?.url ||
		server?.docsUrl ||
		"https://mcp.umate.ai"
	);
}

function iconUrl(entry) {
	const server = entry?.server || entry;
	const official = entry?.official || server?.official || {};
	const meta = discoveryMeta(entry);
	const officialIcon = Array.isArray(official.icons) ? official.icons[0]?.src : "";
	return (
		meta.iconUrl ||
		meta.brandIconUrl ||
		entry?.iconUrl ||
		entry?.logoUrl ||
		officialIcon ||
		server?.iconUrl ||
		server?.logoUrl ||
		""
	);
}

function normalizeEntries(kind, data) {
	if (Array.isArray(data)) return data;
	if (Array.isArray(data?.[kind])) return data[kind];
	if (kind === "servers" && Array.isArray(data?.items)) return data.items;
	return [];
}

function sectionSummary(kind, count) {
	return `${discoverySourceLabel()} · ${count} ${kind}`;
}

function serverCategories(curated, meta) {
	if (Array.isArray(curated.categories)) {
		return curated.categories.slice(0, 2).join(", ");
	}
	if (Array.isArray(meta.categories)) {
		return meta.categories.slice(0, 2).join(", ");
	}
	return "";
}

function entryMeta(entry, kind) {
	const server = entry?.server || entry;
	const official = entry?.official || server?.official || {};
	const curated = entry?.curated || server?.curated || {};
	const meta = discoveryMeta(entry);
	if (kind === "servers") {
		const categories = serverCategories(curated, meta);
		const score =
			typeof meta.rating?.score === "number" ? `Score ${meta.rating.score}` : "";
		return {
			signal: curated.recommendationTier || meta.quality?.status || score,
			meta: categories || firstTransport(official) || firstTransport(server),
		};
	}
	if (kind === "clients") {
		return {
			signal: entry?.signal || entry?.category || "",
			meta: entry?.meta || entry?.config?.kind || "",
		};
	}
	return {
		signal: entry?.signal || meta.quality?.status || "",
		meta: entry?.meta || meta.category || meta.platform || "",
	};
}

function entryName(entry, kind) {
	const server = entry?.server || entry;
	const official = entry?.official || server?.official || {};
	const curated = entry?.curated || server?.curated || {};
	if (kind === "servers") {
		return (
			curated.displayName ||
			official.title ||
			official.name ||
			server?.title ||
			server?.name ||
			"Untitled"
		);
	}
	if (kind === "clients") {
		return entry?.displayName || entry?.title || entry?.identifier || "Untitled";
	}
	return entry?.title || server?.title || server?.name || "Untitled";
}

function entryDescription(entry, kind) {
	const server = entry?.server || entry;
	const official = entry?.official || server?.official || {};
	const curated = entry?.curated || server?.curated || {};
	if (kind === "servers") {
		return curated.summary || official.description || server?.description || "Curated server entry.";
	}
	if (kind === "clients") {
		return entry?.description || `Curated ${kind.slice(0, -1)} entry.`;
	}
	return entry?.description || server?.description || `Curated ${kind.slice(0, -1)} entry.`;
}

function entrySource(entry) {
	const server = entry?.server || entry;
	const official = entry?.official || server?.official || {};
	return (
		entry?.source ||
		official.repository?.source ||
		server?.repository?.source ||
		discoveryMeta(entry).source ||
		discoverySourceLabel()
	);
}

function entryCard(kind, entry) {
	const metaBits = entryMeta(entry, kind);
	return card({
		name: entryName(entry, kind),
		description: entryDescription(entry, kind),
		url: entryUrl(entry),
		source: entrySource(entry),
		signal: metaBits.signal,
		meta: metaBits.meta,
		iconUrl: iconUrl(entry),
	});
}

async function renderSection(kind) {
	setSectionStatus(kind, activeCopy.loading[kind]);
	setSectionEntries(kind, []);
	const response = await fetch(discoveryEndpoints()[kind], {
		headers: { accept: "application/json" },
	});
	if (!response.ok) {
		throw new Error(`${kind}:${response.status}`);
	}
	const data = await response.json();
	const entries = normalizeEntries(kind, data);
	if (entries.length === 0) {
		setSectionStatus(kind, activeCopy.empty[kind]);
		return;
	}
	setSectionStatus(kind, sectionSummary(kind, entries.length));
	setSectionEntries(
		kind,
		entries.map((entry) => entryCard(kind, entry)),
	);
}

function activatePanel(panelName) {
	for (const tab of document.querySelectorAll("[data-panel-target]")) {
		tab.classList.toggle("is-active", tab.dataset.panelTarget === panelName);
	}
	for (const panel of document.querySelectorAll(".panel")) {
		panel.classList.toggle("is-active", panel.dataset.panel === panelName);
	}
	document
		.getElementById("settings-button")
		.classList.toggle("is-active", panelName === "settings");
	const content = document.getElementById("content-area");
	if (content) content.scrollTop = 0;
}

document.addEventListener("DOMContentLoaded", async () => {
	renderInlineIcons();
	let settings = await readSettings();
	const languageSelect = document.getElementById("language-select");
	const themeSelect = document.getElementById("theme-select");
	languageSelect.value = settings.language;
	themeSelect.value = settings.theme;
	applyCopy(settings.language);
	applyTheme(settings.theme);

	for (const kind of ["portals", "servers", "clients"]) {
		renderSection(kind).catch(() => {
			setSectionStatus(kind, unavailableMessage(kind));
		});
	}

	async function persist(nextSettings) {
		settings = await writeSettings(nextSettings);
		applyCopy(settings.language);
		applyTheme(settings.theme);
	}

	languageSelect.addEventListener("change", () =>
		persist({ ...settings, language: languageSelect.value }),
	);
	themeSelect.addEventListener("change", () =>
		persist({ ...settings, theme: themeSelect.value }),
	);

	for (const tab of document.querySelectorAll("[data-panel-target]")) {
		tab.addEventListener("click", () => activatePanel(tab.dataset.panelTarget));
	}
	for (const button of document.querySelectorAll("[data-open-url]")) {
		button.addEventListener("click", () => openExternalUrl(button.dataset.openUrl));
	}
	document
		.getElementById("settings-button")
		.addEventListener("click", () => activatePanel("settings"));
	window
		.matchMedia("(prefers-color-scheme: dark)")
		.addEventListener("change", () => applyTheme(settings.theme));
});
