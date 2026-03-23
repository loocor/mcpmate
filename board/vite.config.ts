import { readFileSync } from "node:fs";
import type { ClientRequest } from "node:http";
import path from "node:path";
import react from "@vitejs/plugin-react";
import type { Plugin, ViteDevServer } from "vite";
import { defineConfig } from "vite";

const packageJson = JSON.parse(
	readFileSync(new URL("./package.json", import.meta.url), "utf8"),
) as { version?: string };

const appVersion =
	typeof packageJson.version === "string" ? packageJson.version : "";

type HttpProxyError = Error & { code?: string };

type HttpProxyResponse = {
	writeHead(statusCode: number, headers?: Record<string, string | number>): void;
	end(chunk?: string): void;
};

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
		if (error.code === "ECONNREFUSED" && res) {
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

function marketProxyPlugin(): Plugin {
	return {
		name: "mcpmate-market-proxy",
		configureServer(_server: ViteDevServer) {
			// Portal proxy disabled - third-party market support removed
			// Kept as a placeholder for potential future third-party market support
		},
	};
}

export default defineConfig({
	define: {
		"import.meta.env.VITE_APP_VERSION": JSON.stringify(appVersion),
	},
	plugins: [react(), marketProxyPlugin()],
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
			"/api": {
				target: "http://127.0.0.1:8080",
				changeOrigin: true,
				secure: false,
				configure: (proxy: HttpProxyServer) => {
					attachBackendStartupProxyHandler(proxy);
					proxy.on("proxyReq", (proxyReq: ClientRequest) => {
						if (proxyReq && typeof proxyReq.removeHeader === "function") {
							try {
								proxyReq.removeHeader("origin");
							} catch {
								/* noop */
							}
						}
					});
				},
			},
			"/ws": {
				target: "ws://127.0.0.1:8080",
				ws: true,
				changeOrigin: true,
				secure: false,
				configure: (proxy: HttpProxyServer) => {
					attachBackendStartupProxyHandler(proxy);
					proxy.on("proxyReqWs", (proxyReq: ClientRequest) => {
						if (proxyReq && typeof proxyReq.removeHeader === "function") {
							try {
								proxyReq.removeHeader("origin");
							} catch {
								/* noop */
							}
						}
					});
				},
			},
		},
	},
});
