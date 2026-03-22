/**
 * Auth worker (Cloudflare Worker) base URL — must match desktop `embed.env` `AUTH_WORKER_BASE`.
 * Set `VITE_AUTH_WORKER_BASE` in `.env` when pointing at a staging worker.
 *
 * GitHub OAuth App (same Client ID as the worker) must register this exact callback URL
 * (scheme, host, path; no trailing slash unless the worker sends one):
 * `{AUTH_WORKER_BASE}/auth/github/callback`
 * If GitHub shows “redirect_uri is not associated with this application”, the callback URL
 * in GitHub Developer Settings does not match what the worker sends.
 *
 * OAuth `state` for the MCPMate worker is stored in **Workers KV** (`auth/src/index.ts`); the
 * Worker must **await** the KV put before redirecting to GitHub, or the desktop app may show
 * `invalid_state`. Cookie-based state (if used elsewhere) needs **SameSite=Lax**, not Strict.
 */
function trimTrailingSlash(url: string): string {
	return url.replace(/\/+$/, "");
}

export const AUTH_WORKER_BASE = trimTrailingSlash(
	(typeof import.meta !== "undefined" &&
		import.meta.env?.VITE_AUTH_WORKER_BASE &&
		String(import.meta.env.VITE_AUTH_WORKER_BASE).trim()) ||
		"https://auth.mcp.umate.ai",
);

export const AUTH_GITHUB_LOGIN_URL = `${AUTH_WORKER_BASE}/auth/github`;
