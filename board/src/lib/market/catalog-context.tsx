import { createContext, useContext, useMemo, type ReactNode } from "react";
import type { MarketCatalogProvider } from "./types";
import { McpRegistryProvider } from "./providers/mcp-registry-provider";

const PROVIDER_REGISTRY: Record<string, () => MarketCatalogProvider> = {
  "mcp-registry": () => new McpRegistryProvider(),
};

const DEFAULT_PROVIDER_ID = "mcp-registry";

function createProvider(id: string): MarketCatalogProvider | null {
  const factory = PROVIDER_REGISTRY[id];
  return factory ? factory() : null;
}

interface CatalogContextValue {
  provider: MarketCatalogProvider;
  providerId: string;
}

const CatalogContext = createContext<CatalogContextValue | null>(null);

export function CatalogProvider({
  providerId,
  children,
}: {
  providerId?: string;
  children: ReactNode;
}) {
  const value = useMemo(() => {
    const resolvedId = providerId ?? DEFAULT_PROVIDER_ID;
    const provider = createProvider(resolvedId);
    if (!provider) {
      throw new Error(`Unknown catalog provider: ${resolvedId}`);
    }
    return { provider, providerId: resolvedId };
  }, [providerId]);

  return (
    <CatalogContext.Provider value={value}>{children}</CatalogContext.Provider>
  );
}

export function useCatalogProvider(): CatalogContextValue {
  const ctx = useContext(CatalogContext);
  if (!ctx) {
    throw new Error(
      "useCatalogProvider must be used within a <CatalogProvider>",
    );
  }
  return ctx;
}
