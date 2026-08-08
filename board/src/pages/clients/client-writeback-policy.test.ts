import { describe, expect, test } from "bun:test";

import {
  hasPendingClientWritebackMutation,
  resolveCreateClientTemplateStrategy,
  resolveCreateClientWritebackBaseline,
  resolveClientWritebackDecision,
} from "./client-writeback-policy";

describe("client writeback policy", () => {
  test("prefers the selected preset strategy over a matching draft identifier", () => {
    expect(
      resolveCreateClientTemplateStrategy({
        selectedTemplateStrategy: "deep_merge",
        matchingTemplateStrategy: "replace",
      }),
    ).toBe("deep_merge");
  });

  test("resolves create inheritance without inventing a template strategy", () => {
    expect(
      resolveCreateClientWritebackBaseline({
        systemSettingsLoaded: true,
        systemOverride: "replace",
        templateStrategy: "deep_merge",
      }),
    ).toEqual({
      inheritedStrategy: "replace",
      effectiveStrategy: "replace",
    });
    expect(
      resolveCreateClientWritebackBaseline({
        systemSettingsLoaded: true,
        systemOverride: null,
        templateStrategy: "deep_merge",
      }),
    ).toEqual({
      inheritedStrategy: "deep_merge",
      effectiveStrategy: "deep_merge",
    });
    expect(
      resolveCreateClientWritebackBaseline({
        systemSettingsLoaded: true,
        systemOverride: null,
        templateStrategy: null,
      }),
    ).toBeNull();
    expect(
      resolveCreateClientWritebackBaseline({
        systemSettingsLoaded: false,
        systemOverride: null,
        templateStrategy: "deep_merge",
      }),
    ).toBeNull();
  });

  test("blocks overlapping writeback mutations", () => {
    expect(
      hasPendingClientWritebackMutation({
        applyPending: false,
        attachmentPending: false,
      }),
    ).toBe(false);

    for (const pending of ["applyPending", "attachmentPending"] as const) {
      expect(
        hasPendingClientWritebackMutation({
          applyPending: pending === "applyPending",
          attachmentPending: pending === "attachmentPending",
        }),
      ).toBe(true);
    }
  });

  test("persists an Admin discovery strategy for a new client", () => {
    expect(
      resolveClientWritebackDecision({
        mode: "create",
        configFileChoice: "with_config_file",
        selectedStrategy: "deep_merge",
        discoveryStrategy: "deep_merge",
        supportedTransportsChanged: false,
        transportEditorsChanged: false,
        baseline: {
          inheritedStrategy: "replace",
          effectiveStrategy: "replace",
        },
      }),
    ).toEqual({
      update: { merge_strategy_override: "deep_merge" },
      effectiveStrategyChanged: false,
      shouldReapplyClientConfig: false,
    });
  });

  for (const mode of ["create", "edit"] as const) {
    test(`skips writeback and re-apply in ${mode} mode without a config file`, () => {
      expect(
        resolveClientWritebackDecision({
          mode,
          configFileChoice: "without_config_file",
          selectedStrategy: "deep_merge",
          baseline: null,
          discoveryStrategy: mode === "create" ? "deep_merge" : null,
          supportedTransportsChanged: true,
          transportEditorsChanged: true,
        }),
      ).toEqual({
        update: {},
        effectiveStrategyChanged: false,
        shouldReapplyClientConfig: false,
      });
    });
  }

  test("requires a writeback baseline for a client with a config file", () => {
    expect(
      resolveClientWritebackDecision({
        mode: "create",
        configFileChoice: "with_config_file",
        selectedStrategy: "deep_merge",
        baseline: null,
        discoveryStrategy: "deep_merge",
        supportedTransportsChanged: false,
        transportEditorsChanged: false,
      }),
    ).toBeNull();
  });

  test("preserves an unchanged client override", () => {
    expect(
      resolveClientWritebackDecision({
        mode: "edit",
        configFileChoice: "with_config_file",
        selectedStrategy: "deep_merge",
        supportedTransportsChanged: false,
        transportEditorsChanged: false,
        baseline: {
          inheritedStrategy: "replace",
          effectiveStrategy: "deep_merge",
        },
      }),
    ).toEqual({
      update: {},
      effectiveStrategyChanged: false,
      shouldReapplyClientConfig: false,
    });
  });

  test("re-applies a config file when supported transports change", () => {
    expect(
      resolveClientWritebackDecision({
        mode: "edit",
        configFileChoice: "with_config_file",
        selectedStrategy: "deep_merge",
        supportedTransportsChanged: true,
        transportEditorsChanged: false,
        baseline: {
          inheritedStrategy: "replace",
          effectiveStrategy: "deep_merge",
        },
      }),
    ).toEqual({
      update: {},
      effectiveStrategyChanged: false,
      shouldReapplyClientConfig: true,
    });
  });

  test("re-applies a config file when transport write rules change", () => {
    expect(
      resolveClientWritebackDecision({
        mode: "edit",
        configFileChoice: "with_config_file",
        selectedStrategy: "deep_merge",
        supportedTransportsChanged: false,
        transportEditorsChanged: true,
        baseline: {
          inheritedStrategy: "replace",
          effectiveStrategy: "deep_merge",
        },
      }),
    ).toEqual({
      update: {},
      effectiveStrategyChanged: false,
      shouldReapplyClientConfig: true,
    });
  });

  test("does not re-apply transport write rules while creating a client", () => {
    expect(
      resolveClientWritebackDecision({
        mode: "create",
        configFileChoice: "with_config_file",
        selectedStrategy: "replace",
        discoveryStrategy: null,
        supportedTransportsChanged: false,
        transportEditorsChanged: true,
        baseline: {
          inheritedStrategy: "replace",
          effectiveStrategy: "deep_merge",
        },
      }),
    ).toEqual({
      update: {},
      effectiveStrategyChanged: false,
      shouldReapplyClientConfig: false,
    });
  });

  test("clears an override when the selection returns to the inherited strategy", () => {
    expect(
      resolveClientWritebackDecision({
        mode: "edit",
        configFileChoice: "with_config_file",
        selectedStrategy: "replace",
        supportedTransportsChanged: false,
        transportEditorsChanged: false,
        baseline: {
          inheritedStrategy: "replace",
          effectiveStrategy: "deep_merge",
        },
      }),
    ).toEqual({
      update: { clear_merge_strategy_override: true },
      effectiveStrategyChanged: true,
      shouldReapplyClientConfig: true,
    });
  });

  test("writes an explicit override when the selection differs from inheritance", () => {
    expect(
      resolveClientWritebackDecision({
        mode: "edit",
        configFileChoice: "with_config_file",
        selectedStrategy: "deep_merge",
        supportedTransportsChanged: false,
        transportEditorsChanged: false,
        baseline: {
          inheritedStrategy: "replace",
          effectiveStrategy: "replace",
        },
      }),
    ).toEqual({
      update: { merge_strategy_override: "deep_merge" },
      effectiveStrategyChanged: true,
      shouldReapplyClientConfig: true,
    });
  });
});
