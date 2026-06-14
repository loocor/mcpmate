import { OAUTH_SECRET_KINDS } from "./secret-origin-hints";
import type { SecretMetadata } from "./types";

export type SecretLifecycleState =
  | "active"
  | "unused"
  | "oauth_managed";

export type SecretLifecycleFilter = SecretLifecycleState | "all";

export interface SecretLifecycle {
  state: SecretLifecycleState;
  activeCount: number;
  historicalCount: number;
}

const OAUTH_SECRET_KIND_SET = new Set<string>(OAUTH_SECRET_KINDS);

export function isOAuthManagedSecret(
  secret: Pick<SecretMetadata, "kind">,
): boolean {
  return OAUTH_SECRET_KIND_SET.has(secret.kind);
}

export function classifySecretLifecycle(
  secret: SecretMetadata,
): SecretLifecycle {
  const activeCount = secret.used_by_count;
  const historicalCount = secret.historical_usage_count;

  if (activeCount > 0) {
    return {
      state: "active",
      activeCount,
      historicalCount,
    };
  }

  if (isOAuthManagedSecret(secret)) {
    return {
      state: "oauth_managed",
      activeCount,
      historicalCount,
    };
  }

  return {
    state: "unused",
    activeCount,
    historicalCount,
  };
}

export function secretIsUnused(
  secretOrLifecycle: SecretMetadata | SecretLifecycle,
): boolean {
  const lifecycle =
    "state" in secretOrLifecycle
      ? secretOrLifecycle
      : classifySecretLifecycle(secretOrLifecycle);
  return lifecycle.state === "unused";
}

export function filterSecretsByLifecycle(
  secrets: SecretMetadata[],
  filter: SecretLifecycleFilter,
): SecretMetadata[] {
  if (filter === "all") return secrets;
  return secrets.filter((secret) => secretMatchesLifecycleFilter(secret, filter));
}

export function secretMatchesLifecycleFilter(
  secret: SecretMetadata,
  filter: SecretLifecycleFilter,
): boolean {
  if (filter === "all") return true;
  return classifySecretLifecycle(secret).state === filter;
}
