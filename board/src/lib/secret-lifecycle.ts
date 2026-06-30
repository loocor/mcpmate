import { OAUTH_SECRET_KINDS } from "./secret-origin-hints";
import type { SecretMetadata } from "./types";

export type SecretLifecycleState =
  | "active"
  | "unknown"
  | "unused"
  | "oauth_managed";

export type SecretLifecycleFilter = SecretLifecycleState | "all";

export interface SecretLifecycle {
  state: SecretLifecycleState;
  activeCount: number;
  historicalCount: number;
  unknownCount: number;
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
  const unknownCount = secret.unknown_usage_count ?? 0;

  if (activeCount > 0) {
    return {
      state: "active",
      activeCount,
      historicalCount,
      unknownCount,
    };
  }

  if (unknownCount > 0) {
    return {
      state: "unknown",
      activeCount,
      historicalCount,
      unknownCount,
    };
  }

  if (isOAuthManagedSecret(secret)) {
    return {
      state: "oauth_managed",
      activeCount,
      historicalCount,
      unknownCount,
    };
  }

  return {
    state: "unused",
    activeCount,
    historicalCount,
    unknownCount,
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
