import type { QueryClient } from "@tanstack/react-query";

export function invalidateServerCatalogAfterImport(
	queryClient: QueryClient,
	importedCount: number,
): Promise<void> {
	if (importedCount <= 0) {
		return Promise.resolve();
	}

	return queryClient.invalidateQueries({ queryKey: ["servers"] });
}
