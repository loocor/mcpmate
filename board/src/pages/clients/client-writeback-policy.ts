import type {
  ClientConfigFileState,
  ClientMergeStrategy,
} from "../../lib/types";

export interface ClientWritebackMutationState {
  applyPending: boolean;
  attachmentPending: boolean;
}

export interface ClientWritebackBaseline {
  inheritedStrategy: ClientMergeStrategy;
  effectiveStrategy: ClientMergeStrategy;
}

export function resolveCreateClientWritebackBaseline({
  systemSettingsLoaded,
  systemOverride,
  templateStrategy,
}: {
  systemSettingsLoaded: boolean;
  systemOverride: ClientMergeStrategy | null;
  templateStrategy: ClientMergeStrategy | null;
}): ClientWritebackBaseline | null {
  if (!systemSettingsLoaded) return null;
  const inheritedStrategy = systemOverride ?? templateStrategy;
  if (!inheritedStrategy) return null;

  return {
    inheritedStrategy,
    effectiveStrategy: inheritedStrategy,
  };
}

export function resolveCreateClientTemplateStrategy({
  selectedTemplateStrategy,
  matchingTemplateStrategy,
}: {
  selectedTemplateStrategy: ClientMergeStrategy | null;
  matchingTemplateStrategy: ClientMergeStrategy | null;
}): ClientMergeStrategy | null {
  return selectedTemplateStrategy ?? matchingTemplateStrategy;
}

export interface ClientWritebackDecisionInput {
  mode: "create" | "edit";
  configFileChoice: ClientConfigFileState;
  selectedStrategy: ClientMergeStrategy;
  baseline: ClientWritebackBaseline | null;
  discoveryStrategy?: ClientMergeStrategy | null;
  supportedTransportsChanged: boolean;
  transportEditorsChanged: boolean;
}

export type ClientWritebackUpdate =
  | Record<string, never>
  | { clear_merge_strategy_override: true }
  | { merge_strategy_override: ClientMergeStrategy };

export interface ClientWritebackDecision {
  update: ClientWritebackUpdate;
  effectiveStrategyChanged: boolean;
  shouldReapplyClientConfig: boolean;
}

export function resolveClientWritebackDecision({
  mode,
  configFileChoice,
  selectedStrategy,
  baseline,
  discoveryStrategy,
  supportedTransportsChanged,
  transportEditorsChanged,
}: ClientWritebackDecisionInput): ClientWritebackDecision | null {
  if (configFileChoice === "without_config_file") {
    return {
      update: {},
      effectiveStrategyChanged: false,
      shouldReapplyClientConfig: false,
    };
  }

  if (!baseline) {
    return null;
  }

  let decision: Omit<ClientWritebackDecision, "shouldReapplyClientConfig">;
  if (mode === "create") {
    if (discoveryStrategy) {
      decision = {
        update: { merge_strategy_override: selectedStrategy },
        effectiveStrategyChanged: false,
      };
    } else {
      decision = {
        update:
          selectedStrategy === baseline.inheritedStrategy
            ? {}
            : { merge_strategy_override: selectedStrategy },
        effectiveStrategyChanged: false,
      };
    }
  } else if (selectedStrategy === baseline.effectiveStrategy) {
    decision = { update: {}, effectiveStrategyChanged: false };
  } else {
    decision = {
      update:
        selectedStrategy === baseline.inheritedStrategy
          ? { clear_merge_strategy_override: true }
          : { merge_strategy_override: selectedStrategy },
      effectiveStrategyChanged: true,
    };
  }

  return {
    ...decision,
    shouldReapplyClientConfig:
      mode === "edit" &&
      (supportedTransportsChanged ||
        transportEditorsChanged ||
        decision.effectiveStrategyChanged),
  };
}

export function hasPendingClientWritebackMutation({
  applyPending,
  attachmentPending,
}: ClientWritebackMutationState): boolean {
  return applyPending || attachmentPending;
}
