import { useCallback, useEffect, useState } from "react";
import type { GitHubLatestRelease, PublicDownloadManifest } from "../utils/githubRelease";
import { DOWNLOADS_MANIFEST_API_URL, releaseFromDownloadManifest } from "../utils/githubRelease";

export type ReleaseFetchState =
	| { status: "loading" }
	| { status: "error"; message: string }
	| {
			status: "ok";
			latest: GitHubLatestRelease;
	  };

/**
 * Loads the public download manifest and maps installer assets to admin redirect URLs.
 */
export function useLatestGitHubRelease(): ReleaseFetchState & { refetch: () => void } {
	const [state, setState] = useState<ReleaseFetchState>({ status: "loading" });
	const [tick, setTick] = useState(0);

	useEffect(() => {
		const ac = new AbortController();
		setState({ status: "loading" });

		void (async () => {
			try {
				const latestRes = await fetch(DOWNLOADS_MANIFEST_API_URL, { signal: ac.signal });

				if (ac.signal.aborted) {
					return;
				}
				if (!latestRes.ok) {
					setState({ status: "error", message: `latest HTTP ${latestRes.status}` });
					return;
				}

				const manifest = (await latestRes.json()) as PublicDownloadManifest;
				if (
					manifest?.schemaVersion !== 1 ||
					!manifest.tag ||
					!manifest.releaseUrl ||
					!manifest.assets ||
					typeof manifest.assets !== "object"
				) {
					setState({ status: "error", message: "Invalid download manifest payload" });
					return;
				}

				setState({ status: "ok", latest: releaseFromDownloadManifest(manifest) });
			} catch (e) {
				if (ac.signal.aborted) {
					return;
				}
				setState({ status: "error", message: (e as Error).message || "fetch failed" });
			}
		})();

		return () => ac.abort();
	}, [tick]);

	const refetch = useCallback(() => {
		setTick((n) => n + 1);
	}, []);

	return { ...state, refetch };
}
