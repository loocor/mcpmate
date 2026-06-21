/**
 * Format token count for display.
 * - Under 1000: show as-is (e.g., "500")
 * - 1000-999999: show as K or rounded M near the upper boundary (e.g., "12.5K", "1.0M")
 * - 1000000+: show as M (e.g., "1.2M")
 */
function formatMillionTokenCount(tokens: number): string {
	const m = tokens / 1000000;
	return m >= 10 ? `${Math.round(m)}M` : `${m.toFixed(1)}M`;
}

function formatThousandTokenCount(tokens: number): string {
	const k = tokens / 1000;
	if (k < 10) {
		return `${k.toFixed(1)}K`;
	}

	const roundedK = Math.round(k);
	return roundedK >= 1000 ? formatMillionTokenCount(tokens) : `${roundedK}K`;
}

export function formatTokenCount(tokens: number): string {
	if (tokens < 1000) {
		return tokens.toString();
	}
	if (tokens < 1000000) {
		return formatThousandTokenCount(tokens);
	}
	return formatMillionTokenCount(tokens);
}
