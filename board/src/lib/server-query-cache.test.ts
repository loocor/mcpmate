import { QueryClient } from "@tanstack/react-query";
import { describe, expect, test } from "bun:test";

import { invalidateServerCatalogAfterImport } from "./server-query-cache";

describe("invalidateServerCatalogAfterImport", () => {
	test("invalidates the server catalog after importing servers", async () => {
		const queryClient = new QueryClient();
		queryClient.setQueryData(["servers"], { servers: [] });
		queryClient.setQueryData(["clients"], { clients: [] });

		await invalidateServerCatalogAfterImport(queryClient, 2);

		expect(queryClient.getQueryState(["servers"])?.isInvalidated).toBe(true);
		expect(queryClient.getQueryState(["clients"])?.isInvalidated).toBe(false);
	});

	test("keeps the server catalog fresh when nothing was imported", async () => {
		const queryClient = new QueryClient();
		queryClient.setQueryData(["servers"], { servers: [] });

		await invalidateServerCatalogAfterImport(queryClient, 0);

		expect(queryClient.getQueryState(["servers"])?.isInvalidated).toBe(false);
	});
});
