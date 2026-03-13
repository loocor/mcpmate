(() => {
	try {
		const emitToParent = (level, message, detail) => {
			try {
				if (window?.parent && window.parent !== window) {
					window.parent.postMessage(
						{
							type: "mcpmate-market-log",
							payload: {
								level,
								message,
								detail,
								portalId,
							},
						},
						"*",
					);
				}
			} catch {}
		};
		// Enhanced error handling for Next.js apps
		const setupNextjsErrorHandling = () => {
			if (portalId !== "mcpso") return;

			// Capture and handle Next.js hydration errors
			window.addEventListener("error", (event) => {
				if (
					event.error?.message?.includes("Hydration") ||
					event.error?.message?.includes("Text content does not match")
				) {
					console.warn(
						"[mcpmate] Suppressed Next.js hydration error:",
						event.error.message,
					);
					emitToParent("warn", "Suppressed Next.js hydration error", {
						message: event.error?.message,
					});
					event.preventDefault();
					event.stopPropagation();
					return false;
				}
			});

			// Handle React hydration warnings
			const originalConsoleError = console.error;
			console.error = (...args) => {
				const message = args.join(" ");
				if (
					message.includes("Warning: Text content did not match") ||
					message.includes("Warning: Prop")
				) {
					console.warn("[mcpmate] Suppressed React warning:", message);
					emitToParent("warn", "Suppressed React warning", { message });
					return;
				}
				originalConsoleError.apply(console, args);
			};
		};

		// Safe global fallbacks to avoid third-party loader aborts
		if (typeof window !== "undefined") {
			try {
				if (typeof window.__name !== "function") {
					window.__name = (x) => x;
				}
				if (!Array.isArray(window.dataLayer)) {
					window.dataLayer = [];
				}
				if (typeof window.gtag !== "function") {
					window.gtag = function gtag(...args) {
						try {
							window.dataLayer.push(args);
						} catch {}
					};
				}
			} catch {}
		}
		const rawConfig =
			typeof window !== "undefined" ? window.__MCPMATE_PORTAL__ || {} : {};
		if (typeof window !== "undefined") {
			try {
				delete window.__MCPMATE_PORTAL__;
			} catch (_error) {
				/* noop */
			}
		}
		const prefixBase =
			typeof rawConfig.prefix === "string" && rawConfig.prefix.trim()
				? rawConfig.prefix.trim()
				: "/market-proxy";
		const prefix = prefixBase.replace(/\/$/, "");
		const prefixWithSlash = `${prefix}/`;
		const remoteOrigin =
			typeof rawConfig.remoteOrigin === "string" &&
			rawConfig.remoteOrigin.trim()
				? rawConfig.remoteOrigin.trim()
				: window.location.origin;
		const portalId =
			typeof rawConfig.portalId === "string" && rawConfig.portalId.trim()
				? rawConfig.portalId.trim()
				: "unknown";
		const adapterId =
			typeof rawConfig.adapter === "string" && rawConfig.adapter.trim()
				? rawConfig.adapter.trim()
				: "default";
		const adapters = {
			default: {},
			mcpmarket: {},
			mcpso: {
				detectConfigSnippet: (text, defaultDetector) => {
					// MCP.so specific configuration detection
					const result = defaultDetector(text);
					if (result) return result;

					// Special handling for MCP.so's format
					if (text.includes('"mcpServers"') || text.includes('"servers"')) {
						try {
							const jsonMatch = text.match(/\{[\s\S]*\}/);
							if (jsonMatch) {
								const parsed = JSON.parse(jsonMatch[0]);
								if (parsed.mcpServers || parsed.servers) {
									return { format: "json", text: jsonMatch[0] };
								}
							}
						} catch {}
					}
					return null;
				},
			},
		};
		const adapter = adapters[adapterId] || adapters.default;
		const notifyReady = (() => {
			let sent = false;
			return () => {
				if (sent) return;
				sent = true;
				try {
					if (window?.parent && window.parent !== window) {
						window.parent.postMessage(
							{
								type: "mcpmate-market-ready",
								payload: { portalId, adapter: adapterId },
							},
							"*",
						);
						emitToParent("info", "market-ready", {
							portalId,
							adapter: adapterId,
						});
					}
				} catch {
					/* noop */
				}
			};
		})();

		// Global error forwarding (outside of mcp.so specifics)
		try {
			window.addEventListener("error", (e) => {
				emitToParent("error", "window.error", {
					message: e?.error?.message || String(e?.message || ""),
					stack: e?.error?.stack || "",
				});
			});
			window.addEventListener("unhandledrejection", (e) => {
				emitToParent("error", "unhandledrejection", {
					reason: String(e?.reason || ""),
				});
			});
		} catch {}

		const mapUrl = (input) => {
			try {
				if (!input || typeof input !== "string") return input;
				// Already mapped to our proxy
				if (input.startsWith(`${prefix}/`)) return input;
				const currentOrigin = window.location.origin;
				let remoteOriginOrigin = null;
				try {
					remoteOriginOrigin = new URL(remoteOrigin).origin;
				} catch {}
				// Absolute or protocol-relative URLs
				try {
					const abs = new URL(input, currentOrigin);
					const sameOrigin = abs.origin === currentOrigin;
					const isRemoteOrigin =
						remoteOriginOrigin && abs.origin === remoteOriginOrigin;
					if ((sameOrigin || isRemoteOrigin) && abs.pathname.startsWith("/")) {
						if (
							abs.pathname.startsWith("/_next/") ||
							abs.pathname.startsWith("/static/") ||
							abs.pathname.startsWith("/assets/") ||
							abs.pathname.startsWith("/images/") ||
							// Add special handling for mcp.so's locale routes
							(portalId === "mcpso" &&
								abs.pathname.match(/^\/(en|zh|ja)(\/|$)/))
						) {
							return `${prefix}${abs.pathname}${abs.search}${abs.hash}`;
						}
					}
				} catch {}
				// Root-relative URLs
				if (input.startsWith("/")) {
					// Special handling for mcp.so locale routes
					if (portalId === "mcpso" && input.match(/^\/(en|zh|ja)(\/|$)/)) {
						return `${prefix}${input}`;
					}
					return prefix + input;
				}
				// Other absolute cross-origin URLs: keep as-is
				return input;
			} catch {
				return input;
			}
		};
		const assetPrefix = `${prefixWithSlash}_next/`;
		const updatePublicPath = () => {
			try {
				const nextRequire =
					window.__next_require__ ||
					window.__webpack_require__ ||
					window.__NEXT_DATA__?.scriptLoader?.webpackRequire ||
					self.__webpack_require__;
				if (nextRequire) {
					try {
						const current = nextRequire.p;
						if (
							typeof current !== "string" ||
							!String(current).startsWith(prefixWithSlash)
						) {
							nextRequire.p = assetPrefix;
						}
					} catch {
						// set regardless
						nextRequire.p = assetPrefix;
					}
					emitToParent("debug", "set webpack public path", { p: assetPrefix });
				}
				// Also set webpack public path fallback
				try {
					window.__webpack_public_path__ = assetPrefix;
				} catch {}
			} catch {
				/* noop */
			}
		};
		const patchRuntimeEntry = (entry) => {
			if (!Array.isArray(entry) || entry.length < 3) return;
			const runtime = entry[2];
			if (typeof runtime !== "function" || runtime.__mcpmatePatched) return;
			entry[2] = function patchedRuntime(...args) {
				updatePublicPath();
				const result = runtime.apply(this, args);
				updatePublicPath();
				return result;
			};
			entry[2].__mcpmatePatched = true;
		};
		const ensureChunkArrayPatched = () => {
			try {
				const chunkArray = window.webpackChunk_N_E;
				if (!chunkArray) return;
				if (!chunkArray.__mcpmatePatched) {
					const originalPush = chunkArray.push;
					if (typeof originalPush === "function") {
						chunkArray.push = function patchedPush(...args) {
							for (let i = 0; i < args.length; i += 1) {
								patchRuntimeEntry(args[i]);
							}
							updatePublicPath();
							const res = originalPush.apply(this, args);
							updatePublicPath();
							return res;
						};
						chunkArray.push.__mcpmatePatched = true;
					}
					chunkArray.__mcpmatePatched = true;
				}
				chunkArray.forEach(patchRuntimeEntry);
			} catch {
				/* noop */
			}
		};
		const ensureNextPublicPath = () => {
			updatePublicPath();
			ensureChunkArrayPatched();
		};
		const patchUrlSetter = (Ctor, property) => {
			try {
				if (!Ctor || !Ctor.prototype) return;
				const descriptor = Object.getOwnPropertyDescriptor(
					Ctor.prototype,
					property,
				);
				if (!descriptor || typeof descriptor.set !== "function") return;
				const originalSet = descriptor.set;
				const originalGet = descriptor.get;
				Object.defineProperty(Ctor.prototype, property, {
					configurable: descriptor.configurable !== false,
					enumerable: descriptor.enumerable ?? false,
					get: originalGet
						? function get() {
								return originalGet.call(this);
							}
						: undefined,
					set(value) {
						let mapped = value;
						try {
							mapped = mapUrl(value);
						} catch {
							mapped = value;
						}
						return originalSet.call(this, mapped);
					},
				});
			} catch {
				/* noop */
			}
		};
		const patchSetAttribute = () => {
			try {
				const rawSetAttribute = Element.prototype.setAttribute;
				if (!rawSetAttribute || rawSetAttribute.__mcpmatePatched) return;
				const shouldMap = new Set(["src", "href", "action", "data"]);
				const shouldMapStartsWith = ["data-src", "data-href", "data-url"];
				const patched = function (name, value) {
					let nextValue = value;
					try {
						if (typeof name === "string") {
							const lower = name.toLowerCase();
							if (
								shouldMap.has(lower) ||
								shouldMapStartsWith.some((item) => lower.startsWith(item))
							) {
								nextValue = mapUrl(value);
							}
						}
					} catch {
						nextValue = value;
					}
					return rawSetAttribute.call(this, name, nextValue);
				};
				patched.__mcpmatePatched = true;
				Element.prototype.setAttribute = patched;
			} catch {
				/* noop */
			}
		};

		const rewriteNodeResource = (node) => {
			try {
				if (!node || !node.tagName) return;
				const tag = String(node.tagName || "").toLowerCase();
				if (tag === "script") {
					if (node.src) {
						const mapped = mapUrl(node.src);
						if (mapped && mapped !== node.src) node.src = mapped;
					}
				} else if (tag === "link") {
					if (node.href) {
						const mapped = mapUrl(node.href);
						if (mapped && mapped !== node.href) node.href = mapped;
					}
				} else if (tag === "img") {
					if (node.src) {
						const mapped = mapUrl(node.src);
						if (mapped && mapped !== node.src) node.src = mapped;
					}
				}
			} catch {
				/* noop */
			}
		};

		const patchInsertionHooks = () => {
			try {
				const rawAppend = Element.prototype.appendChild;
				if (!rawAppend.__mcpmatePatched) {
					Element.prototype.appendChild = function (child) {
						rewriteNodeResource(child);
						return rawAppend.call(this, child);
					};
					Element.prototype.appendChild.__mcpmatePatched = true;
				}
				const rawInsert = Element.prototype.insertBefore;
				if (!rawInsert.__mcpmatePatched) {
					Element.prototype.insertBefore = function (child, ref) {
						rewriteNodeResource(child);
						return rawInsert.call(this, child, ref);
					};
					Element.prototype.insertBefore.__mcpmatePatched = true;
				}
			} catch {
				/* noop */
			}
		};
		if (!window.__mcpmatePatchedUrlSetters) {
			window.__mcpmatePatchedUrlSetters = true;
			patchUrlSetter(window.HTMLScriptElement, "src");
			patchUrlSetter(window.HTMLLinkElement, "href");
			patchUrlSetter(window.HTMLImageElement, "src");
			patchUrlSetter(window.HTMLAnchorElement, "href");
			patchSetAttribute();
			patchInsertionHooks();
		}
		ensureNextPublicPath();
		emitToParent("info", "market-shim loaded", {
			portalId,
			adapter: adapterId,
			prefix: prefixWithSlash,
		});
		setupNextjsErrorHandling();
		const rewriteLogoElement = (node) => {
			try {
				if (!node) return;
				const original = node.getAttribute("data-original-url");
				if (original) {
					const mapped = mapUrl(original);
					if (mapped !== original)
						node.setAttribute("data-original-url", mapped);
				}
				const inlineBg = node.style?.backgroundImage;
				if (inlineBg?.includes("url(")) {
					const match = inlineBg.match(/url\(([^)]+)\)/);
					const raw = match?.[1] ? match[1].replace(/^['"]|['"]$/g, "") : null;
					if (raw) {
						const mapped = mapUrl(raw);
						if (mapped !== raw) {
							node.style.backgroundImage = `url(${mapped})`;
						}
					}
				}
				const dataLogo = node.getAttribute("data-logo-url");
				if (dataLogo) {
					const mapped = mapUrl(dataLogo);
					if (mapped !== dataLogo) node.setAttribute("data-logo-url", mapped);
				}
				const targetIcon = original ? mapUrl(original) : null;
				if (
					targetIcon &&
					node.getAttribute("data-mcpmate-icon-url") !== targetIcon
				) {
					node.setAttribute("data-mcpmate-icon-url", targetIcon);
					try {
						const img = new Image();
						img.onload = () => {
							try {
								node.style.backgroundImage = `url(${targetIcon})`;
								node.setAttribute("data-mcpmate-icon-loaded", "1");
							} catch {
								/* noop */
							}
						};
						img.src = targetIcon;
					} catch {
						/* noop */
					}
				}
			} catch {
				/* noop */
			}
		};
		const scanLogos = (root) => {
			try {
				(root || document)
					.querySelectorAll(".server-logo")
					.forEach(rewriteLogoElement);
			} catch {
				/* noop */
			}
		};
		const rewriteAnchors = (root) => {
			try {
				(root || document).querySelectorAll("a[href]").forEach((anchor) => {
					const href = anchor.getAttribute("href");
					const mapped = mapUrl(href);
					if (mapped !== href) anchor.setAttribute("href", mapped);
				});
				(root || document).querySelectorAll("form[action]").forEach((form) => {
					const action = form.getAttribute("action");
					const mapped = mapUrl(action);
					if (mapped !== action) form.setAttribute("action", mapped);
				});
			} catch {
				/* noop */
			}
		};
		const ensureStyles = () => {
			const id = "mcpmate-market-outline-style";
			if (document.getElementById(id)) return;

			// Load external CSS file
			const link = document.createElement("link");
			link.id = id;
			link.rel = "stylesheet";
			link.href = `${prefix}/scripts/market/market-style.css`;
			document.head.appendChild(link);
		};
		const cleanupKey = "__mcpmateMarketCleanup";
		const stateKey = "__mcpmateMarketCleanupState";
		const previousCleanup =
			typeof window[cleanupKey] === "function" ? window[cleanupKey] : null;
		if (previousCleanup) {
			try {
				previousCleanup();
			} catch {
				/* noop */
			}
		}
		const state = { intervalId: null, observer: null };
		const cleanup = () => {
			try {
				if (typeof state.intervalId === "number") {
					clearInterval(state.intervalId);
				}
			} catch {
				/* noop */
			}
			state.intervalId = null;
			try {
				if (state.observer) {
					state.observer.disconnect();
				}
			} catch {
				/* noop */
			}
			state.observer = null;
			try {
				window.removeEventListener("beforeunload", cleanup);
			} catch {
				/* noop */
			}
			try {
				window.removeEventListener("pagehide", cleanup);
			} catch {
				/* noop */
			}
			window[cleanupKey] = undefined;
			window[stateKey] = undefined;
		};
		try {
			window.addEventListener("beforeunload", cleanup, { once: true });
		} catch {
			/* noop */
		}
		try {
			window.addEventListener("pagehide", cleanup, { once: true });
		} catch {
			/* noop */
		}
		window[cleanupKey] = cleanup;
		window[stateKey] = state;
		const _fetch = window.fetch.bind(window);
		window.fetch = (input, init) => {
			const mapped =
				typeof input === "string"
					? mapUrl(input)
					: input?.url
						? mapUrl(input.url)
						: input;
			if (typeof input === "string") return _fetch(mapped, init);
			if (input && typeof Request !== "undefined" && input instanceof Request) {
				const r = new Request(mapped, input);
				return _fetch(r, init);
			}
			return _fetch(mapped, init);
		};
		const _open = window.XMLHttpRequest?.prototype.open;
		if (_open) {
			window.XMLHttpRequest.prototype.open = function (method, url, ...rest) {
				const mapped = mapUrl(url);
				return _open.apply(this, [method, mapped, ...rest]);
			};
		}
		try {
			window.open = (url) => {
				try {
					if (url) window.location.href = mapUrl(url);
				} catch {
					/* noop */
				}
				return window;
			};
			document.addEventListener(
				"click",
				(e) => {
					const a =
						e.target && (e.target.closest ? e.target.closest("a[href]") : null);
					if (!a) return;
					const href = a.getAttribute("href");
					if (!href) return;

					// Check if this is an internal link (same origin or relative path)
					const isInternalLink = (() => {
						try {
							// Relative paths are internal
							if (
								href.startsWith("/") ||
								href.startsWith("./") ||
								href.startsWith("../")
							) {
								return true;
							}
							// Check if absolute URL is same origin
							const linkUrl = new URL(href, window.location.href);
							return linkUrl.origin === window.location.origin;
						} catch {
							return false;
						}
					})();

					// For internal links or _blank links, prevent default and navigate manually
					// This bypasses Next.js client-side routing which doesn't understand our proxy paths
					if (isInternalLink || a.getAttribute("target") === "_blank") {
						e.preventDefault();
						e.stopPropagation();
						window.location.href = mapUrl(href);
					}
				},
				{ capture: true },
			);
		} catch {
			/* noop */
		}
		const isPlainObject = (value) =>
			Boolean(value) && typeof value === "object" && !Array.isArray(value);
		const looksLikeServerEntry = (entry) => {
			if (!isPlainObject(entry)) return false;
			const hasCommand =
				typeof entry.command === "string" || typeof entry.launch === "string";
			const hasArgs = Array.isArray(entry.args) && entry.args.length > 0;
			const hasEnv =
				isPlainObject(entry.env) && Object.keys(entry.env).length > 0;
			const hasTransport =
				typeof entry.kind === "string" ||
				typeof entry.type === "string" ||
				typeof entry.transport === "string" ||
				typeof entry.url === "string";
			return hasCommand || hasArgs || hasEnv || hasTransport;
		};
		const looksLikeServersCollection = (value) => {
			if (Array.isArray(value)) {
				return value.some(looksLikeServerEntry);
			}
			if (isPlainObject(value)) {
				return Object.values(value).some(looksLikeServerEntry);
			}
			return false;
		};
		const looksLikeMcpJson = (obj) => {
			if (!isPlainObject(obj)) return false;
			if (obj.mcp && isPlainObject(obj.mcp)) {
				if (looksLikeServersCollection(obj.mcp.servers)) return true;
				if (looksLikeServersCollection(obj.mcp.server)) return true;
			}
			if (looksLikeServersCollection(obj.mcpServers)) return true;
			if (looksLikeServersCollection(obj.servers)) return true;
			if (looksLikeServerEntry(obj)) return true;
			return false;
		};
		const tryParseJsonConfig = (rawText) => {
			const trimmed = rawText.trim();
			if (!trimmed) return false;
			const candidate = (() => {
				if (/^[[{]/.test(trimmed)) return trimmed;
				const firstBrace = trimmed.indexOf("{");
				const lastBrace = trimmed.lastIndexOf("}");
				if (firstBrace >= 0 && lastBrace > firstBrace) {
					return trimmed.slice(firstBrace, lastBrace + 1);
				}
				return trimmed;
			})();
			try {
				const parsed = JSON.parse(candidate);
				return looksLikeMcpJson(parsed) ? candidate : false;
			} catch {
				return false;
			}
		};
		const looksLikeTomlConfig = (rawText) => {
			const text = rawText.trim();
			if (!text) return false;
			const hasSection =
				/\[\s*(?:mcp(?:\.[^\]]+)?|mcp_servers[^\]]*|servers)\s*\]/i.test(
					text,
				) || /\[\[\s*servers\s*\]\]/i.test(text);
			const keyHits = text.match(
				/^(\s*)(command|args|env|type|kind|transport|url|binary)\s*=\s*/gim,
			);
			if (hasSection && keyHits && keyHits.length >= 1) return text;
			if (keyHits && keyHits.length >= 2) return text;
			return false;
		};
		const defaultDetectConfigSnippet = (text) => {
			const content = text.trim();
			if (!content) return null;
			const jsonCandidate = tryParseJsonConfig(content);
			if (jsonCandidate) {
				return { format: "json", text: jsonCandidate };
			}
			const tomlCandidate = looksLikeTomlConfig(content);
			if (tomlCandidate) {
				return { format: "toml", text: tomlCandidate };
			}
			return null;
		};
		const detectConfigSnippet = (text) => {
			if (typeof adapter.detectConfigSnippet === "function") {
				try {
					const result = adapter.detectConfigSnippet(
						text,
						defaultDetectConfigSnippet,
					);
					if (result) return result;
				} catch (_error) {
					/* noop */
				}
			}
			return defaultDetectConfigSnippet(text);
		};
		const toRemoteUrl = () => {
			try {
				const current = new URL(window.location.href);
				const withSlash = prefixWithSlash;
				const withoutSlash = prefix;
				let remainder = null;
				if (current.pathname === withoutSlash) {
					remainder = "/";
				} else if (current.pathname.startsWith(withSlash)) {
					const raw = current.pathname.slice(withSlash.length);
					remainder = raw ? (raw.startsWith("/") ? raw : `/${raw}`) : "/";
				}
				if (remainder !== null) {
					const remote = new URL(remainder, remoteOrigin);
					remote.search = current.search;
					remote.hash = current.hash;
					return remote.toString();
				}
				return current.toString();
			} catch {
				return window.location.href;
			}
		};
		const ensureImportButton = (pre, detection) => {
			let button = pre.querySelector(".mcpmate-config-button");
			if (!button) {
				button = document.createElement("button");
				button.type = "button";
				button.className = "mcpmate-config-button";
				button.setAttribute("aria-label", "Import configuration into MCPMate");
				const icon = document.createElement("span");
				icon.className = "mcpmate-config-icon";
				const label = document.createElement("span");
				label.className = "mcpmate-config-label";
				label.textContent = "MCPMate";
				button.append(icon);
				button.addEventListener("click", (event) => {
					event.preventDefault();
					event.stopPropagation();
					const payload = {
						type: "mcpmate-market-import",
						payload: {
							text:
								typeof button.__mcpmateConfig === "string"
									? button.__mcpmateConfig
									: "",
							format: button.dataset.mcpmateFormat ?? "unknown",
							source: toRemoteUrl(),
							portalId,
							adapter: adapterId,
						},
					};
					try {
						if (window?.parent && window.parent !== window) {
							window.parent.postMessage(payload, "*");
						}
					} catch {
						/* noop */
					}
				});
				pre.appendChild(button);
			}
			button.dataset.mcpmateFormat = detection.format;
			button.dataset.mcpmatePortal = portalId;
			button.dataset.mcpmateAdapter = adapterId;
			button.querySelectorAll(".mcpmate-config-label").forEach((node) => {
				node.remove();
			});
			const textNode = document.createElement("span");
			textNode.className = "mcpmate-config-label";
			textNode.textContent = "MCPMate";
			button.appendChild(textNode);
			button.__mcpmateConfig = detection.text;
		};
		const removeImportButton = (pre) => {
			const button = pre.querySelector(".mcpmate-config-button");
			if (button) {
				delete button.__mcpmateConfig;
				button.remove();
			}
		};
		const mark = (root) => {
			try {
				const scope = root || document;
				ensureStyles();
				scope.querySelectorAll("pre").forEach((pre) => {
					const code = pre.querySelector("code");
					const snippet = code?.textContent ?? pre.textContent ?? "";
					const detection = detectConfigSnippet(snippet);
					if (detection) {
						pre.classList.add("mcpmate-code-outline");
						pre.setAttribute("data-mcpmate-highlight", detection.format);
						ensureImportButton(pre, detection);
					} else {
						pre.classList.remove("mcpmate-code-outline");
						pre.removeAttribute("data-mcpmate-highlight");
						removeImportButton(pre);
					}
				});
				rewriteAnchors(root);
			} catch {
				/* noop */
			}
		};
		const initialReady = () => {
			mark(document);
			scanLogos(document);
			rewriteAnchors(document);
			notifyReady();
		};

		if (document.readyState === "loading") {
			document.addEventListener(
				"DOMContentLoaded",
				() => {
					initialReady();
				},
				{ once: true },
			);
		} else {
			initialReady();
		}
		state.intervalId = window.setInterval(() => {
			ensureNextPublicPath();
			mark(document);
			scanLogos(document);
			rewriteAnchors(document);
		}, 1000);
		const target = document.getElementById("readme-content") || document.body;
		try {
			const mo = new MutationObserver((muts) => {
				for (const m of muts) {
					if (m.addedNodes) {
						m.addedNodes.forEach((n) => {
							if (n && n.nodeType === 1) {
								mark(n);
								scanLogos(n);
								rewriteAnchors(n);
							}
						});
					}
				}
				notifyReady();
			});
			state.observer = mo;
			mo.observe(target, { childList: true, subtree: true });
		} catch {
			/* noop */
		}
	} catch (e) {
		console.warn("[mcpmate] market shim failed", e);
	}
})();
