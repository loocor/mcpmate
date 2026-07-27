import type { MCPServerConfig } from "./types";

export function assertServerCrudUpdate(
  serverConfig: Partial<MCPServerConfig>,
): void {
  if (serverConfig.enabled !== undefined) {
    throw new Error(
      "Server enabled state must be changed through the server management API.",
    );
  }
  if (serverConfig.profile_ids !== undefined) {
    throw new Error(
      "Server profile relationships must be changed through the profile management API.",
    );
  }
  if (serverConfig.unify_direct_exposure_eligible !== undefined) {
    throw new Error(
      "Server direct exposure eligibility must be changed through the server management API.",
    );
  }
}
