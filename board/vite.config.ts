import { readFileSync } from "node:fs";
import type { ClientRequest, IncomingMessage } from "node:http";
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

const devApiBaseUrl =
	typeof process.env.VITE_API_BASE_URL === "string" &&
	process.env.VITE_API_BASE_URL.trim().length > 0
		? process.env.VITE_API_BASE_URL.trim()
		: "http://127.0.0.1:8080";

const devWsBaseUrl = (() => {
	try {
		const parsed = new URL(devApiBaseUrl);
		parsed.protocol = parsed.protocol === "https:" ? "wss:" : "ws:";
		return parsed.toString();
	} catch {
		return "ws://127.0.0.1:8080";
	}
})();

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

function backendReadinessTargetLabel(): string {
	try {
		const parsed = new URL(devApiBaseUrl);
		return `${parsed.host}/api/system/readiness`;
	} catch {
		return `${devApiBaseUrl}/api/system/readiness`;
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
	const targetLabel = backendReadinessTargetLabel();
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
				`[vite] backend readiness proxy waiting: ${targetLabel} ECONNREFUSED (x${repeats}, ${waitedSeconds}s)`,
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
				`[vite] backend readiness proxy recovered: ${targetLabel} (${waitedSeconds}s${suppressedSuffix})`,
			);
			unavailableSinceMs = null;
			lastLoggedAtMs = 0;
			suppressedCount = 0;
		},
	};
}

function writeBackendStartingResponse(res: HttpProxyResponse): void {
	if (res.headersSent) {
		return;
	}
	res.writeHead(503, {
		"Content-Type": "application/json",
		"Retry-After": "1",
	});
	res.end(
		JSON.stringify({
			success: false,
			error: { message: "Backend is starting" },
		}),
	);
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
					const targetUrl = new URL(req.url ?? "/api/system/readiness", devApiBaseUrl);
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

export default defineConfig({
	define: {
		"import.meta.env.VITE_APP_VERSION": JSON.stringify(appVersion),
	},
	plugins: [compressedBackendReadinessProxyPlugin(), react(), wasm(), topLevelAwait()],
	resolve: {
		alias: {
			"@": path.resolve(__dirname, "./src"),
		},
	},
	optimizeDeps: {
		exclude: ["lucide-react"],
	},
	server: {
		proxy: {
			"^/api(?:/|$)": {
				target: devApiBaseUrl,
				changeOrigin: true,
				secure: false,
				configure: (proxy: HttpProxyServer) => configureBackendProxy(proxy, "proxyReq"),
			},
			"^/docs(?:/|$)": {
				target: devApiBaseUrl,
				changeOrigin: true,
				secure: false,
				configure: (proxy: HttpProxyServer) => configureBackendProxy(proxy, "proxyReq"),
			},
			"^/openapi\\.json$": {
				target: devApiBaseUrl,
				changeOrigin: true,
				secure: false,
				configure: (proxy: HttpProxyServer) => configureBackendProxy(proxy, "proxyReq"),
			},
			"^/ws(?:/|$)": {
				target: devWsBaseUrl,
				ws: true,
				changeOrigin: true,
				secure: false,
				configure: (proxy: HttpProxyServer) => configureBackendProxy(proxy, "proxyReqWs"),
			},
		},
	},
});
