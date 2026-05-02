import { useCallback, useEffect, useState } from "react";
import type { GitHubLatestRelease } from "../utils/githubRelease";
import { LATEST_RELEASE_API_URL, fetchAllPublishedReleases } from "../utils/githubRelease";

export type ReleaseFetchState =
	| { status: "loading" }
	| { status: "error"; message: string }
	| {
			status: "ok";
			latest: GitHubLatestRelease;
			allReleases: GitHubLatestRelease[] | null;
			historyError?: string;
	  };

/**
 * Loads the latest release (install URLs) plus published release history (cumulative download counts).
 */
export function useLatestGitHubRelease(): ReleaseFetchState & { refetch: () => void } {
	const [state, setState] = useState<ReleaseFetchState>({ status: "loading" });
	const [tick, setTick] = useState(0);

	useEffect(() => {
		const ac = new AbortController();
		setState({ status: "loading" });

		void (async () => {
			try {
				const latestRes = await fetch(LATEST_RELEASE_API_URL, {
					signal: ac.signal,
					headers: { Accept: "application/vnd.github+json" },
				});

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

				try {
					const allReleases = await fetchAllPublishedReleases(ac.signal);
					if (ac.signal.aborted) {
						return;
					}
					setState({ status: "ok", latest, allReleases });
				} catch (e) {
					if (ac.signal.aborted) {
						return;
					}
					setState({
						status: "ok",
						latest,
						allReleases: null,
						historyError: (e as Error).message || "fetch failed",
					});
				}
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
