import { dirname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const port = Number(process.env.PORT || 5179);
const host = process.env.HOST || "127.0.0.1";

const MIME_TYPES = {
	".css": "text/css; charset=utf-8",
	".html": "text/html; charset=utf-8",
	".js": "text/javascript; charset=utf-8",
	".json": "application/json; charset=utf-8",
	".mjs": "text/javascript; charset=utf-8",
	".png": "image/png",
	".svg": "image/svg+xml",
};

function contentType(pathname) {
	const dot = pathname.lastIndexOf(".");
	if (dot === -1) {
		return "application/octet-stream";
	}
	return MIME_TYPES[pathname.slice(dot)] || "application/octet-stream";
}

function resolvePath(pathname) {
	const decoded = decodeURIComponent(pathname === "/" ? "/dev/popup.html" : pathname);
	const normalized = normalize(decoded).replace(/^(\.\.(\/|\\|$))+/, "");
	const absolute = join(rootDir, normalized);
	if (!absolute.startsWith(rootDir)) {
		return null;
	}
	return absolute;
}

const server = Bun.serve({
	hostname: host,
	port,
	async fetch(request) {
		const url = new URL(request.url);
		const filePath = resolvePath(url.pathname);
		if (!filePath) {
			return new Response("Forbidden", { status: 403 });
		}

		const file = Bun.file(filePath);
		if (!(await file.exists())) {
			return new Response("Not Found", { status: 404 });
		}

		return new Response(file, {
			headers: {
				"Content-Type": contentType(url.pathname),
				"Cache-Control": "no-store",
			},
		});
	},
});

console.log(`MCPMate extension dev preview: http://${host}:${server.port}/`);
console.log("Mock catalog (default): http://127.0.0.1:5179/dev/popup.html?mode=mock");
console.log("Live discovery API:     http://127.0.0.1:5179/dev/popup.html?mode=account");
