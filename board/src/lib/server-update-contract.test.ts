import { describe, expect, test } from "bun:test";
import type { MCPServerConfig } from "./types";
import { assertServerCrudUpdate } from "./server-update-contract";

describe("server CRUD update contract", () => {
  test("accepts server configuration fields", () => {
    const update: Partial<MCPServerConfig> = {
      kind: "streamable_http",
      url: "https://example.com/mcp",
      pending_import: false,
    };

    expect(() => assertServerCrudUpdate(update)).not.toThrow();
  });

  test("rejects global status changes", () => {
    expect(() => assertServerCrudUpdate({ enabled: true })).toThrow(
      "Server enabled state must be changed through the server management API.",
    );
  });

  test("rejects profile relationship changes", () => {
    expect(() =>
      assertServerCrudUpdate({ profile_ids: ["profile-a"] }),
    ).toThrow(
      "Server profile relationships must be changed through the profile management API.",
    );
  });

  test("rejects direct exposure eligibility changes", () => {
    expect(() =>
      assertServerCrudUpdate({ unify_direct_exposure_eligible: true }),
    ).toThrow(
      "Server direct exposure eligibility must be changed through the server management API.",
    );
  });
});
