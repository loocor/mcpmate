import { describe, expect, it } from "vitest";
import type { SecretMetadata } from "./types";
import {
  classifySecretLifecycle,
  filterSecretsByLifecycle,
  secretHasCleanupAvailable,
} from "./secret-lifecycle";

function secret(
  alias: string,
  overrides: Partial<SecretMetadata> = {},
): SecretMetadata {
  return {
    alias,
    placeholder: `[[secret:${alias}]]`,
    kind: "token",
    label: null,
    origin: null,
    provider_id: "local",
    provider_kind: "local",
    version: 1,
    used_by_count: 0,
    historical_usage_count: 0,
    created_at: null,
    updated_at: null,
    ...overrides,
  };
}

describe("classifySecretLifecycle", () => {
  it("marks active secrets before cleanup states", () => {
    expect(
      classifySecretLifecycle(
        secret("active-token", {
          used_by_count: 2,
          historical_usage_count: 3,
        }),
      ).state,
    ).toBe("active");
  });

  it("marks historical-only secrets as cleanup available", () => {
    const lifecycle = classifySecretLifecycle(
      secret("old-token", { historical_usage_count: 2 }),
    );

    expect(lifecycle.state).toBe("cleanup_available");
    expect(secretHasCleanupAvailable(lifecycle)).toBe(true);
  });

  it("marks unreferenced user-created secrets as unused", () => {
    expect(classifySecretLifecycle(secret("unused-token")).state).toBe(
      "unused",
    );
  });

  it("marks OAuth secrets as managed even without active usage", () => {
    expect(
      classifySecretLifecycle(
        secret("oauth/server/access-token", {
          kind: "oauth_access_token",
          historical_usage_count: 1,
        }),
      ).state,
    ).toBe("oauth_managed");
  });
});

describe("filterSecretsByLifecycle", () => {
  const secrets = [
    secret("active", { used_by_count: 1 }),
    secret("cleanup", { historical_usage_count: 1 }),
    secret("unused"),
    secret("oauth", { kind: "oauth_refresh_token" }),
  ];

  it("filters cleanup-available secrets", () => {
    expect(
      filterSecretsByLifecycle(secrets, "cleanup_available").map(
        (item) => item.alias,
      ),
    ).toEqual(["cleanup"]);
  });

  it("keeps all secrets when filter is all", () => {
    expect(filterSecretsByLifecycle(secrets, "all")).toHaveLength(4);
  });
});
