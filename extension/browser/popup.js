import {
	clientCatalogMeta,
	entryUrl,
	iconUrl,
} from "./catalog-entry.mjs";
import { communityFooterForLanguage } from "./community-links.mjs";
import {
	clearDiscoveryCacheForKind,
	clearSessionSnapshots,
	isCacheEntryFresh,
	pruneExpiredDiscoveryCaches,
	readDiscoveryCacheData,
	readDiscoveryCacheEntry,
	readSessionSnapshot,
	writeDiscoveryCache,
	writeSessionSnapshot,
} from "./discovery-cache.mjs";
import {
	discoveryAcceptLanguage,
	discoveryLocaleFromLanguage,
	extensionLanguageFromBrowser,
} from "./discovery-locale.mjs";
import {
	DISCOVERY_PAGE_SIZE,
	buildDiscoveryUrl,
	discoveryPageState,
	discoveryQueryForPage,
	entriesForPageRender,
	isPageableDiscoveryKind,
	nextDiscoveryPageState,
	responseMetadata,
	shouldClearEntriesBeforeLoad,
	shouldRenderPanel,
	shouldStartPullRefresh,
} from "./discovery-state.mjs";

const ADMIN_ORIGIN = globalThis.MCPMATE_EXTENSION_CONFIG.adminApiOrigin;
const BUILD_DATE = globalThis.MCPMATE_EXTENSION_BUILD.buildDate;
const DISCOVERY_MODE = globalThis.MCPMATE_EXTENSION_CONFIG.discoveryMode || "account";
const DISCOVERY_ENDPOINTS = {
	portals: `${ADMIN_ORIGIN}/discovery/portals`,
	servers: `${ADMIN_ORIGIN}/discovery/servers`,
	clients: `${ADMIN_ORIGIN}/discovery/clients`,
};

function extensionResourceUrl(relativePath) {
	if (typeof chrome?.runtime?.getURL === "function") {
		return chrome.runtime.getURL(relativePath);
	}
	const base = globalThis.location.pathname.includes("/dev/")
		? new URL("../", globalThis.location.href)
		: new URL("./", globalThis.location.href);
	return new URL(relativePath, base).href;
}

function mockDiscoveryEndpoints() {
	return {
		portals: extensionResourceUrl("mock/portals.json"),
		servers: extensionResourceUrl("mock/servers.json"),
		clients: extensionResourceUrl("mock/clients.json"),
	};
}
const SETTINGS_KEY = "mcpmate.discovery.settings";
const PULL_REFRESH_THRESHOLD = 56;
const DEFAULT_SETTINGS = {
	language: "en",
	theme: "system",
};
let currentSettings = { ...DEFAULT_SETTINGS };

function activeDiscoveryLocale() {
	return discoveryLocaleFromLanguage(currentSettings.language);
}
const COPY = {
	en: {
		title: "MCPMate",
		subtitle: "Your progressive MCP management partner",
		tabs: {
			portals: "Portals",
			servers: "Servers",
			clients: "Clients",
		},
		footer: {
			github: "GitHub",
			website: "Website",
			settings: "Settings",
			community: "Discord",
		},
		actions: {
			refresh: "Refresh",
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
		loadingMore: "Loading more...",
		pullRefresh: "Pull to refresh",
		releaseRefresh: "Release to refresh",
		refreshing: "Refreshing...",
	},
	"zh-cn": {
		title: "MCPMate",
		subtitle: "你的渐进式 MCP 管理伙伴",
		tabs: {
			portals: "入口",
			servers: "服务",
			clients: "客户端",
		},
		footer: {
			github: "GitHub",
			website: "官网",
			settings: "设置",
			community: "飞书社群",
		},
		actions: {
			refresh: "刷新",
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
		loadingMore: "正在加载更多...",
		pullRefresh: "下拉刷新",
		releaseRefresh: "松开刷新",
		refreshing: "正在刷新...",
	},
	ja: {
		title: "MCPMate",
		subtitle: "育てながら使う MCP 管理パートナー",
		tabs: {
			portals: "ポータル",
			servers: "サーバー",
			clients: "クライアント",
		},
		footer: {
			github: "GitHub",
			website: "Web サイト",
			settings: "設定",
			community: "Discord",
		},
		actions: {
			refresh: "更新",
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
		loadingMore: "さらに読み込み中...",
		pullRefresh: "引いて更新",
		releaseRefresh: "離して更新",
		refreshing: "更新中...",
	},
};

let activeCopy = COPY.en;
let activePanelName = "servers";
const discoveryStates = new Map();
let paginationObserver = null;

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
	feishu:
		'<svg viewBox="0 0 152.43 121.72" fill="none" aria-hidden="true"><path d="m59.72 78.46c10.91 5.21 22.6 9.68 34.96 12.41 9.16 2.02 18.42.19 26.07-4.59 2.1-1.31 3.48-3.13 6.17-3.19-27.19 39.64-82.54 50.85-123.14 23.88-2.49-1.44-3.78-4.35-3.78-6.13v-65.92c18.29 19.15 37.71 32.95 59.72 43.54z" fill="#3570fa"/><path d="m114.54 36.97c-15.74 4.73-23.4 15.72-35.31 26.5-14.16-24.41-33.12-45.28-56.81-63.47h71.87c10.51 10.59 16.27 23.5 20.24 36.97z" fill="#06d4b9"/><path d="m126.92 83.09c-2.69.06-4.07 1.88-6.17 3.19-7.65 4.78-16.91 6.62-26.07 4.59-12.36-2.73-24.05-7.21-34.96-12.41 7.37-4.17 13.47-9.52 19.5-14.99 11.91-10.78 19.57-21.77 35.31-26.5 12.29-3.7 25.56-3.01 37.89 2.95-11.65 12.95-15.3 28.29-25.51 43.17z" fill="#143d99"/></svg>',
	refresh:
		'<svg viewBox="0 0 24 24" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 12a9 9 0 0 1-15 6.7L3 16"/><path d="M3 21v-5h5"/><path d="M3 12a9 9 0 0 1 15-6.7L21 8"/><path d="M21 3v5h-5"/></svg>',
	settings:
		'<svg viewBox="0 0 24 24" fill="none" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 15.5A3.5 3.5 0 1 0 12 8a3.5 3.5 0 0 0 0 7.5Z"/><path d="M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06A1.7 1.7 0 0 0 15 19.4a1.7 1.7 0 0 0-1 .6V20a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1-.6 1.7 1.7 0 0 0-1.88.34l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.7 1.7 0 0 0 4.6 15a1.7 1.7 0 0 0-.6-1H4a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 .6-1 1.7 1.7 0 0 0-.34-1.88l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.7 1.7 0 0 0 9 4.6a1.7 1.7 0 0 0 1-.6V4a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 .6 1.7 1.7 0 0 0 1.88-.34l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.7 1.7 0 0 0 19.4 9c.12.36.33.69.6 1H20a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-.5 1Z"/></svg>',
};

function renderInlineIcons() {
	for (const el of document.querySelectorAll("[data-icon]")) {
		if (el.closest("#community-button")) continue;
		el.innerHTML = ICONS[el.dataset.icon] || "";
	}
}

function discoveryEndpoints() {
	return DISCOVERY_MODE === "mock" ? mockDiscoveryEndpoints() : DISCOVERY_ENDPOINTS;
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

function discoveryContext() {
	return { mode: DISCOVERY_MODE, origin: ADMIN_ORIGIN };
}

function initialSettingsFromBrowser() {
	return {
		...DEFAULT_SETTINGS,
		language: extensionLanguageFromBrowser(),
	};
}

async function readSettings() {
	const area = storageArea();
	if (area) {
		const stored = await area.get(SETTINGS_KEY);
		if (Object.prototype.hasOwnProperty.call(stored, SETTINGS_KEY)) {
			return normalizeSettings(stored[SETTINGS_KEY]);
		}
		return normalizeSettings(initialSettingsFromBrowser());
	}
	try {
		const raw = localStorage.getItem(SETTINGS_KEY);
		if (raw) {
			return normalizeSettings(JSON.parse(raw));
		}
		return normalizeSettings(initialSettingsFromBrowser());
	} catch {
		return normalizeSettings(initialSettingsFromBrowser());
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
	try {
		chrome.storage?.local?.set({ "mcpmate.toolbarTheme": resolvedTheme(theme) === "dark" ? "dark" : "light" });
	} catch { }
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

function applyCommunityFooter(language) {
	const copy = COPY[language] || COPY.en;
	const { iconKey, href } = communityFooterForLanguage(language);
	const button = document.getElementById("community-button");
	if (!button) return;
	button.dataset.openUrl = href;
	button.classList.toggle("is-feishu", iconKey === "feishu");
	setButtonLabel("community-button", copy.footer.community);
	setText("community-label", copy.footer.community);
	const iconEl = button.querySelector("[data-icon]");
	if (iconEl) {
		iconEl.dataset.icon = iconKey;
		iconEl.innerHTML = ICONS[iconKey] || "";
	}
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
	setButtonLabel("refresh-button", activeCopy.actions.refresh);
	applyCommunityFooter(language);
	setText("build-date", BUILD_DATE);
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
		.slice(0, 1)
		.toUpperCase();
}

function renderFallbackIcon(badge, name) {
	badge.replaceChildren(document.createTextNode(initialBadge(name)));
}

function renderIcon(name, iconUrl) {
	const badge = document.createElement("span");
	badge.className = "icon-badge";
	if (iconUrl) {
		const img = document.createElement("img");
		img.alt = "";
		img.loading = "lazy";
		img.decoding = "async";
		img.referrerPolicy = "no-referrer";
		img.addEventListener("error", () => renderFallbackIcon(badge, name), {
			once: true,
		});
		img.src = iconUrl;
		badge.appendChild(img);
		return badge;
	}
	renderFallbackIcon(badge, name);
	return badge;
}

function entryCardProps(kind, entry) {
	const metaBits = entryMeta(entry, kind);
	return {
		name: entryName(entry, kind),
		description: entryDescription(entry, kind),
		url: entryUrl(entry),
		source: entrySource(entry),
		signal: metaBits.signal,
		meta: metaBits.meta,
		iconUrl: iconUrl(entry, ADMIN_ORIGIN),
	};
}

function createOpenButton(url) {
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
	return openButton;
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

	title.appendChild(heading);
	title.appendChild(createOpenButton(url));

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
	if (metaEl.childElementCount > 0) {
		el.appendChild(metaEl);
	}
	return el;
}

function compactCard({ name, description, url, iconUrl }) {
	const el = document.createElement("article");
	el.className = "card card--compact";
	el.tabIndex = 0;
	el.setAttribute("role", "link");
	el.addEventListener("click", (event) => {
		if (event.target.closest(".open-button")) return;
		openExternalUrl(url);
	});
	el.addEventListener("keydown", (event) => {
		if (event.key !== "Enter" && event.key !== " ") return;
		event.preventDefault();
		openExternalUrl(url);
	});
	el.appendChild(renderIcon(name, iconUrl));

	const copy = document.createElement("div");
	copy.className = "card-compact-copy";
	const nameEl = document.createElement("div");
	nameEl.className = "card-compact-name";
	nameEl.textContent = name;
	const descriptionEl = document.createElement("div");
	descriptionEl.className = "card-compact-description";
	descriptionEl.textContent = description;
	copy.appendChild(nameEl);
	copy.appendChild(descriptionEl);
	el.appendChild(copy);
	el.appendChild(createOpenButton(url));
	return el;
}

function createFeaturedCarousel(kind, entries) {
	const carousel = document.createElement("div");
	carousel.className = "featured-carousel";
	const track = document.createElement("div");
	track.className = "featured-carousel-track";

	const buildSlide = (entry) => {
		const cardEl = entryCard(kind, entry);
		cardEl.classList.add("card--featured");
		return cardEl;
	};

	const slides =
		entries.length > 1
			? [
				buildSlide(entries[entries.length - 1]),
				...entries.map((entry) => buildSlide(entry)),
				buildSlide(entries[0]),
			]
			: entries.map((entry) => buildSlide(entry));

	for (const slide of slides) {
		track.appendChild(slide);
	}
	carousel.appendChild(track);

	if (entries.length > 1) {
		const dots = document.createElement("div");
		dots.className = "featured-carousel-dots";
		const realCount = entries.length;
		let jumping = false;
		let scrollEndTimer = null;

		const slideStride = () => {
			const slide = track.querySelector(".card--featured");
			if (!slide) return 0;
			const styles = getComputedStyle(track);
			const gap = Number.parseFloat(styles.columnGap || styles.gap) || 0;
			return slide.getBoundingClientRect().width + gap;
		};

		const getSlideIndex = () => {
			const stride = slideStride();
			if (!stride) return null;
			return Math.round(track.scrollLeft / stride);
		};

		const jumpToSlideIndex = (index) => {
			const stride = slideStride();
			if (!stride) return;
			track.classList.add("is-teleporting");
			track.scrollLeft = stride * index;
			void track.offsetHeight;
			track.classList.remove("is-teleporting");
		};

		const goToSlideIndex = (index, smooth) => {
			const stride = slideStride();
			if (!stride) return;
			track.scrollTo({
				left: stride * index,
				behavior: smooth ? "smooth" : "auto",
			});
		};

		const syncDots = (realIndex) => {
			for (const [dotIndex, dot] of [...dots.children].entries()) {
				dot.classList.toggle("is-active", dotIndex === realIndex);
			}
		};

		const getActiveRealIndex = () => {
			const index = getSlideIndex();
			if (index === null) return 0;
			if (index <= 0) return realCount - 1;
			if (index >= realCount + 1) return 0;
			return index - 1;
		};

		const finishJump = (physicalIndex, realIndex) => {
			jumping = true;
			jumpToSlideIndex(physicalIndex);
			syncDots(realIndex);
			requestAnimationFrame(() => {
				jumping = false;
			});
		};

		const teleportIfNeeded = () => {
			clearTimeout(scrollEndTimer);
			scrollEndTimer = null;
			if (jumping) return;
			const index = getSlideIndex();
			if (index === null) return;
			if (index <= 0) {
				finishJump(realCount, realCount - 1);
				return;
			}
			if (index >= realCount + 1) {
				finishJump(1, 0);
			}
		};

		const scheduleTeleportCheck = () => {
			clearTimeout(scrollEndTimer);
			scrollEndTimer = setTimeout(teleportIfNeeded, 140);
		};

		const goToRealIndex = (targetRealIndex, smooth) => {
			if (jumping) return;
			const currentReal = getActiveRealIndex();
			const targetPhysical = targetRealIndex + 1;

			if (!smooth) {
				jumpToSlideIndex(targetPhysical);
				syncDots(targetRealIndex);
				return;
			}

			if (targetRealIndex === currentReal) {
				goToSlideIndex(targetPhysical, true);
				return;
			}

			const forwardSteps = (targetRealIndex - currentReal + realCount) % realCount;
			const backwardSteps = (currentReal - targetRealIndex + realCount) % realCount;

			if (forwardSteps > 0 && forwardSteps < backwardSteps) {
				goToSlideIndex(realCount + 1, true);
				return;
			}
			if (backwardSteps > 0 && backwardSteps < forwardSteps) {
				goToSlideIndex(0, true);
				return;
			}

			goToSlideIndex(targetPhysical, true);
		};

		entries.forEach((_, index) => {
			const dot = document.createElement("button");
			dot.type = "button";
			dot.className = `featured-carousel-dot${index === 0 ? " is-active" : ""}`;
			dot.setAttribute("aria-label", `Featured slide ${index + 1}`);
			dot.addEventListener("click", () => goToRealIndex(index, true));
			dots.appendChild(dot);
		});

		track.addEventListener("scroll", () => {
			if (!jumping) syncDots(getActiveRealIndex());
			scheduleTeleportCheck();
		}, { passive: true });

		track.addEventListener("scrollend", teleportIfNeeded, { passive: true });

		carousel.appendChild(dots);
		jumping = true;
		requestAnimationFrame(() => {
			jumpToSlideIndex(1);
			syncDots(0);
			requestAnimationFrame(() => {
				jumping = false;
			});
		});
	}

	return carousel;
}

function createEntryListContent(kind, entries) {
	const fragment = document.createDocumentFragment();
	if (isPageableDiscoveryKind(kind)) {
		const featured = entries.slice(0, 3);
		const rest = entries.slice(3);
		if (featured.length > 0) {
			fragment.appendChild(createFeaturedCarousel(kind, featured));
		}
		if (rest.length > 0) {
			const compactList = document.createElement("div");
			compactList.className = "compact-list";
			for (const entry of rest) {
				compactList.appendChild(entryCompactCard(kind, entry));
			}
			fragment.appendChild(compactList);
		}
		return fragment;
	}
	for (const entry of entries) {
		fragment.appendChild(entryCard(kind, entry));
	}
	return fragment;
}

function endpointStatusId(kind) {
	return `${kind.slice(0, -1)}-status`;
}

function endpointListId(kind) {
	return `${kind.slice(0, -1)}-list`;
}

function endpointFooterId(kind) {
	return `${kind.slice(0, -1)}-footer`;
}

function endpointSentinelId(kind) {
	return `${kind.slice(0, -1)}-sentinel`;
}

function setSectionStatus(kind, text) {
	document.getElementById(endpointStatusId(kind)).textContent = text;
}

function createEntryCardsFragment(kind, entries) {
	return createEntryListContent(kind, entries);
}

function setSectionEntries(kind, entries, { append = false, appendOnly = false } = {}) {
	const list = document.getElementById(endpointListId(kind));
	if (appendOnly) {
		appendCompactEntries(kind, entries, list);
		return;
	}
	const cards = createEntryCardsFragment(kind, entries);
	if (append) {
		list.appendChild(cards);
		return;
	}
	list.replaceChildren(cards);
}

function appendCompactEntries(kind, entries, list = document.getElementById(endpointListId(kind))) {
	if (!list || entries.length === 0) return;
	const allEntries = sectionState(kind).entries;
	if (allEntries.length <= 3) {
		setSectionEntries(kind, entriesForPageRender(sectionState(kind)));
		return;
	}
	let compactList = list.querySelector(".compact-list");
	if (!compactList) {
		setSectionEntries(kind, entriesForPageRender(sectionState(kind)));
		return;
	}
	const fragment = document.createDocumentFragment();
	for (const entry of entries) {
		fragment.appendChild(entryCompactCard(kind, entry));
	}
	compactList.appendChild(fragment);
}

function discoveryEntriesSignature(kind, entries) {
	return entries
		.map((entry) => `${entryName(entry, kind)}:${entryUrl(entry)}`)
		.join("|");
}

function restorePanelScroll(scrollTop) {
	if (!Number.isFinite(scrollTop) || scrollTop <= 0) return;
	requestAnimationFrame(() => {
		const content = document.getElementById("content-area");
		if (content) content.scrollTop = scrollTop;
	});
}

async function persistSessionSnapshot(kind) {
	const state = sectionState(kind);
	if (!state.loaded || state.entries.length === 0) return;
	const content = document.getElementById("content-area");
	await writeSessionSnapshot(discoveryContext(), kind, activeDiscoveryLocale(), {
		state,
		scrollTop: content?.scrollTop ?? 0,
	});
}

function buildDiscoveryStateFromData(kind, data, { offset, limit, reset, current }) {
	const entries = normalizeEntries(kind, data);
	const page = discoveryPageState({
		kind,
		entries,
		metadata: responseMetadata(data),
		limit,
		offset,
	});
	const next = nextDiscoveryPageState(reset ? blankDiscoveryState() : current, page, { reset });
	const catalogGeneratedAt =
		typeof data?.generatedAt === "string" ? data.generatedAt : next.catalogGeneratedAt;
	return {
		...next,
		catalogGeneratedAt,
		loaded: true,
		loading: false,
	};
}

function renderDiscoveryState(kind, state, { appendOnlyEntries = null } = {}) {
	if (state.entries.length === 0) {
		setSectionStatus(kind, activeCopy.empty[kind]);
		setSectionEntries(kind, []);
		setSectionFooter(kind, "");
		return;
	}
	setSectionStatus(kind, "");
	if (appendOnlyEntries?.length) {
		setSectionEntries(kind, appendOnlyEntries, { appendOnly: true });
	} else {
		setSectionEntries(kind, entriesForPageRender(state));
	}
	setSectionFooter(kind, "");
}

async function tryRenderFromSessionOrCache(kind, { limit, locale }) {
	const session = await readSessionSnapshot(discoveryContext(), kind, locale);
	if (session) {
		discoveryStates.set(kind, { ...session.state, loading: false });
		renderDiscoveryState(kind, session.state);
		restorePanelScroll(session.scrollTop);
		return true;
	}

	const requestUrl = discoveryRequestUrl(kind, { limit, offset: 0, locale });
	const cached = await readDiscoveryCacheEntry(discoveryContext(), kind, requestUrl);
	if (!isCacheEntryFresh(cached)) {
		return false;
	}

	const state = buildDiscoveryStateFromData(kind, cached.data, {
		offset: 0,
		limit,
		reset: true,
		current: blankDiscoveryState(),
	});
	discoveryStates.set(kind, state);
	renderDiscoveryState(kind, state);
	return true;
}

async function fetchDiscoveryFromNetwork(kind, requestUrl, locale) {
	const response = await fetch(requestUrl, {
		credentials: "omit",
		headers: {
			accept: "application/json",
			...(locale ? { "Accept-Language": discoveryAcceptLanguage(locale) } : {}),
		},
	});
	if (!response.ok) {
		throw new Error(`${kind}:${response.status}`);
	}
	return response.json();
}

async function prefetchDiscoveryPage(kind, { offset = 0 } = {}) {
	if (sectionLoaded(kind) && offset === 0) return;
	const locale = activeDiscoveryLocale();
	const requestUrl = discoveryRequestUrl(kind, {
		limit: DISCOVERY_PAGE_SIZE,
		offset,
		locale,
	});
	const cached = await readDiscoveryCacheEntry(discoveryContext(), kind, requestUrl);
	if (isCacheEntryFresh(cached)) return;
	try {
		const data = await fetchDiscoveryFromNetwork(kind, requestUrl, locale);
		await writeDiscoveryCache(discoveryContext(), kind, requestUrl, data);
	} catch {
		// Prefetch is best-effort.
	}
}

function scheduleDiscoveryPrefetch(kind) {
	const run = () => {
		for (const panelKind of ["portals", "servers", "clients"]) {
			if (panelKind === kind) continue;
			void prefetchDiscoveryPage(panelKind);
		}
		const state = sectionState(kind);
		if (state.hasMore && Number.isFinite(state.nextOffset)) {
			void prefetchDiscoveryPage(kind, { offset: state.nextOffset });
		}
	};
	if ("requestIdleCallback" in window) {
		requestIdleCallback(run, { timeout: 2000 });
	} else {
		setTimeout(run, 250);
	}
}

function saveActivePanelSnapshot() {
	if (activePanelName === "settings" || !sectionLoaded(activePanelName)) return;
	void persistSessionSnapshot(activePanelName);
}

function sectionLoaded(kind) {
	return sectionState(kind).loaded;
}

function setSectionFooter(kind, text) {
	const footer = document.getElementById(endpointFooterId(kind));
	if (footer) footer.textContent = text;
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

function normalizeEntries(kind, data) {
	if (Array.isArray(data)) return data;
	if (Array.isArray(data?.[kind])) return data[kind];
	if (kind === "servers" && Array.isArray(data?.items)) return data.items;
	return [];
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
		return clientCatalogMeta(entry);
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
	return card(entryCardProps(kind, entry));
}

function entryCompactCard(kind, entry) {
	return compactCard(entryCardProps(kind, entry));
}

function discoveryRequestUrl(kind, { limit, offset, locale }) {
	const endpoint = discoveryEndpoints()[kind];
	if (DISCOVERY_MODE === "mock") {
		return endpoint;
	}
	return buildDiscoveryUrl(
		endpoint,
		discoveryQueryForPage({ kind, limit, offset, locale }),
	);
}

async function fetchDiscoveryData(kind, { limit, offset, bypassCache = false, locale, catalogGeneratedAt, forceNetwork = false }) {
	const requestUrl = discoveryRequestUrl(kind, { limit, offset, locale });
	if (!bypassCache && !forceNetwork) {
		const data = await readDiscoveryCacheData(
			discoveryContext(),
			kind,
			requestUrl,
			catalogGeneratedAt,
		);
		if (data) return data;
	}
	const data = await fetchDiscoveryFromNetwork(kind, requestUrl, locale);
	await writeDiscoveryCache(discoveryContext(), kind, requestUrl, data);
	return data;
}

function blankDiscoveryState() {
	return {
		entries: [],
		hasMore: false,
		nextOffset: 0,
		catalogGeneratedAt: null,
		loaded: false,
		loading: false,
	};
}

function sectionState(kind) {
	if (!discoveryStates.has(kind)) {
		discoveryStates.set(kind, blankDiscoveryState());
	}
	return discoveryStates.get(kind);
}

async function loadDiscoveryPage(kind, { reset = false, bypassCache = false } = {}) {
	const current = sectionState(kind);
	if (current.loading) return;
	if (!reset && (!current.loaded || !current.hasMore)) return;

	const offset = reset ? 0 : current.nextOffset;
	const limit = DISCOVERY_PAGE_SIZE;
	const locale = activeDiscoveryLocale();
	const shouldClearEntries = shouldClearEntriesBeforeLoad(current, { reset });
	const previousCount = current.entries.length;
	const previousSignature = discoveryEntriesSignature(kind, current.entries);

	if (bypassCache) {
		await clearDiscoveryCacheForKind(discoveryContext(), kind);
		if (reset) {
			await clearSessionSnapshots(discoveryContext(), locale);
		}
	}

	let renderedFromFastPath = false;
	if (reset && !bypassCache) {
		renderedFromFastPath = await tryRenderFromSessionOrCache(kind, { limit, locale });
	}

	discoveryStates.set(kind, { ...sectionState(kind), loading: true });
	if (reset) {
		if (!renderedFromFastPath) {
			setSectionStatus(kind, activeCopy.loading[kind]);
			if (shouldClearEntries) {
				setSectionEntries(kind, []);
			}
		}
		setSectionFooter(kind, "");
	} else {
		setSectionFooter(kind, activeCopy.loadingMore);
	}

	try {
		const data = await fetchDiscoveryData(kind, {
			limit,
			offset,
			bypassCache,
			locale,
			catalogGeneratedAt: reset ? null : current.catalogGeneratedAt,
			forceNetwork: reset && renderedFromFastPath,
		});

		if (reset && renderedFromFastPath) {
			const freshFirstPage = buildDiscoveryStateFromData(kind, data, {
				offset: 0,
				limit,
				reset: true,
				current: blankDiscoveryState(),
			});
			const existing = sectionState(kind);
			const catalogChanged =
				Boolean(freshFirstPage.catalogGeneratedAt) &&
				Boolean(existing.catalogGeneratedAt) &&
				freshFirstPage.catalogGeneratedAt !== existing.catalogGeneratedAt;
			const firstPageChanged =
				discoveryEntriesSignature(kind, existing.entries.slice(0, limit)) !==
				discoveryEntriesSignature(kind, freshFirstPage.entries);

			if (catalogChanged || firstPageChanged) {
				discoveryStates.set(kind, freshFirstPage);
				renderDiscoveryState(kind, freshFirstPage);
				await persistSessionSnapshot(kind);
			} else {
				discoveryStates.set(kind, {
					...existing,
					loading: false,
					catalogGeneratedAt: freshFirstPage.catalogGeneratedAt || existing.catalogGeneratedAt,
				});
			}
			setSectionStatus(kind, "");
			setSectionFooter(kind, "");
			if (activePanelName === kind) {
				requestAnimationFrame(() => loadMoreIfActiveSentinelVisible());
				scheduleDiscoveryPrefetch(kind);
			}
			return;
		}

		const next = buildDiscoveryStateFromData(kind, data, {
			offset,
			limit,
			reset,
			current: reset ? blankDiscoveryState() : current,
		});
		const nextSignature = discoveryEntriesSignature(kind, next.entries);
		const appendOnlyEntries = !reset ? next.entries.slice(previousCount) : null;
		const unchanged = nextSignature === previousSignature && !bypassCache;

		discoveryStates.set(kind, next);
		if (next.entries.length === 0) {
			setSectionStatus(kind, activeCopy.empty[kind]);
			setSectionEntries(kind, []);
			setSectionFooter(kind, "");
			return;
		}

		if (!unchanged) {
			if (reset) {
				renderDiscoveryState(kind, next);
			} else {
				setSectionStatus(kind, "");
				renderDiscoveryState(kind, next, { appendOnlyEntries });
				setSectionFooter(kind, "");
			}
		} else {
			setSectionStatus(kind, "");
			setSectionFooter(kind, "");
		}

		await persistSessionSnapshot(kind);
		if (activePanelName === kind) {
			requestAnimationFrame(() => loadMoreIfActiveSentinelVisible());
			if (reset) scheduleDiscoveryPrefetch(kind);
		}
	} catch (error) {
		discoveryStates.set(kind, { ...sectionState(kind), loading: false });
		if (renderedFromFastPath && reset) {
			setSectionFooter(kind, "");
			return;
		}
		if (reset) {
			setSectionStatus(kind, unavailableMessage(kind));
		} else {
			setSectionFooter(kind, unavailableMessage(kind));
		}
		throw error;
	}
}

async function refreshActivePanel() {
	await ensureSectionRendered(activePanelName, { bypassCache: true });
}

function activePaginationSentinel() {
	if (!isPageableDiscoveryKind(activePanelName)) return null;
	return document.getElementById(endpointSentinelId(activePanelName));
}

function sentinelIsNearScrollEnd(sentinel, content) {
	const sentinelRect = sentinel.getBoundingClientRect();
	const contentRect = content.getBoundingClientRect();
	return sentinelRect.top <= contentRect.bottom + 120;
}

function loadMoreIfActiveSentinelVisible() {
	const sentinel = activePaginationSentinel();
	const content = document.getElementById("content-area");
	if (!sentinel || !content) return;
	if (!sentinelIsNearScrollEnd(sentinel, content)) return;
	loadDiscoveryPage(activePanelName).catch(() => { });
}

function setupPaginationObserver(content) {
	if (!("IntersectionObserver" in window)) return;
	paginationObserver?.disconnect();
	paginationObserver = new IntersectionObserver(
		(entries) => {
			for (const entry of entries) {
				if (!entry.isIntersecting) continue;
				const kind = entry.target.dataset.paginationKind;
				if (kind !== activePanelName) continue;
				loadDiscoveryPage(kind).catch(() => { });
			}
		},
		{
			root: content,
			rootMargin: "120px 0px 120px 0px",
			threshold: 0,
		},
	);
	for (const kind of ["portals", "servers", "clients"]) {
		const sentinel = document.getElementById(endpointSentinelId(kind));
		if (!sentinel) continue;
		sentinel.dataset.paginationKind = kind;
		paginationObserver.observe(sentinel);
	}
}

function setRefreshIndicator(text, visible) {
	const indicator = document.getElementById("refresh-indicator");
	if (!indicator) return;
	indicator.textContent = text || "";
	indicator.classList.toggle("is-visible", visible);
}

function setupPullToRefresh(content) {
	let pointerStartY = null;
	let pullDistance = 0;
	let pulling = false;

	function resetPull() {
		pointerStartY = null;
		pullDistance = 0;
		pulling = false;
		setRefreshIndicator("", false);
	}

	content.addEventListener("pointerdown", (event) => {
		if (
			!shouldStartPullRefresh({
				button: event.button,
				pointerType: event.pointerType,
				scrollTop: content.scrollTop,
				panelName: activePanelName,
				selectionType: window.getSelection()?.type,
			})
		) {
			return;
		}
		pointerStartY = event.clientY;
	});

	content.addEventListener(
		"pointermove",
		(event) => {
			if (pointerStartY === null || content.scrollTop > 0) return;
			pullDistance = Math.max(0, event.clientY - pointerStartY);
			if (pullDistance <= 0) return;
			pulling = true;
			event.preventDefault();
			setRefreshIndicator(
				pullDistance >= PULL_REFRESH_THRESHOLD
					? activeCopy.releaseRefresh
					: activeCopy.pullRefresh,
				true,
			);
		},
		{ passive: false },
	);

	async function finishPull() {
		if (!pulling) {
			resetPull();
			return;
		}
		const shouldRefresh = pullDistance >= PULL_REFRESH_THRESHOLD;
		if (!shouldRefresh) {
			resetPull();
			return;
		}
		setRefreshIndicator(activeCopy.refreshing, true);
		try {
			await refreshActivePanel();
		} catch {
			// renderSection already surfaces the section-specific error state.
		}
		resetPull();
	}

	content.addEventListener("pointerup", () => {
		finishPull();
	});
	content.addEventListener("pointercancel", resetPull);
}

async function renderSection(kind, { bypassCache = false } = {}) {
	await loadDiscoveryPage(kind, { reset: true, bypassCache });
}

async function ensureSectionRendered(kind, { bypassCache = false } = {}) {
	if (!shouldRenderPanel({ panelName: kind, loaded: sectionLoaded(kind), bypassCache })) {
		return;
	}
	await renderSection(kind, { bypassCache });
}

function activatePanel(panelName) {
	saveActivePanelSnapshot();
	activePanelName = panelName;
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
	ensureSectionRendered(panelName).catch(() => {
		if (panelName !== "settings") {
			setSectionStatus(panelName, unavailableMessage(panelName));
		}
	});
	requestAnimationFrame(() => loadMoreIfActiveSentinelVisible());
}

document.addEventListener("DOMContentLoaded", async () => {
	void pruneExpiredDiscoveryCaches(discoveryContext());
	window.addEventListener("pagehide", saveActivePanelSnapshot);
	currentSettings = await readSettings();
	const languageSelect = document.getElementById("language-select");
	const themeSelect = document.getElementById("theme-select");
	const content = document.getElementById("content-area");
	languageSelect.value = currentSettings.language;
	themeSelect.value = currentSettings.theme;
	applyCopy(currentSettings.language);
	renderInlineIcons();
	applyTheme(currentSettings.theme);

	ensureSectionRendered(activePanelName).catch(() => {
		setSectionStatus(activePanelName, unavailableMessage(activePanelName));
	});

	async function persist(nextSettings) {
		const languageChanged = nextSettings.language !== currentSettings.language;
		currentSettings = await writeSettings(nextSettings);
		if (languageChanged) {
			applyCopy(currentSettings.language);
		}
		applyTheme(currentSettings.theme);
	}

	function resetDiscoveryPanelsForLocaleChange(previousLocale) {
		for (const kind of ["portals", "servers", "clients"]) {
			discoveryStates.delete(kind);
		}
		void clearSessionSnapshots(discoveryContext(), previousLocale);
	}

	languageSelect.addEventListener("change", () => {
		const previousLocale = activeDiscoveryLocale();
		void persist({ ...currentSettings, language: languageSelect.value }).then(() => {
			resetDiscoveryPanelsForLocaleChange(previousLocale);
			refreshActivePanel().catch(() => { });
		});
	});
	themeSelect.addEventListener("change", () =>
		persist({ ...currentSettings, theme: themeSelect.value }),
	);

	for (const tab of document.querySelectorAll("[data-panel-target]")) {
		tab.addEventListener("click", () => activatePanel(tab.dataset.panelTarget));
	}
	setupPaginationObserver(content);
	setupPullToRefresh(content);
	document.getElementById("refresh-button").addEventListener("click", () => {
		refreshActivePanel().catch(() => { });
	});
	for (const button of document.querySelectorAll("[data-open-url]")) {
		button.addEventListener("click", () => openExternalUrl(button.dataset.openUrl));
	}
	document
		.getElementById("settings-button")
		.addEventListener("click", () => activatePanel("settings"));
	window
		.matchMedia("(prefers-color-scheme: dark)")
		.addEventListener("change", () => applyTheme(currentSettings.theme));
});
