/**
 * Format token count for display.
 * - Under 1000: show as-is (e.g., "500")
 * - 1000-999999: show as K (e.g., "12.5K")
 * - 1000000+: show as M (e.g., "1.2M")
 */
export function formatTokenCount(tokens: number): string {
	if (tokens < 1000) {
		return tokens.toString();
	}
	if (tokens < 1000000) {
		const k = tokens / 1000;
		return k >= 10 ? `${Math.round(k)}K` : `${k.toFixed(1)}K`;
	}
	const m = tokens / 1000000;
	return m >= 10 ? `${Math.round(m)}M` : `${m.toFixed(1)}M`;
}
