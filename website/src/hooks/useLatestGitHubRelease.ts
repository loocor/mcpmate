import { useCallback, useEffect, useState } from "react";
import type { GitHubLatestRelease } from "../utils/githubRelease";
import { LATEST_RELEASE_API_URL, fetchAllPublishedReleases } from "../utils/githubRelease";

export type ReleaseFetchState =
	| { status: "loading" }
	| { status: "ok"; latest: GitHubLatestRelease; allReleases: GitHubLatestRelease[] }
	| { status: "error"; message: string };

/**
 * Loads the latest release (install URLs) plus all published releases (cumulative download counts).
 */
export function useLatestGitHubRelease(): ReleaseFetchState & { refetch: () => void } {
	const [state, setState] = useState<ReleaseFetchState>({ status: "loading" });
	const [tick, setTick] = useState(0);

	useEffect(() => {
		const ac = new AbortController();
		setState({ status: "loading" });

		void (async () => {
			try {
				const [latestRes, allReleases] = await Promise.all([
					fetch(LATEST_RELEASE_API_URL, {
						signal: ac.signal,
						headers: { Accept: "application/vnd.github+json" },
					}),
					fetchAllPublishedReleases(ac.signal),
				]);

				if (ac.signal.aborted) {
					return;
				}
				if (!latestRes.ok) {
					setState({ status: "error", message: `latest HTTP ${latestRes.status}` });
					return;
				}
				const latest = (await latestRes.json()) as GitHubLatestRelease;
				if (!latest?.tag_name || !Array.isArray(latest.assets)) {
					setState({ status: "error", message: "Invalid latest payload" });
					return;
				}
				setState({ status: "ok", latest, allReleases });
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
