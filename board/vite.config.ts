import type {
	ClientRequest,
	IncomingMessage,
	ServerResponse,
} from "node:http";
import path from "node:path";
import react from "@vitejs/plugin-react";
import type { Plugin, ViteDevServer } from "vite";
import { defineConfig } from "vite";

type HttpProxyServer = {
	on(event: "proxyReq", listener: (proxyReq: ClientRequest) => void): void;
	on(event: "proxyReqWs", listener: (proxyReq: ClientRequest) => void): void;
	on(event: string, listener: (...args: unknown[]) => void): void;
};

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
