import type { ServerSource } from "../types";
import { McpRegistryProvider } from "./providers/mcp-registry-provider";
import type { CatalogEntry } from "./types";

const registryProvider = new McpRegistryProvider();

export async function fetchCatalogEntryForSource(
	source?: ServerSource | null,
): Promise<CatalogEntry | null> {
	if (source?.type !== "registry" || !source.ref) return null;
	return registryProvider.fetchByKey(source.ref);
}
