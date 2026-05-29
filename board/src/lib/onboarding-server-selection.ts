import type { AdminDiscoveryServerCandidate } from "./admin-discovery";
import type { OnboardingServerCandidate } from "./onboarding-api";

export type OnboardingServerCandidateWithImport =
	OnboardingServerCandidate | AdminDiscoveryServerCandidate;

export function groupSelectedDiscoveryServerConfigs(
	candidates: OnboardingServerCandidateWithImport[],
	selectedKeys: Set<string>,
): Record<string, unknown> {
	const mcpServers = Object.create(null) as Record<string, unknown>;
	const selectedNames = new Set<string>();
	for (const candidate of candidates) {
		if (!selectedKeys.has(candidate.key) || !("import_config" in candidate)) continue;
		if (selectedNames.has(candidate.name)) {
			throw new Error(`Duplicate preset server name: ${candidate.name}`);
		}
		selectedNames.add(candidate.name);
		mcpServers[candidate.name] = candidate.import_config;
	}
	return mcpServers;
}
