import { describe, expect, test } from "bun:test";
import { QueryClient } from "@tanstack/react-query";

import {
  buildClientWritebackDefaultUpdate,
  removeClientWritebackDecisionCache,
  resolveClientWritebackDefaultSelection,
} from "./client-writeback-default";

describe("client writeback default", () => {
  test("uses client recommendations when no system override exists", () => {
    expect(resolveClientWritebackDefaultSelection(null)).toBe("default");
    expect(resolveClientWritebackDefaultSelection(undefined)).toBe("default");
  });

  test("shows an explicit system override", () => {
    expect(resolveClientWritebackDefaultSelection("deep_merge")).toBe("deep_merge");
    expect(resolveClientWritebackDefaultSelection("replace")).toBe("replace");
  });

  test("clears the system override when recommendations are selected", () => {
    expect(buildClientWritebackDefaultUpdate("default")).toEqual({
      clear_default_merge_strategy_override: true,
    });
  });

  test("writes an explicit system override", () => {
    expect(buildClientWritebackDefaultUpdate("deep_merge")).toEqual({
      default_merge_strategy_override: "deep_merge",
    });
    expect(buildClientWritebackDefaultUpdate("replace")).toEqual({
      default_merge_strategy_override: "replace",
    });
  });

  test("removes cached client writeback decision inputs after the default changes", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["client-config", "cursor"], {
      effective_merge_strategy: "deep_merge",
    });
    queryClient.setQueryData(["systemSettings"], {
      default_merge_strategy_override: "deep_merge",
    });
    queryClient.setQueryData(["clients"], [{ identifier: "cursor" }]);

    removeClientWritebackDecisionCache(queryClient);

    expect(queryClient.getQueryData(["client-config", "cursor"])).toBeUndefined();
    expect(queryClient.getQueryData(["systemSettings"])).toBeUndefined();
    expect(queryClient.getQueryData(["clients"])).toEqual([{ identifier: "cursor" }]);
  });
});
