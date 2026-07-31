import type { QueryClient } from "@tanstack/react-query";

type RefreshCapabilities = (serverId: string) => Promise<unknown>;

export async function syncAuthenticatedServerCapabilities({
  serverId,
  queryClient,
  refreshCapabilities,
}: {
  serverId: string;
  queryClient: QueryClient;
  refreshCapabilities: RefreshCapabilities;
}): Promise<void> {
  try {
    await refreshCapabilities(serverId);
  } finally {
    await Promise.all([
      queryClient.invalidateQueries({
        queryKey: ["server", serverId],
        refetchType: "active",
      }),
      queryClient.invalidateQueries({
        queryKey: ["servers"],
        refetchType: "active",
      }),
      queryClient.invalidateQueries({
        queryKey: ["server-oauth", serverId],
        refetchType: "active",
      }),
      queryClient.invalidateQueries({
        queryKey: ["server-cap"],
        refetchType: "active",
      }),
    ]);
  }
}
