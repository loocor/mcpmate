import { readFileSync } from "node:fs";
import type { ClientRequest } from "node:http";
import path from "node:path";
import react from "@vitejs/plugin-react";
import topLevelAwait from "vite-plugin-top-level-await";
import wasm from "vite-plugin-wasm";
import { defineConfig } from "vite";

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

function attachBackendStartupProxyHandler(proxy: HttpProxyServer): void {
	proxy.on("error", (error, _req, res) => {
		if (error.code === "ECONNREFUSED" && isHttpProxyResponse(res)) {
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

export default defineConfig({
	define: {
		"import.meta.env.VITE_APP_VERSION": JSON.stringify(appVersion),
	},
	plugins: [react(), wasm(), topLevelAwait()],
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
				configure: (proxy: HttpProxyServer) => {
					attachBackendStartupProxyHandler(proxy);
					proxy.on("proxyReq", (proxyReq: ClientRequest) => {
						removeOriginHeader(proxyReq);
					});
				},
			},
			"^/docs(?:/|$)": {
				target: devApiBaseUrl,
				changeOrigin: true,
				secure: false,
				configure: (proxy: HttpProxyServer) => {
					attachBackendStartupProxyHandler(proxy);
					proxy.on("proxyReq", (proxyReq: ClientRequest) => {
						removeOriginHeader(proxyReq);
					});
				},
			},
			"^/openapi\\.json$": {
				target: devApiBaseUrl,
				changeOrigin: true,
				secure: false,
				configure: (proxy: HttpProxyServer) => {
					attachBackendStartupProxyHandler(proxy);
					proxy.on("proxyReq", (proxyReq: ClientRequest) => {
						removeOriginHeader(proxyReq);
					});
				},
			},
			"^/ws(?:/|$)": {
				target: devWsBaseUrl,
				ws: true,
				changeOrigin: true,
				secure: false,
				configure: (proxy: HttpProxyServer) => {
					attachBackendStartupProxyHandler(proxy);
					proxy.on("proxyReqWs", (proxyReq: ClientRequest) => {
						removeOriginHeader(proxyReq);
					});
				},
			},
		},
	},
});
