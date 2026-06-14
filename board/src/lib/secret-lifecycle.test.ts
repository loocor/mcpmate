import { describe, expect, it } from "vitest";
import type { SecretMetadata } from "./types";
import {
  classifySecretLifecycle,
  filterSecretsByLifecycle,
  secretIsUnused,
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
  it("marks active secrets before inactive ownership states", () => {
    expect(
      classifySecretLifecycle(
        secret("active-token", {
          used_by_count: 2,
          historical_usage_count: 3,
        }),
      ).state,
    ).toBe("active");
  });

  it("marks historical-only secrets as unused", () => {
    const lifecycle = classifySecretLifecycle(
      secret("old-token", { historical_usage_count: 2 }),
    );

    expect(lifecycle.state).toBe("unused");
    expect(secretIsUnused(lifecycle)).toBe(true);
  });

  it("marks unreferenced user-created secrets as unused", () => {
    expect(classifySecretLifecycle(secret("unused-token")).state).toBe(
      "unused",
    );
  });

  it("marks inactive OAuth secrets as managed", () => {
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
		secret("active-oauth", {
			kind: "oauth_access_token",
			used_by_count: 1,
		}),
	];

	it("filters unused secrets by exclusive lifecycle classification", () => {
		expect(
			filterSecretsByLifecycle(secrets, "unused").map((item) => item.alias),
		).toEqual(["cleanup", "unused"]);
	});

	it("keeps all secrets when filter is all", () => {
		expect(filterSecretsByLifecycle(secrets, "all")).toHaveLength(5);
	});

	it("includes active OAuth managed secrets in active filter", () => {
		expect(
			filterSecretsByLifecycle(secrets, "active").map((item) => item.alias),
		).toEqual(["active", "active-oauth"]);
	});

	it("keeps OAuth managed filter exclusive to inactive OAuth ownership", () => {
		expect(
			filterSecretsByLifecycle(secrets, "oauth_managed").map(
				(item) => item.alias,
			),
		).toEqual(["oauth"]);
	});
});
