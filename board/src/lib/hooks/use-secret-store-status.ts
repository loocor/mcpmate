import {
	useQuery,
	type QueryClient,
	type UseQueryOptions,
} from "@tanstack/react-query";
import { secretsApi } from "../api";
import type { SecretStoreStatusData } from "../types";

export const SECRET_STORE_STATUS_QUERY_KEY = ["secrets", "status"] as const;
export const SECRET_STORE_CATALOG_QUERY_KEY = ["secrets"] as const;
export const SECRET_STORE_USAGES_QUERY_KEY = ["secrets", "usages"] as const;

const DEFAULT_STALE_TIME_MS = 30_000;

type SecretStoreStatusQueryOptions = Pick<
	UseQueryOptions<SecretStoreStatusData>,
	"enabled" | "staleTime" | "retry" | "refetchOnWindowFocus"
>;

export function useSecretStoreStatusQuery(
	options: SecretStoreStatusQueryOptions = {},
) {
	return useQuery({
		queryKey: SECRET_STORE_STATUS_QUERY_KEY,
		queryFn: secretsApi.status,
		staleTime: DEFAULT_STALE_TIME_MS,
		...options,
	});
}

export async function invalidateSecretStoreStatus(
	queryClient: QueryClient,
): Promise<void> {
	await queryClient.invalidateQueries({ queryKey: SECRET_STORE_STATUS_QUERY_KEY });
}

export async function invalidateSecretStoreCatalog(
	queryClient: QueryClient,
): Promise<void> {
	await Promise.all([
		queryClient.invalidateQueries({
			queryKey: SECRET_STORE_CATALOG_QUERY_KEY,
			exact: true,
		}),
		queryClient.invalidateQueries({
			queryKey: SECRET_STORE_USAGES_QUERY_KEY,
		}),
	]);
}

export async function invalidateSecretStoreData(
	queryClient: QueryClient,
	options: { catalog?: boolean } = {},
): Promise<void> {
	await invalidateSecretStoreStatus(queryClient);
	if (options.catalog) {
		await invalidateSecretStoreCatalog(queryClient);
	}
}
