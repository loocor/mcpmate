import type { ClientMergeStrategy } from "../../lib/types";

export interface ClientWritebackMutationState {
  applyPending: boolean;
  attachmentPending: boolean;
}

export interface ClientWritebackBaseline {
  inheritedStrategy: ClientMergeStrategy;
  effectiveStrategy: ClientMergeStrategy;
}

export interface ClientWritebackDecisionInput {
  mode: "create" | "edit";
  selectedStrategy: ClientMergeStrategy;
  baseline: ClientWritebackBaseline;
  discoveryStrategy?: ClientMergeStrategy | null;
}

export type ClientWritebackUpdate =
  | Record<string, never>
  | { clear_merge_strategy_override: true }
  | { merge_strategy_override: ClientMergeStrategy };

export interface ClientWritebackDecision {
  update: ClientWritebackUpdate;
  effectiveStrategyChanged: boolean;
}

export function resolveClientWritebackDecision({
  mode,
  selectedStrategy,
  baseline,
  discoveryStrategy,
}: ClientWritebackDecisionInput): ClientWritebackDecision {
  if (mode === "create") {
    if (discoveryStrategy) {
      return {
        update: { merge_strategy_override: selectedStrategy },
        effectiveStrategyChanged: false,
      };
    }

    return {
      update:
        selectedStrategy === baseline.inheritedStrategy
          ? {}
          : { merge_strategy_override: selectedStrategy },
      effectiveStrategyChanged: false,
    };
  }

  if (selectedStrategy === baseline.effectiveStrategy) {
    return { update: {}, effectiveStrategyChanged: false };
  }

  return {
    update:
      selectedStrategy === baseline.inheritedStrategy
        ? { clear_merge_strategy_override: true }
        : { merge_strategy_override: selectedStrategy },
    effectiveStrategyChanged: true,
  };
}

export function hasPendingClientWritebackMutation({
  applyPending,
  attachmentPending,
}: ClientWritebackMutationState): boolean {
  return applyPending || attachmentPending;
}
