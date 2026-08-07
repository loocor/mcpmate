import { ApiRequestError } from "./api";

const PROFILE_CAPABILITY_QUERY_KEYS = new Set([
	"configSuitTools",
	"configSuitResources",
	"configSuitPrompts",
	"configSuitResourceTemplates",
]);

export function shouldRefreshProfileCapabilityQuery(
	queryKey: readonly unknown[],
	data: unknown,
	profileId: string,
	dependencyServerIds: ReadonlySet<string>,
): boolean {
	if (
		typeof queryKey[0] !== "string" ||
		!PROFILE_CAPABILITY_QUERY_KEYS.has(queryKey[0]) ||
		queryKey[1] !== profileId ||
		!data ||
		typeof data !== "object"
	) {
		return false;
	}
	const revisions = (data as { source_revision_set?: unknown })
		.source_revision_set;
	if (!revisions || typeof revisions !== "object") {
		return false;
	}
	return Object.keys(revisions).some((serverId) =>
		dependencyServerIds.has(serverId),
	);
}

interface HandleProfileCapabilityMutationErrorInput {
	error: unknown;
	profileId: string;
	invalidateQueries: (
		predicate: (queryKey: readonly unknown[], data: unknown) => boolean,
	) => void | Promise<unknown>;
}

export async function handleProfileCapabilityMutationError({
	error,
	profileId,
	invalidateQueries,
}: HandleProfileCapabilityMutationErrorInput): Promise<boolean> {
	if (
		!(error instanceof ApiRequestError) ||
		error.code !== "catalog_dependency_changed"
	) {
		return false;
	}
	const dependencyServerIds = new Set(
		error.details?.dependencyServerIds ?? [],
	);
	await invalidateQueries((queryKey, data) =>
		shouldRefreshProfileCapabilityQuery(
			queryKey,
			data,
			profileId,
			dependencyServerIds,
		),
	);
	return true;
}
