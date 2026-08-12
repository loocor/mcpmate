import { useCallback, useEffect, useState } from "react";
import type {
	GitHubLatestRelease,
	PublicDownloadManifest,
	PublicDownloadManifestV2,
} from "../utils/githubRelease";
import {
	DOWNLOADS_MANIFEST_API_URL,
	exactDownloadsManifestApiUrl,
	releaseFromDownloadManifest,
} from "../utils/githubRelease";

export type ReleaseFetchState =
	| { status: "loading" }
	| { status: "error"; message: string }
	| {
			status: "ok";
			latest: GitHubLatestRelease;
	  };

function isBrowserOffline(): boolean {
	return typeof navigator !== "undefined" && navigator.onLine === false;
}

/**
 * Loads the public download manifest and maps installer assets to admin redirect URLs.
 */
export function useLatestGitHubRelease(): ReleaseFetchState & { refetch: () => void } {
	const [state, setState] = useState<ReleaseFetchState>({ status: "loading" });
	const [tick, setTick] = useState(0);
	const [offline, setOffline] = useState(isBrowserOffline);

	useEffect(() => {
		const ac = new AbortController();
		setState({ status: "loading" });

		void (async () => {
			try {
				if (offline) {
					setState({ status: "error", message: "offline" });
					return;
				}

				const latestRes = await fetch(DOWNLOADS_MANIFEST_API_URL, {
					cache: "no-store",
					signal: ac.signal,
				});

				if (ac.signal.aborted) {
					return;
				}
				if (!latestRes.ok) {
					setState({ status: "error", message: `latest HTTP ${latestRes.status}` });
					return;
				}

				const latestManifest = (await latestRes.json()) as PublicDownloadManifest;
				if (
					latestManifest?.schemaVersion !== 1 ||
					typeof latestManifest.tag !== "string" ||
					typeof latestManifest.releaseUrl !== "string" ||
					!latestManifest.assets ||
					typeof latestManifest.assets !== "object" ||
					Array.isArray(latestManifest.assets)
				) {
					setState({ status: "error", message: "Invalid download manifest payload" });
					return;
				}

				const exactRes = await fetch(exactDownloadsManifestApiUrl(latestManifest.tag), {
					cache: "no-store",
					signal: ac.signal,
				});
				if (ac.signal.aborted) {
					return;
				}
				if (!exactRes.ok) {
					setState({ status: "error", message: `exact HTTP ${exactRes.status}` });
					return;
				}

				const manifest = (await exactRes.json()) as PublicDownloadManifestV2;
				if (
					manifest?.schemaVersion !== 2 ||
					manifest.tag !== latestManifest.tag ||
					typeof manifest.releaseUrl !== "string" ||
					!manifest.assets ||
					typeof manifest.assets !== "object" ||
					Array.isArray(manifest.assets)
				) {
					setState({ status: "error", message: "Invalid exact download manifest payload" });
					return;
				}
				const latest = releaseFromDownloadManifest(manifest);
				if (!latest) {
					setState({ status: "error", message: "Invalid exact download manifest payload" });
					return;
				}

				if (isBrowserOffline()) {
					setState({ status: "error", message: "offline" });
					return;
				}

				setState({ status: "ok", latest });
			} catch (e) {
				if (ac.signal.aborted) {
					return;
				}
				setState({ status: "error", message: (e as Error).message || "fetch failed" });
			}
		})();

		return () => ac.abort();
	}, [offline, tick]);

	useEffect(() => {
		if (typeof window === "undefined") {
			return;
		}

		const handleOffline = () => {
			setOffline(true);
		};
		const handleOnline = () => {
			setOffline(false);
			setTick((n) => n + 1);
		};

		window.addEventListener("offline", handleOffline);
		window.addEventListener("online", handleOnline);
		return () => {
			window.removeEventListener("offline", handleOffline);
			window.removeEventListener("online", handleOnline);
		};
	}, []);

	const refetch = useCallback(() => {
		setTick((n) => n + 1);
	}, []);

	return { ...state, refetch };
}
