import { readFileSync } from "node:fs";
import type { ClientRequest, IncomingMessage } from "node:http";
import { homedir } from "node:os";
import path from "node:path";
import react from "@vitejs/plugin-react";
import topLevelAwait from "vite-plugin-top-level-await";
import wasm from "vite-plugin-wasm";
import { defineConfig, type Plugin } from "vite";

const packageJson = JSON.parse(
	readFileSync(new URL("./package.json", import.meta.url), "utf8"),
) as { version?: string };

const appVersion =
	typeof packageJson.version === "string" ? packageJson.version : "";

function readDevSettingsApiBaseUrl(): string | null {
	const dataDir =
		typeof process.env.MCPMATE_DATA_DIR === "string" &&
		process.env.MCPMATE_DATA_DIR.trim().length > 0
			? process.env.MCPMATE_DATA_DIR.trim()
			: path.join(homedir(), ".mcpmate");
	const configPath = path.join(dataDir, "config.json");
	try {
		const payload = JSON.parse(readFileSync(configPath, "utf8")) as {
			api_port?: unknown;
		};
		if (
			typeof payload.api_port === "number" &&
			Number.isInteger(payload.api_port) &&
			payload.api_port > 0
		) {
			return `http://127.0.0.1:${payload.api_port}`;
		}
	} catch {
		return null;
	}
	return null;
}

let cachedDevSettingsApiBaseUrl: {
	value: string | null;
	expiresAt: number;
} | null = null;

function readCachedDevSettingsApiBaseUrl(): string | null {
	const now = Date.now();
	if (cachedDevSettingsApiBaseUrl && cachedDevSettingsApiBaseUrl.expiresAt > now) {
		return cachedDevSettingsApiBaseUrl.value;
	}
	const value = readDevSettingsApiBaseUrl();
	cachedDevSettingsApiBaseUrl = {
		value,
		expiresAt: now + 1_000,
	};
	return value;
}

const devApiBaseUrl =
	typeof process.env.VITE_API_BASE_URL === "string" &&
	process.env.VITE_API_BASE_URL.trim().length > 0
		? process.env.VITE_API_BASE_URL.trim()
		: (readDevSettingsApiBaseUrl() ?? "http://127.0.0.1:8080");

function devWsBaseUrlFromApiBase(apiBaseUrl: string, fallback: string): string {
	try {
		const parsed = new URL(apiBaseUrl);
		parsed.protocol = parsed.protocol === "https:" ? "wss:" : "ws:";
		return parsed.toString();
	} catch {
		return fallback;
	}
}

const devWsBaseUrl = devWsBaseUrlFromApiBase(devApiBaseUrl, "ws://127.0.0.1:8080");

function currentDevApiBaseUrl(): string {
	return readCachedDevSettingsApiBaseUrl() ?? devApiBaseUrl;
}

function currentDevWsBaseUrl(): string {
	return devWsBaseUrlFromApiBase(currentDevApiBaseUrl(), devWsBaseUrl);
}

type HttpProxyError = Error & { code?: string };

type HttpProxyResponse = {
	headersSent?: boolean;
	writeHead(statusCode: number, headers?: Record<string, string | number>): void;
	end(chunk?: string): void;
};

function isHttpProxyResponse(value: unknown): value is HttpProxyResponse {
	if (!value || typeof value !== "object") {
		return false;
	}
	const candidate = value as {
		writeHead?: unknown;
		end?: unknown;
	};
	return typeof candidate.writeHead === "function" && typeof candidate.end === "function";
}

type HttpProxyServer = {
	on(event: "proxyReq", listener: (proxyReq: ClientRequest) => void): void;
	on(event: "proxyReqWs", listener: (proxyReq: ClientRequest) => void): void;
	on(
		event: "error",
		listener: (
			error: HttpProxyError,
			_req: unknown,
			res: HttpProxyResponse | undefined,
		) => void,
	): void;
	on(event: string, listener: (...args: unknown[]) => void): void;
};

const BACKEND_READINESS_PROXY_LOG_INTERVAL_MS = 5_000;
const DEV_CORE_SOURCE_PATH = "/__mcpmate/dev-core-source";

function backendReadinessTargetLabel(): string {
	const apiBaseUrl = currentDevApiBaseUrl();
	try {
		const parsed = new URL(apiBaseUrl);
		return `${parsed.host}/api/system/readiness`;
	} catch {
		return `${apiBaseUrl}/api/system/readiness`;
	}
}

function isBackendReadinessRequest(req: IncomingMessage): boolean {
	const method = req.method?.toUpperCase();
	if (method !== "GET" && method !== "HEAD") {
		return false;
	}
	const pathName = req.url?.split(/[?#]/, 1)[0];
	return pathName === "/api/system/readiness";
}

function isBackendStartupError(error: unknown): boolean {
	const candidates: unknown[] = [error];
	if (error && typeof error === "object" && "cause" in error) {
		candidates.push((error as { cause?: unknown }).cause);
	}
	return candidates.some((candidate) => {
		if (!candidate || typeof candidate !== "object" || !("code" in candidate)) {
			return false;
		}
		return (candidate as { code?: unknown }).code === "ECONNREFUSED";
	});
}

function createBackendReadinessProxyLogger() {
	let unavailableSinceMs: number | null = null;
	let lastLoggedAtMs = 0;
	let suppressedCount = 0;

	return {
		recordStartupError(): void {
			const nowMs = Date.now();
			if (unavailableSinceMs === null) {
				unavailableSinceMs = nowMs;
			}
			suppressedCount += 1;
			if (
				lastLoggedAtMs !== 0 &&
				nowMs - lastLoggedAtMs < BACKEND_READINESS_PROXY_LOG_INTERVAL_MS
			) {
				return;
			}
			const waitedSeconds = Math.round((nowMs - unavailableSinceMs) / 1_000);
			const repeats = suppressedCount;
			lastLoggedAtMs = nowMs;
			suppressedCount = 0;
			console.warn(
				`[vite] backend readiness proxy waiting: ${backendReadinessTargetLabel()} ECONNREFUSED (x${repeats}, ${waitedSeconds}s)`,
			);
		},
		recordSuccess(): void {
			if (unavailableSinceMs === null) {
				return;
			}
			const nowMs = Date.now();
			const waitedSeconds = Math.round((nowMs - unavailableSinceMs) / 1_000);
			const suppressedSuffix =
				suppressedCount > 0 ? `, suppressed ${suppressedCount} repeat(s)` : "";
			console.info(
				`[vite] backend readiness proxy recovered: ${backendReadinessTargetLabel()} (${waitedSeconds}s${suppressedSuffix})`,
			);
			unavailableSinceMs = null;
			lastLoggedAtMs = 0;
			suppressedCount = 0;
		},
	};
}

function writeJsonResponse(
	res: HttpProxyResponse,
	statusCode: number,
	payload: Record<string, unknown>,
): void {
	if (res.headersSent) {
		return;
	}
	res.writeHead(statusCode, {
		"Cache-Control": "no-store",
		"Content-Type": "application/json",
	});
	res.end(JSON.stringify(payload));
}

function writeBackendStartingResponse(res: HttpProxyResponse): void {
	writeJsonResponse(res, 503, {
		success: false,
		error: { message: "Backend is starting" },
	});
}

function requestHeadersForBackend(req: IncomingMessage): Headers {
	const headers = new Headers();
	for (const [key, value] of Object.entries(req.headers)) {
		if (key === "host" || key === "origin") {
			continue;
		}
		if (Array.isArray(value)) {
			for (const item of value) {
				headers.append(key, item);
			}
			continue;
		}
		if (typeof value === "string") {
			headers.set(key, value);
		}
	}
	return headers;
}

function devCoreSourcePlugin(): Plugin {
	return {
		name: "mcpmate-dev-core-source",
		configureServer(server) {
			server.middlewares.use((req, res, next) => {
				const pathName = req.url?.split(/[?#]/, 1)[0];
				if (pathName !== DEV_CORE_SOURCE_PATH) {
					next();
					return;
				}
				if (req.method?.toUpperCase() !== "GET") {
					writeJsonResponse(res, 405, {
						success: false,
						error: { message: "Method not allowed" },
					});
					return;
				}
				const apiBaseUrl = currentDevApiBaseUrl();
				writeJsonResponse(res, 200, {
					apiBaseUrl,
				});
			});
		},
	};
}

function compressedBackendReadinessProxyPlugin(): Plugin {
	return {
		name: "mcpmate-compressed-backend-readiness-proxy",
		configureServer(server) {
			const readinessProxyLogger = createBackendReadinessProxyLogger();
			server.middlewares.use(async (req, res, next) => {
				if (!isBackendReadinessRequest(req)) {
					next();
					return;
				}
				try {
					const targetUrl = new URL(req.url ?? "/api/system/readiness", currentDevApiBaseUrl());
					const response = await fetch(targetUrl, {
						headers: requestHeadersForBackend(req),
						method: req.method,
					});
					readinessProxyLogger.recordSuccess();
					res.statusCode = response.status;
					response.headers.forEach((value, key) => {
						res.setHeader(key, value);
					});
					if (req.method?.toUpperCase() === "HEAD") {
						res.end();
						return;
					}
					res.end(Buffer.from(await response.arrayBuffer()));
				} catch (error) {
					if (isBackendStartupError(error)) {
						readinessProxyLogger.recordStartupError();
						writeBackendStartingResponse(res);
						return;
					}
					next();
				}
			});
		},
	};
}

function attachBackendStartupProxyHandler(proxy: HttpProxyServer): void {
	proxy.on("error", (error, _req, res) => {
		if (error.code === "ECONNREFUSED" && isHttpProxyResponse(res)) {
			writeBackendStartingResponse(res);
			return;
		}
	});
}

function removeOriginHeader(proxyReq: ClientRequest): void {
	if (!proxyReq || typeof proxyReq.removeHeader !== "function") {
		return;
	}

	try {
		proxyReq.removeHeader("origin");
	} catch {
		/* noop */
	}
}

function configureBackendProxy(
	proxy: HttpProxyServer,
	event: "proxyReq" | "proxyReqWs",
): void {
	attachBackendStartupProxyHandler(proxy);
	proxy.on(event, (proxyReq: ClientRequest) => {
		removeOriginHeader(proxyReq);
	});
}

const MANUAL_CHUNK_GROUPS: Array<[string, string[]]> = [
	[
		"react-vendor",
		[
			"/node_modules/react/",
			"/node_modules/react-dom/",
			"/node_modules/react-router-dom/",
			"/node_modules/scheduler/",
		],
	],
	[
		"ui-vendor",
		[
			"/node_modules/@radix-ui/",
			"/node_modules/class-variance-authority/",
			"/node_modules/clsx/",
			"/node_modules/cmdk/",
			"/node_modules/lucide-react/",
			"/node_modules/tailwind-merge/",
			"/node_modules/vaul/",
		],
	],
	[
		"data-vendor",
		[
			"/node_modules/@tanstack/",
			"/node_modules/date-fns/",
			"/node_modules/zod/",
			"/node_modules/zustand/",
		],
	],
	[
		"chart-vendor",
		[
			"/node_modules/d3-",
			"/node_modules/recharts/",
			"/node_modules/victory-vendor/",
		],
	],
	[
		"markdown-vendor",
		[
			"/node_modules/hast-",
			"/node_modules/mdast-",
			"/node_modules/micromark",
			"/node_modules/react-markdown/",
			"/node_modules/rehype-",
			"/node_modules/remark-",
			"/node_modules/unified/",
			"/node_modules/unist-",
			"/node_modules/vfile",
		],
	],
	[
		"desktop-vendor",
		[
			"/node_modules/@tauri-apps/",
		],
	],
	[
		"tokenizer-vendor",
		[
			"/node_modules/gpt-tokenizer/",
			"/node_modules/tiktoken/",
			"/src/lib/vendor/claude-tokenizer.json",
		],
	],
	[
		"i18n-vendor",
		[
			"/node_modules/i18next",
			"/node_modules/react-i18next/",
		],
	],
];

function manualChunks(id: string): string | undefined {
	const normalizedId = id.split(path.sep).join("/");
	for (const [chunkName, matches] of MANUAL_CHUNK_GROUPS) {
		if (matches.some((match) => normalizedId.includes(match))) {
			return chunkName;
		}
	}
	if (normalizedId.includes("/node_modules/")) {
		return "vendor";
	}
	return undefined;
}

export default defineConfig({
	define: {
		"import.meta.env.VITE_APP_VERSION": JSON.stringify(appVersion),
	},
	plugins: [
		devCoreSourcePlugin(),
		compressedBackendReadinessProxyPlugin(),
		react(),
		wasm(),
		topLevelAwait(),
	],
	resolve: {
		alias: {
			"@": path.resolve(__dirname, "./src"),
		},
	},
	optimizeDeps: {
		exclude: ["lucide-react"],
	},
	build: {
		chunkSizeWarningLimit: 5_000,
		rollupOptions: {
			output: {
				manualChunks,
			},
		},
	},
	server: {
		proxy: {
			"^/registry-api(?:/|$)": {
				target: "https://registry.modelcontextprotocol.io",
				changeOrigin: true,
				rewrite: (path: string) => path.replace(/^\/registry-api/, "/v0.1"),
			},
			"^/github-raw(?:/|$)": {
				target: "https://raw.githubusercontent.com",
				changeOrigin: true,
				rewrite: (path: string) => path.replace(/^\/github-raw/, ""),
			},
			"^/github-api(?:/|$)": {
				target: "https://api.github.com",
				changeOrigin: true,
				rewrite: (path: string) => path.replace(/^\/github-api/, ""),
			},
			"^/api(?:/|$)": {
				target: devApiBaseUrl,
				router: () => currentDevApiBaseUrl(),
				changeOrigin: true,
				secure: false,
				configure: (proxy: HttpProxyServer) => configureBackendProxy(proxy, "proxyReq"),
			},
			"^/docs(?:/|$)": {
				target: devApiBaseUrl,
				router: () => currentDevApiBaseUrl(),
				changeOrigin: true,
				secure: false,
				configure: (proxy: HttpProxyServer) => configureBackendProxy(proxy, "proxyReq"),
			},
			"^/openapi\\.json$": {
				target: devApiBaseUrl,
				router: () => currentDevApiBaseUrl(),
				changeOrigin: true,
				secure: false,
				configure: (proxy: HttpProxyServer) => configureBackendProxy(proxy, "proxyReq"),
			},
			"^/ws(?:/|$)": {
				target: devWsBaseUrl,
				router: () => currentDevWsBaseUrl(),
				ws: true,
				changeOrigin: true,
				secure: false,
				configure: (proxy: HttpProxyServer) => configureBackendProxy(proxy, "proxyReqWs"),
			},
		},
	},
});
