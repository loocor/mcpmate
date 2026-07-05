import { communityFooterForLanguage } from "./community-links.mjs";
import { extensionLanguageFromBrowser } from "./discovery-locale.mjs";
import {
	buildMcpMateImportUrl,
	consumeHandoffRecord,
} from "./import-handoff.mjs";
import {
	HANDOFF_DOWNLOAD_URL,
	HANDOFF_ISSUES_URL,
	HANDOFF_SETTINGS_KEY,
	handoffCopyForLanguage,
	normalizeHandoffLanguage,
} from "./handoff-copy.mjs";

const statusEl = document.getElementById("handoff-status");
const eyebrowEl = document.getElementById("handoff-eyebrow");
const titleEl = document.getElementById("handoff-title");
const summaryEl = document.getElementById("handoff-summary");
const noteEl = document.getElementById("handoff-note");
const helpTitleEl = document.getElementById("handoff-help-title");
const openButton = document.getElementById("open-button");
const openButtonLabel = document.getElementById("open-button-label");
const installLink = document.getElementById("install-link");
const installLinkLabel = document.getElementById("install-link-label");
const communityLink = document.getElementById("community-link");
const issuesLink = document.getElementById("issues-link");

const AUTO_OPEN_FALLBACK_DELAY_MS = 2400;

let activeCopy = handoffCopyForLanguage("en");
let activeImportUrl = "";

function setStatus(text, state) {
	statusEl.textContent = text;
	statusEl.dataset.state = state || "";
}

function setText(el, text) {
	if (el) {
		el.textContent = text;
	}
}

function setPageVisible(isVisible) {
	document.body.dataset.handoffVisible = isVisible ? "true" : "false";
}

function storageArea() {
	return globalThis.chrome?.storage?.sync || globalThis.chrome?.storage?.local || null;
}

function handoffIdFromLocation() {
	return new URL(globalThis.location.href).searchParams.get("id") || "";
}

function shouldAutoOpenFromLocation() {
	return new URL(globalThis.location.href).searchParams.get("fallback") !== "1";
}

function initialLanguageFromBrowser() {
	return normalizeHandoffLanguage(extensionLanguageFromBrowser());
}

async function readLanguage() {
	const fallbackLanguage = initialLanguageFromBrowser();
	const area = storageArea();
	if (area) {
		const stored = await area.get(HANDOFF_SETTINGS_KEY);
		return normalizeHandoffLanguage(
			stored[HANDOFF_SETTINGS_KEY]?.language ?? fallbackLanguage,
		);
	}

	try {
		const raw = localStorage.getItem(HANDOFF_SETTINGS_KEY);
		if (!raw) {
			return fallbackLanguage;
		}
		return normalizeHandoffLanguage(JSON.parse(raw)?.language ?? fallbackLanguage);
	} catch {
		return fallbackLanguage;
	}
}

function applyCopy(language) {
	activeCopy = handoffCopyForLanguage(language);
	const community = communityFooterForLanguage(language);

	document.title = activeCopy.documentTitle;
	document.documentElement.lang = activeCopy.documentLanguage;
	setText(eyebrowEl, activeCopy.eyebrow);
	setText(titleEl, activeCopy.title);
	setText(summaryEl, activeCopy.summary);
	setText(noteEl, activeCopy.note);
	setText(helpTitleEl, activeCopy.help.title);
	setText(openButtonLabel, activeCopy.actions.open);
	setText(installLinkLabel, activeCopy.actions.install);
	setText(communityLink, activeCopy.help.community);
	setText(issuesLink, activeCopy.help.issues);
	activeImportUrl = "";
	openButton.disabled = true;
	setPageVisible(false);
	setStatus(activeCopy.status.loading, "loading");

	installLink.href = HANDOFF_DOWNLOAD_URL;
	communityLink.href = community.href;
	issuesLink.href = HANDOFF_ISSUES_URL;
}

function setUnavailableState(message, state = "error") {
	activeImportUrl = "";
	openButton.disabled = true;
	setStatus(message, state);
	setText(summaryEl, activeCopy.unavailable.summary);
	setText(noteEl, activeCopy.unavailable.note);
	setPageVisible(true);
}

function activateCurrentTab() {
	if (!globalThis.chrome?.tabs?.getCurrent || !globalThis.chrome?.tabs?.update) {
		return;
	}
	globalThis.chrome.tabs.getCurrent((tab) => {
		if (tab?.id) {
			globalThis.chrome.tabs.update(tab.id, { active: true });
		}
	});
}

function showFallbackPage() {
	openButton.disabled = false;
	setStatus(activeCopy.status.ready, "ready");
	setPageVisible(true);
	activateCurrentTab();
}

async function loadHandoff() {
	const language = await readLanguage();
	applyCopy(language);

	const id = handoffIdFromLocation();
	if (!id) {
		setUnavailableState(activeCopy.status.missingId);
		return;
	}

	const record = await consumeHandoffRecord(id);
	if (!record) {
		setUnavailableState(activeCopy.status.expired, "stale");
		return;
	}

	activeImportUrl = buildMcpMateImportUrl(record.payload);
	if (shouldAutoOpenFromLocation()) {
		openMcpMate();
		setTimeout(showFallbackPage, AUTO_OPEN_FALLBACK_DELAY_MS);
		return;
	}

	showFallbackPage();
}

function openMcpMate() {
	if (!activeImportUrl) return;
	globalThis.location.href = activeImportUrl;
}

openButton.addEventListener("click", openMcpMate);

void loadHandoff();
