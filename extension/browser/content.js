/**
 * MCPMate browser extension content script.
 * Attaches import affordances directly to matching `pre` / `code` blocks
 * and injects "Install in MCPMate" into GitHub MCP page dropdown menus.
 */
(function () {
	if (
		!document.documentElement ||
		document.documentElement.localName !== "html"
	) {
		return;
	}
	const contentType = document.contentType?.toLowerCase() ?? "";
	if (contentType && !contentType.includes("html")) {
		return;
	}

	const MAX_PAYLOAD_CHARS = 48000;
	const IMPORT_FALLBACK_DELAY_MS = 1600;
	const IMPORT_HANDOFF_ERROR_MESSAGE =
		"MCPMate import handoff failed. Reload the extension and try again.";
	/** JSON/YAML-ish keys that usually wrap MCP server maps (Cursor/VS Code `mcp`, Claude Desktop `mcpServers`, etc.). */
	const MCP_SNIPPET_PATTERNS = [
		/["']mcpServers["']\s*:/i,
		// e.g. "mcp": { "context7": { "type": "remote", "url": "…" } } — allow newline after colon
		/["']mcp["']\s*:\s*(?:\r?\n\s*)?\{/i,
		// VS 2026 format — require an MCP-signature key alongside "servers" to avoid false positives
		/["']servers["']\s*:[\s\S]{0,200}["'](?:command|type|url|args)["']\s*:/i,
	];
	// Inline SVG (same paths as icons/logo.svg). Page CSP often blocks <img src="chrome-extension://...">.
	const MCPMATE_LOGO_SVG = `<svg viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg" fill="currentColor" aria-hidden="true" width="22" height="22" focusable="false"><path d="m357.31 214.37c24.62-25 49.95-49.38 74.21-74.68 3.93-8.02-4.62-15.74-12.15-11.13l-72.44 72.3c-9.14 8.79-23.69 9.71-33.58 1.61-11.69-9.57-12.88-25.74-2.93-37.12l72.5-72.49c5.68-8.56-4.45-18.08-12.63-11.37-23.11 24.32-48.61 47.02-71.49 71.48-11.31 12.09-15.2 28.3-9.52 44.14 10.03 28 45.72 37.02 68.03 17.26z"/><path d="m141.7 81.49c-8.18-6.71-18.31 2.81-12.63 11.37l72.5 72.49c9.95 11.38 8.76 27.55-2.93 37.12-9.89 8.1-24.44 7.17-33.58-1.61l-72.44-72.3c-7.53-4.61-16.07 3.12-12.15 11.13 24.26 25.3 49.59 49.69 74.21 74.68 22.31 19.76 58 10.74 68.03-17.26 5.68-15.85 1.78-32.06-9.52-44.14-22.88-24.46-48.38-47.16-71.49-71.48z"/><path d="m366.31 257.61c-.27-.53-.48-1.07-.51-1.61.04-.54.24-1.08.51-1.61l90.87-91.06c23.78-28.84 1.29-72.11-35.75-70.1-.71.04-2.97.74-3.29.27-.28-.42.36-2.91.4-3.66.75-14.43-6.81-29.52-18.79-37.42-16.12-10.63-37.23-9.18-51.74 3.46l-92.02 92-92.02-92c-14.51-12.64-35.61-14.09-51.74-3.46-11.99 7.9-19.54 23-18.79 37.42.04.75.69 3.24.4 3.66-.32.47-2.58-.23-3.29-.27-37.04-2.01-59.53 41.26-35.75 70.1l90.87 91.06c.27.53.48 1.07.51 1.61-.04.54-.24 1.08-.51 1.61l-90.87 91.06c-23.78 28.84-1.29 72.11 35.75 70.1.71-.04 2.97-.74 3.29-.27.28.42-.36 2.91-.4 3.66-.75 14.43 6.81 29.52 18.79 37.42 16.12 10.63 37.23 9.18 51.74-3.46l92.02-92 92.02 92c14.51 12.64 35.61 14.09 51.74 3.46 11.99-7.9 19.54-23 18.79-37.42-.04-.75-.69-3.24-.4-3.66.32-.47 2.58.23 3.29.27 37.04 2.01 59.53-41.26 35.75-70.1zm40.48 137.41-73.5-73.5c-7.92-6.24-17.8 2.57-12.74 11.17l74.51 74.57c19.89 23.6-9.55 55.36-34.59 37.12l-98.92-98.61c-1.84-1.34-3.76-1.9-5.56-1.86-1.8-.03-3.72.52-5.56 1.86l-98.92 98.61c-25.04 18.24-54.48-13.51-34.59-37.12l74.51-74.57c5.05-8.61-4.83-17.41-12.74-11.17l-73.5 73.5c-23.31 20.44-56.02-9.25-38.1-34.55l80.75-80.84c2.67-2.52 5.37-5.1 7.76-7.77l1.5-1.5c4.64-5.19 6.42-9.94 6.32-14.37.1-4.43-1.68-9.18-6.32-14.37l-1.5-1.5c-2.39-2.67-5.09-5.25-7.76-7.77l-80.75-80.84c-17.92-25.3 14.79-54.98 38.1-34.55l73.5 73.5c7.92 6.24 17.8-2.57 12.74-11.17l-74.51-74.57c-19.89-23.6 9.55-55.36 34.59-37.12l98.92 98.61c1.84 1.34 3.76 1.9 5.56 1.86 1.8.03 3.72-.52 5.56-1.86l98.92-98.61c25.04-18.24 54.48 13.51 34.59 37.12l-74.51 74.57c-5.05 8.61 4.83 17.41 12.74 11.17l73.5-73.5c23.31-20.44 56.02 9.25 38.1 34.55l-80.75 80.84c-2.67 2.52-5.37 5.1-7.76 7.77l-1.5 1.5c-4.64 5.19-6.42 9.94-6.32 14.37-.1 4.43 1.68 9.18 6.32 14.37l1.5 1.5c2.39 2.67 5.09 5.25 7.76 7.77l80.75 80.84c17.92 25.3-14.79 54.98-38.1 34.55z"/><path d="m357.31 297.63c-22.31-19.76-58-10.74-68.03 17.26-5.68 15.85-1.78 32.06 9.52 44.14 22.88 24.46 48.38 47.16 71.49 71.48 8.18 6.71 18.31-2.81 12.63-11.37l-72.5-72.49c-9.95-11.38-8.76-27.55 2.93-37.12 9.89-8.1 24.44-7.17 33.58 1.61l72.44 72.3c7.53 4.61 16.07-3.12 12.15-11.13-24.26-25.3-49.59-49.69-74.21-74.68z"/><path d="m154.69 297.63c-24.62 25-49.95 49.38-74.21 74.68-3.93 8.02 4.62 15.74 12.15 11.13l72.44-72.3c9.14-8.79 23.69-9.71 33.58-1.61 11.69 9.57 12.88 25.74 2.93 37.12l-72.5 72.49c-5.68 8.56 4.45 18.08 12.63 11.37 23.11-24.32 48.61-47.02 71.49-71.48 11.31-12.09 15.2-28.3 9.52-44.14-10.03-28-45.72-37.02-68.03-17.26z"/><path d="m0 0h512v512h-512z" fill="none"/></svg>`;
	const STYLE_ID = "mcpmate-import-style";
	const HOST_ATTR = "data-mcpmate-import-host";
	const PATCHED_POSITION_ATTR = "data-mcpmate-import-patched-position";
	const CONTROLS_SELECTOR = "[data-mcpmate-import-controls]";
	const GITHUB_MCP_INJECTED_ATTR = "data-mcpmate-github-injected";
	let scanTimer = 0;

	/** GitHub MCP page dropdown menu selectors — prefer data-component over fragile CSS-module hashes */
	const GITHUB_SELECTORS = {
		installButton: 'button[data-component="Button"][aria-haspopup="true"]',
		dropdownMenu: 'ul[role="menu"][data-component="ActionList"]',
		menuItem: 'li[role="menuitem"][data-component="ActionList.Item"]',
		itemLabel: '[data-component="ActionList.Item.Label"]',
		itemIcon: '[data-component="ActionList.LeadingVisual"]',
	};

	/**
	 * GitHub MCP page handler.
	 * Detects GitHub MCP pages and injects "Install in MCPMate" into Install dropdown menus.
	 */
	const githubMcpHandler = {
		menuObserver: null,
		isActive: false,

		/**
		 * Check if current page is a GitHub MCP page.
		 */
		isGitHubMcpPage() {
			if (location.hostname !== "github.com") return false;
			const path = location.pathname;
			return (
				path === "/mcp" ||
				path.startsWith("/mcp/") ||
				(path === "/search" && (location.search.includes("topic%3Amcp") || location.search.includes("topic:mcp")))
			);
		},

		/**
		 * Initialize the handler if on a GitHub MCP page.
		 */
		init() {
			if (!this.isGitHubMcpPage()) {
				this.cleanup();
				return;
			}
			if (this.isActive) return;

			this.isActive = true;
			this.injectStyles();
			this.setupMenuObserver();

			// Scan existing menus on page load
			this.scanExistingMenus();
		},

		/**
		 * Inject custom styles for MCPMate menu items.
		 */
		injectStyles() {
			if (document.getElementById("mcpmate-github-style")) return;

			const style = document.createElement("style");
			style.id = "mcpmate-github-style";
			style.textContent = `
				.mcpmate-menu-item [data-component="ActionList.LeadingVisual"] svg {
					width: 16px;
					height: 16px;
				}
				.mcpmate-menu-item:hover {
					background: var(--bgColor-neutral-muted, rgba(175, 184, 193, 0.2));
				}
				.mcpmate-menu-item {
					color: var(--fgColor-default, #1f2328);
				}
				[data-color-mode="dark"] .mcpmate-menu-item {
					color: var(--fgColor-default, #e6edf3);
				}
				[data-color-mode="auto"][data-light-theme="dark"] .mcpmate-menu-item {
					color: var(--fgColor-default, #e6edf3);
				}
				[data-color-mode="auto"][data-dark-theme^="dark"] .mcpmate-menu-item {
					color: var(--fgColor-default, #e6edf3);
				}
				.mcpmate-menu-item [data-component="ActionList.LeadingVisual"] svg {
					fill: currentColor;
				}
				li.mcpmate-divider {
					height: 1px;
					margin: 4px 0;
					background: var(--borderColor-muted, rgba(31, 35, 40, 0.15));
					list-style: none;
				}
			`;
			(document.head || document.documentElement).appendChild(style);
		},

		/**
		 * Set up MutationObserver to detect dropdown menus appearing.
		 */
		setupMenuObserver() {
			if (this.menuObserver) return;

			this.menuObserver = new MutationObserver((mutations) => {
				for (const mutation of mutations) {
					for (const node of mutation.addedNodes) {
						if (node.nodeType !== Node.ELEMENT_NODE) continue;

						// Check if the added node is a menu or contains one
						const menu =
							node.matches?.(GITHUB_SELECTORS.dropdownMenu)
								? node
								: node.querySelector?.(GITHUB_SELECTORS.dropdownMenu);

						if (menu) {
							this.injectMcpMateItem(menu);
						}
					}
				}
			});

			this.menuObserver.observe(document.body, {
				childList: true,
				subtree: true,
			});
		},

		/**
		 * Scan for existing menus that are already open.
		 */
		scanExistingMenus() {
			const menus = document.querySelectorAll(
				GITHUB_SELECTORS.dropdownMenu,
			);
			for (const menu of menus) {
				this.injectMcpMateItem(menu);
			}
		},

		/**
		 * Inject MCPMate menu item into a dropdown menu.
		 */
		injectMcpMateItem(menuElement) {
			// Check if already injected
			if (menuElement.hasAttribute(GITHUB_MCP_INJECTED_ATTR)) return;

			// Verify this is an Install menu (contains VS Code option)
			const hasVSCodeItem = Array.from(
				menuElement.querySelectorAll(GITHUB_SELECTORS.itemLabel),
			).some(
				(label) =>
					label.textContent.includes("Install in VS Code"),
			);

			if (!hasVSCodeItem) return;

			// Mark as injected to prevent duplicates
			menuElement.setAttribute(GITHUB_MCP_INJECTED_ATTR, "1");

			// Add divider
			const divider = document.createElement("li");
			divider.className = "mcpmate-divider";
			divider.setAttribute("role", "separator");

			// Create MCPMate menu item using safe DOM methods
			const menuItem = document.createElement("li");
			menuItem.className =
				"prc-ActionList-ActionListItem-So4vC mcpmate-menu-item";
			menuItem.setAttribute("role", "menuitem");
			menuItem.setAttribute("tabindex", "-1");
			menuItem.setAttribute("data-component", "ActionList.Item");
			menuItem.setAttribute("data-has-description", "false");

			const contentDiv = document.createElement("div");
			contentDiv.className = "prc-ActionList-ActionListContent-KBb8-";
			contentDiv.setAttribute("data-size", "medium");

			const spacer = document.createElement("span");
			spacer.className = "prc-ActionList-Spacer-4tR2m";

			const iconContainer = document.createElement("span");
			iconContainer.className =
				"prc-ActionList-LeadingVisual-NBr28 prc-ActionList-VisualWrap-bdCsS";
			iconContainer.setAttribute(
				"data-component",
				"ActionList.LeadingVisual",
			);

			// Parse and insert SVG safely
			const parser = new DOMParser();
			const svgDoc = parser.parseFromString(
				MCPMATE_LOGO_SVG,
				"image/svg+xml",
			);
			const svgElement = svgDoc.documentElement;
			if (svgElement && !svgElement.querySelector("parsererror")) {
				iconContainer.appendChild(
					document.importNode(svgElement, true),
				);
			}

			const labelContainer = document.createElement("span");
			labelContainer.className =
				"prc-ActionList-ActionListSubContent-gKsFp";
			labelContainer.setAttribute(
				"data-component",
				"ActionList.Item--DividerContainer",
			);

			const labelSpan = document.createElement("span");
			labelSpan.className = "prc-ActionList-ItemLabel-81ohH";
			labelSpan.setAttribute("data-component", "ActionList.Item.Label");
			labelSpan.textContent = "Install in MCPMate";

			// Assemble the menu item
			labelContainer.appendChild(labelSpan);
			contentDiv.appendChild(spacer);
			contentDiv.appendChild(iconContainer);
			contentDiv.appendChild(labelContainer);
			menuItem.appendChild(contentDiv);

			// Add click handler
			menuItem.addEventListener("click", (event) => {
				event.preventDefault();
				event.stopPropagation();
				this.handleInstall(menuElement).catch((err) => {
					console.error("[MCPMate] Install failed:", err);
				});
			});

			// Insert at end of menu
			menuElement.appendChild(divider);
			menuElement.appendChild(menuItem);
		},

		/**
		 * Handle MCPMate install click.
		 * Fetches server config from GitHub's JSON API (detail page) and sends to MCPMate.
		 */
		async handleInstall(menuElement) {
			// Find the Install button to locate the server card
			const labelledBy = menuElement.getAttribute("aria-labelledby");
			let installButton = labelledBy ? document.getElementById(labelledBy) : null;

			if (!installButton) {
				for (const btn of document.querySelectorAll(GITHUB_SELECTORS.installButton)) {
					if (btn.getAttribute("aria-expanded") === "true") {
						installButton = btn;
						break;
					}
				}
			}

			if (!installButton) {
				console.warn("[MCPMate] Could not find Install button");
				return;
			}

			// Find the server detail page link (/mcp/{owner}/{repo})
			const serverPath = this.findServerPath(installButton);
			if (!serverPath) {
				console.warn("[MCPMate] Could not find server detail page link");
				return;
			}

			// Fetch server config from GitHub's JSON API
			try {
				const config = await this.fetchServerConfig(serverPath);
				if (config) {
					await openMcpMate(config).catch(reportImportHandoffFailure);
				} else {
					console.warn("[MCPMate] Could not build server config");
				}
			} catch (err) {
				console.error("[MCPMate] Failed to fetch server config:", err);
			}
		},

		/**
		 * Find the server detail page path (/mcp/{owner}/{repo}) from the Install button.
		 */
		findServerPath(installButton) {
			// Walk up to find the card, then look for a link to /mcp/...
			let el = installButton;
			while (el && el !== document.body) {
				const link = el.querySelector('a[href^="/mcp/"]');
				if (link) {
					const href = link.getAttribute("href");
					const match = href.match(/^\/mcp\/([^/]+\/[^/?#]+)/);
					if (match) return `/mcp/${match[1]}`;
				}
				el = el.parentElement;
			}

			// Fallback: on detail pages (/mcp/{owner}/{repo}), the Install
			// button belongs to the server whose page we are already on.
			const pageMatch = location.pathname.match(/^\/mcp\/([^/]+\/[^/]+)/);
			if (pageMatch) return `/mcp/${pageMatch[1]}`;

			return null;
		},

		/**
		 * Fetch server config from GitHub MCP detail page JSON API.
		 * Returns a JSON string ready for openMcpMate().
		 */
		async fetchServerConfig(serverPath) {
			const url = `https://github.com${serverPath}`;
			const response = await fetch(url, {
				headers: { Accept: "application/json" },
				credentials: "same-origin",
			});

			if (!response.ok) {
				throw new Error(`HTTP ${response.status}`);
			}

			const data = await response.json();
			const server = data?.payload?.mcpDetailsRoute?.server_data?.raw_data?.server;
			if (!server) {
				throw new Error("No server data in response");
			}

			return this.convertRegistryToMcpMate(server);
		},

		/**
		 * Convert GitHub MCP registry server format to MCPMate import format.
		 */
		convertRegistryToMcpMate(server) {
			return globalThis.__MCPMATE_REGISTRY_IMPORT__?.convertRegistryToMcpMate(
				server,
			) ?? null;
		},

		/**
		 * Clean up observer and reset state.
		 */
		cleanup() {
			if (this.menuObserver) {
				this.menuObserver.disconnect();
				this.menuObserver = null;
			}
			this.isActive = false;
		},
	};

	function ensureStyle() {
		if (document.getElementById(STYLE_ID)) return;
		const style = document.createElement("style");
		style.id = STYLE_ID;
		style.textContent = `
			[${HOST_ATTR}="1"] ${CONTROLS_SELECTOR} {
				position: absolute;
				top: 50%;
				right: 8px;
				left: auto;
				transform: translateY(-50%);
				z-index: 10;
				display: flex;
				flex-direction: row;
				align-items: center;
				gap: 0;
				padding: 4px;
				border-radius: 10px;
				background: rgba(15, 23, 42, 0.88);
				backdrop-filter: blur(6px);
				box-shadow: 0 6px 18px rgba(15, 23, 42, 0.2);
				opacity: 0.42;
				outline: none;
				transition:
					opacity 180ms ease,
					gap 200ms ease,
					box-shadow 180ms ease;
			}
			[${HOST_ATTR}="1"]:hover ${CONTROLS_SELECTOR},
			[${HOST_ATTR}="1"]:focus-within ${CONTROLS_SELECTOR} {
				opacity: 1;
			}
			[${HOST_ATTR}="1"] ${CONTROLS_SELECTOR}:hover,
			[${HOST_ATTR}="1"] ${CONTROLS_SELECTOR}:focus-within {
				opacity: 1;
				gap: 8px;
				box-shadow: 0 10px 24px rgba(37, 99, 235, 0.18);
			}
			[${HOST_ATTR}="1"] .mcpmate-import-brand {
				flex: 0 0 auto;
				display: flex;
				align-items: center;
				justify-content: center;
				width: 30px;
				height: 30px;
				border-radius: 8px;
				border: 1px solid rgba(255, 255, 255, 0.92);
				box-shadow:
					inset 0 0 0 1px rgba(255, 255, 255, 0.18),
					0 0 0 1px rgba(255, 255, 255, 0.22);
				overflow: hidden;
			}
			[${HOST_ATTR}="1"] .mcpmate-import-brand svg {
				width: 22px;
				height: 22px;
				display: block;
				fill: #fff;
			}
			[${HOST_ATTR}="1"] .mcpmate-import-actions {
				display: flex;
				flex-direction: row;
				align-items: center;
				gap: 8px;
				max-width: 0;
				opacity: 0;
				overflow: hidden;
				pointer-events: none;
				white-space: nowrap;
				transition:
					max-width 220ms ease,
					opacity 180ms ease;
			}
			[${HOST_ATTR}="1"] ${CONTROLS_SELECTOR}:hover .mcpmate-import-actions,
			[${HOST_ATTR}="1"] ${CONTROLS_SELECTOR}:focus-within .mcpmate-import-actions {
				max-width: 200px;
				opacity: 1;
				pointer-events: auto;
			}
			[${HOST_ATTR}="1"] .mcpmate-import-button {
				border: none;
				border-radius: 999px;
				padding: 7px 12px;
				cursor: pointer;
				font:
					600 12px/1 system-ui,
					-apple-system,
					BlinkMacSystemFont,
					"Segoe UI",
					sans-serif;
				color: #fff;
				background: linear-gradient(135deg, #2563eb, #7c3aed);
				white-space: nowrap;
			}
			[${HOST_ATTR}="1"] .mcpmate-import-button:hover {
				filter: brightness(1.06);
			}
		`;
		(document.head || document.documentElement).appendChild(style);
	}

	function normalizeText(text) {
		return String(text || "").replace(/\u00a0/g, " ").trim();
	}

	function looksLikeMcpSnippet(text) {
		const t = normalizeText(text);
		if (t.length < 12 || t.length > MAX_PAYLOAD_CHARS) return false;
		return MCP_SNIPPET_PATTERNS.some((re) => re.test(t));
	}

	function inferFormat(text) {
		const t = normalizeText(text);
		if (t.startsWith("{") || t.startsWith("[")) return "json";
		if (/^\s*mcpServers\s*=/i.test(t) || (t.includes("[mcp") && t.includes("]"))) {
			return "toml";
		}
		return undefined;
	}

	function detectSource() {
		if (location.hostname === "github.com") {
			const path = location.pathname;
			if (path === "/mcp" || path.startsWith("/mcp/")) {
				return { type: "portal", ref: "github-mcp-registry" };
			}
		}
		if (location.hostname === "cursor.directory" || location.hostname.endsWith(".cursor.directory")) {
			return { type: "portal", ref: "cursor-directory" };
		}
		return { type: "browser" };
	}

	function openExternalImportUrl(url) {
		const a = document.createElement("a");
		a.href = url;
		a.style.display = "none";
		document.documentElement.appendChild(a);
		a.click();
		a.remove();
	}

	function withQueryParam(url, key, value) {
		const parsed = new URL(url);
		parsed.searchParams.set(key, value);
		return parsed.toString();
	}

	function shouldOpenImportFallback() {
		return document.visibilityState === "visible" && document.hasFocus();
	}

	function requestImportFallbackPage(url) {
		if (globalThis.chrome?.runtime?.sendMessage) {
			globalThis.chrome.runtime.sendMessage({
				type: "mcpmate.openImportFallback",
				url,
			});
			return;
		}
		window.open(url, "_blank", "noopener,noreferrer");
	}

	function scheduleImportFallback(handoff, id) {
		const fallbackUrl = withQueryParam(
			handoff.buildHandoffPageUrl(id),
			"fallback",
			"1",
		);
		window.setTimeout(() => {
			if (!shouldOpenImportFallback()) {
				return;
			}
			requestImportFallbackPage(fallbackUrl);
		}, IMPORT_FALLBACK_DELAY_MS);
	}

	async function openMcpMate(text, sourceOverride) {
		const handoff = globalThis.__MCPMATE_IMPORT_HANDOFF__;
		if (!handoff) {
			window.alert(
				"MCPMate import handoff is unavailable. Reload the extension and try again.",
			);
			return;
		}
		const payload = {
			text,
			format: inferFormat(text),
			source: sourceOverride ?? detectSource(),
		};
		const id = handoff.createHandoffId();
		const record = handoff.createHandoffRecord(payload);
		await handoff.writeHandoffRecord(id, record);
		openExternalImportUrl(handoff.buildMcpMateImportUrl(payload));
		scheduleImportFallback(handoff, id);
	}

	function reportImportHandoffFailure(error) {
		console.error("[MCPMate] Import handoff failed:", error);
		window.alert(IMPORT_HANDOFF_ERROR_MESSAGE);
	}

	function getSnippetText(block) {
		const raw = block.innerText ?? block.textContent ?? "";
		const text = normalizeText(raw);
		return looksLikeMcpSnippet(text) ? text : null;
	}

	function getCandidateBlocks() {
		const blocks = [];
		for (const pre of document.querySelectorAll("pre")) {
			if (pre instanceof HTMLElement) blocks.push(pre);
		}
		for (const code of document.querySelectorAll("code")) {
			if (!(code instanceof HTMLElement)) continue;
			if (code.closest("pre")) continue;
			blocks.push(code);
		}
		return blocks;
	}

	function ensureHostPosition(block) {
		if (window.getComputedStyle(block).position !== "static") return;
		if (block.getAttribute(PATCHED_POSITION_ATTR) === "1") return;
		block.style.position = "relative";
		block.setAttribute(PATCHED_POSITION_ATTR, "1");
	}

	function restoreHostPosition(block) {
		if (block.getAttribute(PATCHED_POSITION_ATTR) !== "1") return;
		block.style.position = "";
		block.removeAttribute(PATCHED_POSITION_ATTR);
	}

	function appendInlineLogo(brandEl) {
		const parsed = new DOMParser().parseFromString(
			MCPMATE_LOGO_SVG,
			"image/svg+xml",
		);
		if (parsed.querySelector("parsererror")) return;
		const svg = parsed.documentElement;
		if (!svg) return;
		brandEl.appendChild(document.importNode(svg, true));
	}

	function buildControls(block) {
		const controls = document.createElement("div");
		controls.setAttribute("data-mcpmate-import-controls", "1");
		controls.setAttribute("tabindex", "0");
		controls.setAttribute("role", "toolbar");
		controls.setAttribute(
			"aria-label",
			"MCPMate: add this configuration to the desktop app",
		);

		const brand = document.createElement("div");
		brand.className = "mcpmate-import-brand";
		appendInlineLogo(brand);

		const actions = document.createElement("div");
		actions.className = "mcpmate-import-actions";

		const button = document.createElement("button");
		button.type = "button";
		button.className = "mcpmate-import-button";
		button.textContent = "Add to MCPMate";
		button.setAttribute(
			"aria-label",
			"Add this MCP server configuration to MCPMate",
		);

		button.addEventListener("click", (event) => {
			event.preventDefault();
			event.stopPropagation();
			const text = getSnippetText(block);
			if (!text) return;
			if (text.length > MAX_PAYLOAD_CHARS) {
				window.alert(
					"This configuration is too large to send via link. Shorten the snippet or add the server manually in MCPMate.",
				);
				return;
			}
			openMcpMate(text).catch(reportImportHandoffFailure);
		});

		actions.appendChild(button);
		controls.appendChild(brand);
		controls.appendChild(actions);
		return controls;
	}

	function attachControls(block) {
		const existing = block.querySelector(CONTROLS_SELECTOR);
		if (existing) return;
		ensureHostPosition(block);
		block.setAttribute(HOST_ATTR, "1");
		block.appendChild(buildControls(block));
	}

	function detachControls(block) {
		const controls = block.querySelector(CONTROLS_SELECTOR);
		if (controls) controls.remove();
		block.removeAttribute(HOST_ATTR);
		restoreHostPosition(block);
	}

	const CURSOR_DEEPLINK_ATTR = "data-mcpmate-cursor-injected";
	const CURSOR_MCP_RE = /^cursor:\/\/anysphere\.cursor-deeplink\/mcp\/install\b/;

	function attachCursorMcpMateButton(link) {
		if (link.getAttribute(CURSOR_DEEPLINK_ATTR) === "1") return;
		link.setAttribute(CURSOR_DEEPLINK_ATTR, "1");

		const container = link.parentElement;
		if (!container) return;
		container.style.position = "relative";

		const btn = document.createElement("a");
		btn.href = "#";
		btn.className = link.className;
		btn.textContent = "Add to MCPMate";
		btn.style.marginLeft = "4px";
		btn.addEventListener("click", (event) => {
			event.preventDefault();
			event.stopPropagation();
			const parseCursorMcpInstallLink =
				globalThis.__MCPMATE_CURSOR_DEEPLINK__?.parseCursorMcpInstallLink;
			const config = parseCursorMcpInstallLink?.(link.href) ?? null;
			if (config) {
				openMcpMate(config, { type: "portal", ref: "cursor-directory" }).catch(
					reportImportHandoffFailure,
				);
			}
		});

		container.appendChild(btn);
	}

	function runScan() {
		ensureStyle();
		for (const block of getCandidateBlocks()) {
			const text = getSnippetText(block);
			if (text) {
				attachControls(block);
			} else if (block.hasAttribute(HOST_ATTR)) {
				detachControls(block);
			}
		}

		// Scan for Cursor MCP deep links (cursor.directory and similar sites)
		for (const link of document.querySelectorAll('a[href^="cursor://anysphere.cursor-deeplink/mcp/install"]')) {
			if (CURSOR_MCP_RE.test(link.href)) {
				attachCursorMcpMateButton(link);
			}
		}
	}

	function scheduleScan() {
		window.clearTimeout(scanTimer);
		scanTimer = window.setTimeout(runScan, 180);
	}

	window.addEventListener("scroll", scheduleScan, { passive: true });
	window.addEventListener("resize", scheduleScan, { passive: true });

	// GitHub MCP page handling: initialize on load and cleanup on navigation
	if (typeof navigation !== "undefined" && navigation.addEventListener) {
		// Modern browsers with Navigation API
		navigation.addEventListener("navigate", () => {
			githubMcpHandler.cleanup();
			window.setTimeout(() => githubMcpHandler.init(), 100);
		});
	}
	window.addEventListener("popstate", () => {
		githubMcpHandler.cleanup();
		window.setTimeout(() => githubMcpHandler.init(), 100);
	});

	const observer = new MutationObserver(() => scheduleScan());
	observer.observe(document.documentElement, {
		childList: true,
		subtree: true,
		characterData: true,
	});

	scheduleScan();

	// Initialize GitHub MCP handler after initial scan
	githubMcpHandler.init();
})();
