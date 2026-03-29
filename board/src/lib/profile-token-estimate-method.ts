/**
 * User-selected tokenizer for profile capability payload estimates (dashboard).
 * Ledger rows are UTF-8 JSON; counting method affects totals shown in the UI.
 */
export type ProfileTokenEstimateMethod = "openai_cl100k" | "anthropic_claude";

export const PROFILE_TOKEN_ESTIMATE_METHOD_DEFAULT: ProfileTokenEstimateMethod =
	"openai_cl100k";

export function isProfileTokenEstimateMethod(
	value: unknown,
): value is ProfileTokenEstimateMethod {
	return value === "openai_cl100k" || value === "anthropic_claude";
}
