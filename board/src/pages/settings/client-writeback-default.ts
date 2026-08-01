import type { QueryClient } from "@tanstack/react-query";

import type { ClientMergeStrategy } from "../../lib/types";

export type ClientWritebackDefaultSelection = "default" | ClientMergeStrategy;

export type ClientWritebackDefaultUpdate =
  | { clear_default_merge_strategy_override: true }
  | { default_merge_strategy_override: ClientMergeStrategy };

export function resolveClientWritebackDefaultSelection(
  override: ClientMergeStrategy | null | undefined,
): ClientWritebackDefaultSelection {
  return override ?? "default";
}

export function buildClientWritebackDefaultUpdate(
  selection: ClientWritebackDefaultSelection,
): ClientWritebackDefaultUpdate {
  if (selection === "default") {
    return { clear_default_merge_strategy_override: true };
  }

  return { default_merge_strategy_override: selection };
}

export function removeClientWritebackDecisionCache(queryClient: QueryClient): void {
  queryClient.removeQueries({ queryKey: ["client-config"] });
  queryClient.removeQueries({ queryKey: ["systemSettings"] });
}
