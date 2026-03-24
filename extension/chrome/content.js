/**
 * MCPMate Chrome extension content script.
 * Attaches import affordances directly to matching `pre` / `code` blocks.
 */
(function () {
	const MAX_PAYLOAD_CHARS = 48000;
	const SCHEME_URL = "mcpmate://import/server";
	/** JSON/YAML-ish keys that usually wrap MCP server maps (Cursor/VS Code `mcp`, Claude Desktop `mcpServers`, etc.). */
	const MCP_SNIPPET_PATTERNS = [
		/["']mcpServers["']\s*:/i,
		// e.g. "mcp": { "context7": { "type": "remote", "url": "…" } } — allow newline after colon
		/["']mcp["']\s*:\s*(?:\r?\n\s*)?\{/i,
	];
	// Inline SVG (same paths as icons/logo.svg). Page CSP often blocks <img src="chrome-extension://...">.
	const MCPMATE_LOGO_SVG = `<svg viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg" fill="#e2e8f0" aria-hidden="true" width="22" height="22" focusable="false"><path d="m357.31 214.37c24.62-25 49.95-49.38 74.21-74.68 3.93-8.02-4.62-15.74-12.15-11.13l-72.44 72.3c-9.14 8.79-23.69 9.71-33.58 1.61-11.69-9.57-12.88-25.74-2.93-37.12l72.5-72.49c5.68-8.56-4.45-18.08-12.63-11.37-23.11 24.32-48.61 47.02-71.49 71.48-11.31 12.09-15.2 28.3-9.52 44.14 10.03 28 45.72 37.02 68.03 17.26z"/><path d="m141.7 81.49c-8.18-6.71-18.31 2.81-12.63 11.37l72.5 72.49c9.95 11.38 8.76 27.55-2.93 37.12-9.89 8.1-24.44 7.17-33.58-1.61l-72.44-72.3c-7.53-4.61-16.07 3.12-12.15 11.13 24.26 25.3 49.59 49.69 74.21 74.68 22.31 19.76 58 10.74 68.03-17.26 5.68-15.85 1.78-32.06-9.52-44.14-22.88-24.46-48.38-47.16-71.49-71.48z"/><path d="m366.31 257.61c-.27-.53-.48-1.07-.51-1.61.04-.54.24-1.08.51-1.61l90.87-91.06c23.78-28.84 1.29-72.11-35.75-70.1-.71.04-2.97.74-3.29.27-.28-.42.36-2.91.4-3.66.75-14.43-6.81-29.52-18.79-37.42-16.12-10.63-37.23-9.18-51.74 3.46l-92.02 92-92.02-92c-14.51-12.64-35.61-14.09-51.74-3.46-11.99 7.9-19.54 23-18.79 37.42.04.75.69 3.24.4 3.66-.32.47-2.58-.23-3.29-.27-37.04-2.01-59.53 41.26-35.75 70.1l90.87 91.06c.27.53.48 1.07.51 1.61-.04.54-.24 1.08-.51 1.61l-90.87 91.06c-23.78 28.84-1.29 72.11 35.75 70.1.71-.04 2.97-.74 3.29-.27.28.42-.36 2.91-.4 3.66-.75 14.43 6.81 29.52 18.79 37.42 16.12 10.63 37.23 9.18 51.74-3.46l92.02-92 92.02 92c14.51 12.64 35.61 14.09 51.74 3.46 11.99-7.9 19.54-23 18.79-37.42-.04-.75-.69-3.24-.4-3.66.32-.47 2.58.23 3.29.27 37.04 2.01 59.53-41.26 35.75-70.1zm40.48 137.41-73.5-73.5c-7.92-6.24-17.8 2.57-12.74 11.17l74.51 74.57c19.89 23.6-9.55 55.36-34.59 37.12l-98.92-98.61c-1.84-1.34-3.76-1.9-5.56-1.86-1.8-.03-3.72.52-5.56 1.86l-98.92 98.61c-25.04 18.24-54.48-13.51-34.59-37.12l74.51-74.57c5.05-8.61-4.83-17.41-12.74-11.17l-73.5 73.5c-23.31 20.44-56.02-9.25-38.1-34.55l80.75-80.84c2.67-2.52 5.37-5.1 7.76-7.77l1.5-1.5c4.64-5.19 6.42-9.94 6.32-14.37.1-4.43-1.68-9.18-6.32-14.37l-1.5-1.5c-2.39-2.67-5.09-5.25-7.76-7.77l-80.75-80.84c-17.92-25.3 14.79-54.98 38.1-34.55l73.5 73.5c7.92 6.24 17.8-2.57 12.74-11.17l-74.51-74.57c-19.89-23.6 9.55-55.36 34.59-37.12l98.92 98.61c1.84 1.34 3.76 1.9 5.56 1.86 1.8.03 3.72-.52 5.56-1.86l98.92-98.61c25.04-18.24 54.48 13.51 34.59 37.12l-74.51 74.57c-5.05 8.61 4.83 17.41 12.74 11.17l73.5-73.5c23.31-20.44 56.02 9.25 38.1 34.55l-80.75 80.84c-2.67 2.52-5.37 5.1-7.76 7.77l-1.5 1.5c-4.64 5.19-6.42 9.94-6.32 14.37-.1 4.43 1.68 9.18 6.32 14.37l1.5 1.5c2.39 2.67 5.09 5.25 7.76 7.77l80.75 80.84c17.92 25.3-14.79 54.98-38.1 34.55z"/><path d="m357.31 297.63c-22.31-19.76-58-10.74-68.03 17.26-5.68 15.85-1.78 32.06 9.52 44.14 22.88 24.46 48.38 47.16 71.49 71.48 8.18 6.71 18.31-2.81 12.63-11.37l-72.5-72.49c-9.95-11.38-8.76-27.55 2.93-37.12 9.89-8.1 24.44-7.17 33.58 1.61l72.44 72.3c7.53 4.61 16.07-3.12 12.15-11.13-24.26-25.3-49.59-49.69-74.21-74.68z"/><path d="m154.69 297.63c-24.62 25-49.95 49.38-74.21 74.68-3.93 8.02 4.62 15.74 12.15 11.13l72.44-72.3c9.14-8.79 23.69-9.71 33.58-1.61 11.69 9.57 12.88 25.74 2.93 37.12l-72.5 72.49c-5.68 8.56 4.45 18.08 12.63 11.37 23.11-24.32 48.61-47.02 71.49-71.48 11.31-12.09 15.2-28.3 9.52-44.14-10.03-28-45.72-37.02-68.03-17.26z"/><path d="m0 0h512v512h-512z" fill="none"/></svg>`;
	const STYLE_ID = "mcpmate-import-style";
	const HOST_ATTR = "data-mcpmate-import-host";
	const PATCHED_POSITION_ATTR = "data-mcpmate-import-patched-position";
	const CONTROLS_SELECTOR = "[data-mcpmate-import-controls]";
	let scanTimer = 0;

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
		document.head.appendChild(style);
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

	function utf8ToBase64Url(obj) {
		const json = JSON.stringify(obj);
		const bytes = new TextEncoder().encode(json);
		let binary = "";
		for (let i = 0; i < bytes.length; i++) {
			binary += String.fromCharCode(bytes[i]);
		}
		const b64 = btoa(binary);
		return b64.replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
	}

	function openMcpMate(text) {
		const payload = {
			text,
			format: inferFormat(text),
			source: typeof location?.href === "string" ? location.href : "",
		};
		const p = utf8ToBase64Url(payload);
		const url = `${SCHEME_URL}?p=${encodeURIComponent(p)}`;
		const a = document.createElement("a");
		a.href = url;
		a.style.display = "none";
		document.documentElement.appendChild(a);
		a.click();
		a.remove();
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
			openMcpMate(text);
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
	}

	function scheduleScan() {
		window.clearTimeout(scanTimer);
		scanTimer = window.setTimeout(runScan, 180);
	}

	window.addEventListener("scroll", scheduleScan, { passive: true });
	window.addEventListener("resize", scheduleScan, { passive: true });

	const observer = new MutationObserver(() => scheduleScan());
	observer.observe(document.documentElement, {
		childList: true,
		subtree: true,
		characterData: true,
	});

	scheduleScan();
})();
