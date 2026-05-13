import { useQuery } from "@tanstack/react-query";
import { configSuitsApi } from "./api";
import { useAppStore } from "./store";
import type { ConfigSuit } from "./types";

/**
 * Stable role identifier for the system-seeded "default anchor" profile.
 *
 * The backend (see `backend/src/config/database.rs::init_default_profile`)
 * guarantees that exactly one profile carries this role, with
 * `is_default = true`, `is_active = true`, and `multi_select = true`. The
 * role string is the canonical handle — the display name is mutable.
 */
export const DEFAULT_ANCHOR_ROLE = "default_anchor";

/** Return the active default-anchor profile from a suits list, or `null`. */
export function findActiveDefaultProfile(
	suits: readonly ConfigSuit[],
): ConfigSuit | null {
	return (
		suits.find(
			(suit) =>
				(suit.role ?? "user") === DEFAULT_ANCHOR_ROLE && suit.is_active,
		) ?? null
	);
}

/**
 * Imperative variant: fetch the active default-anchor profile id from the
 * backend. Returns `null` when the anchor is missing or inactive.
 *
 * `configSuitsApi.getAll()` already swallows network errors and returns
 * `{ suits: [] }`, so this helper never throws.
 */
export async function resolveActiveDefaultProfileId(): Promise<string | null> {
	const { suits } = await configSuitsApi.getAll();
	return findActiveDefaultProfile(suits)?.id ?? null;
}

/**
 * High-level helper for the "Auto Add To Default Profile" setting.
 *
 * Returns the active default-anchor profile id when auto-add is enabled,
 * `null` when the setting is off or no active anchor is available. Callers
 * forward the result as `target_profile_id` to `POST /api/mcp/servers/import`
 * (or `profile_ids` on `PUT /api/mcp/servers/:id`), so that server creation
 * and profile linking happen atomically in the same backend transaction.
 *
 * The default anchor is seeded during backend init, so the only normal
 * reasons for `null` here are (a) auto-add is off, or (b) the user has
 * actively deactivated the anchor. We never invent a fallback profile.
 */
export async function resolveAutoAddTargetProfileId(opts: {
	autoAddEnabled: boolean;
}): Promise<string | null> {
	if (!opts.autoAddEnabled) {
		return null;
	}
	try {
		return await resolveActiveDefaultProfileId();
	} catch (error) {
		console.warn(
			"Failed to resolve default profile for auto-add; skipping link",
			error,
		);
		return null;
	}
}

/**
 * React hook variant of {@link resolveAutoAddTargetProfileId} for rendering
 * UI labels (e.g., the install wizard's result step). Returns the current
 * setting state plus the resolved profile id/name so each call site can
 * focus on its own UI differences while sharing the resolution logic.
 *
 * Uses the canonical `["configSuits"]` query key, so it benefits from cache
 * sharing and invalidations issued by other profile-aware screens.
 */
export function useAutoAddTargetProfile(): {
	enabled: boolean;
	profileId: string | null;
	profileName: string | null;
	isLoading: boolean;
} {
	const enabled = useAppStore(
		(state) => state.dashboardSettings.autoAddServerToDefaultProfile,
	);
	const { data, isLoading } = useQuery({
		queryKey: ["configSuits"],
		queryFn: () => configSuitsApi.getAll(),
		enabled,
		staleTime: 60_000,
	});
	const profile = enabled
		? findActiveDefaultProfile(data?.suits ?? [])
		: null;
	return {
		enabled,
		profileId: profile?.id ?? null,
		profileName: profile?.name ?? null,
		isLoading: enabled && isLoading,
	};
}
