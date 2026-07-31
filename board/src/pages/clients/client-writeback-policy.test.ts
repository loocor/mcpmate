import { describe, expect, test } from "bun:test";

import {
  hasPendingClientWritebackMutation,
  resolveClientWritebackDecision,
} from "./client-writeback-policy";

describe("client writeback policy", () => {
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
        selectedStrategy: "deep_merge",
        discoveryStrategy: "deep_merge",
        baseline: {
          inheritedStrategy: "replace",
          effectiveStrategy: "replace",
        },
      }),
    ).toEqual({
      update: { merge_strategy_override: "deep_merge" },
      effectiveStrategyChanged: false,
    });
  });

  test("preserves an unchanged client override", () => {
    expect(
      resolveClientWritebackDecision({
        mode: "edit",
        selectedStrategy: "deep_merge",
        baseline: {
          inheritedStrategy: "replace",
          effectiveStrategy: "deep_merge",
        },
      }),
    ).toEqual({ update: {}, effectiveStrategyChanged: false });
  });

  test("clears an override when the selection returns to the inherited strategy", () => {
    expect(
      resolveClientWritebackDecision({
        mode: "edit",
        selectedStrategy: "replace",
        baseline: {
          inheritedStrategy: "replace",
          effectiveStrategy: "deep_merge",
        },
      }),
    ).toEqual({
      update: { clear_merge_strategy_override: true },
      effectiveStrategyChanged: true,
    });
  });

  test("writes an explicit override when the selection differs from inheritance", () => {
    expect(
      resolveClientWritebackDecision({
        mode: "edit",
        selectedStrategy: "deep_merge",
        baseline: {
          inheritedStrategy: "replace",
          effectiveStrategy: "replace",
        },
      }),
    ).toEqual({
      update: { merge_strategy_override: "deep_merge" },
      effectiveStrategyChanged: true,
    });
  });
});
