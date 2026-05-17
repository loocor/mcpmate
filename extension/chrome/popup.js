const DISCOVERY_ENDPOINT = `${globalThis.MCPMATE_EXTENSION_CONFIG.accountApiOrigin}/discovery/servers`;

const PORTALS = [
	{
		name: "Official MCP Registry",
		description: "Canonical registry for public MCP servers.",
		url: "https://modelcontextprotocol.io/registry",
		source: "modelcontextprotocol.io",
	},
	{
		name: "MCP Servers on GitHub",
		description: "Open-source MCP server implementations and examples.",
		url: "https://github.com/topics/mcp-server",
		source: "github.com/topics/mcp-server",
	},
	{
		name: "MCPMate Website",
		description: "MCPMate docs, downloads, and product updates.",
		url: "https://mcp.umate.ai",
		source: "mcp.umate.ai",
	},
];

const CLIENTS = [
	{
		name: "Claude Desktop",
		description: "Desktop client with native local MCP server support.",
		url: "https://claude.ai/download",
		source: "MCPMate curated compatibility list",
		signal: "native MCP",
	},
	{
		name: "Cursor",
		description: "Editor workflow with project-level MCP configuration.",
		url: "https://cursor.com",
		source: "MCPMate curated compatibility list",
		signal: "editor",
	},
	{
		name: "VS Code",
		description: "Developer client path for MCP-capable extensions.",
		url: "https://code.visualstudio.com",
		source: "MCPMate curated compatibility list",
		signal: "editor",
	},
];

function openExternalUrl(url) {
	if (chrome?.tabs?.create) {
		chrome.tabs.create({ url });
		return;
	}
	window.open(url, "_blank", "noopener,noreferrer");
}

function card({ name, description, url, source, signal, meta }) {
	const el = document.createElement("article");
	el.className = "card";
	const title = document.createElement("div");
	title.className = "card-title";
	title.textContent = name;
	const openButton = document.createElement("button");
	openButton.type = "button";
	openButton.className = "open-button";
	openButton.textContent = "Open";
	openButton.addEventListener("click", () => openExternalUrl(url));
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

function renderStaticList(targetId, entries) {
	const target = document.getElementById(targetId);
	target.replaceChildren(...entries.map(card));
}

function discoveryMeta(server) {
	return server?._meta?.["ai.mcpmate/discovery"] || {};
}

function firstTransport(server) {
	const remote = Array.isArray(server.remotes) ? server.remotes[0] : null;
	if (remote?.type) return remote.type;
	const pkg = Array.isArray(server.packages) ? server.packages[0] : null;
	if (pkg?.transport?.type) return pkg.transport.type;
	return "";
}

function entryUrl(server) {
	return (
		server.websiteUrl ||
		server.repository?.url ||
		"https://mcp.umate.ai/docs/en/market"
	);
}

function serverCard(entry) {
	const server = entry?.server || entry;
	const meta = discoveryMeta(server);
	const categories = Array.isArray(meta.categories)
		? meta.categories.slice(0, 2).join(", ")
		: "";
	const score =
		typeof meta.rating?.score === "number" ? `Score ${meta.rating.score}` : "";

	return card({
		name: server.title || server.name,
		description: server.description || "Curated MCP server entry.",
		url: entryUrl(server),
		source: server.repository?.source || "MCPMate curated catalog",
		signal: meta.quality?.status || score,
		meta: categories || firstTransport(server),
	});
}

async function renderServers() {
	const status = document.getElementById("server-status");
	const target = document.getElementById("server-list");
	status.textContent = "Loading server recommendations...";
	target.replaceChildren();
	const response = await fetch(DISCOVERY_ENDPOINT, {
		headers: { accept: "application/json" },
	});
	if (!response.ok) {
		throw new Error(`Discovery API returned ${response.status}`);
	}
	const data = await response.json();
	const servers = Array.isArray(data.servers) ? data.servers : [];
	if (servers.length === 0) {
		status.textContent = "No curated discovery entries are published yet.";
		return;
	}
	status.textContent = `Curated MCPMate catalog · ${servers.length} servers`;
	target.replaceChildren(...servers.map(serverCard));
}

function activateTab(tabName) {
	for (const tab of document.querySelectorAll(".tab")) {
		tab.classList.toggle("is-active", tab.dataset.tab === tabName);
	}
	for (const panel of document.querySelectorAll(".panel")) {
		panel.classList.toggle("is-active", panel.dataset.panel === tabName);
	}
}

document.addEventListener("DOMContentLoaded", () => {
	renderStaticList("portal-list", PORTALS);
	renderStaticList("client-list", CLIENTS);
	renderServers().catch((error) => {
		document.getElementById("server-status").textContent = error.message;
	});

	for (const tab of document.querySelectorAll(".tab")) {
		tab.addEventListener("click", () => activateTab(tab.dataset.tab));
	}
	for (const button of document.querySelectorAll("[data-open-url]")) {
		button.addEventListener("click", () => openExternalUrl(button.dataset.openUrl));
	}
});
