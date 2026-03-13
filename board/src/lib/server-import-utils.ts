import type { SkippedServer } from "./types";

export const formatNameList = (names: string[], limit = 3): string => {
	if (!names.length) return "";
	if (names.length <= limit) return names.join(", ");
	const head = names.slice(0, limit).join(", ");
	return `${head}, +${names.length - limit} more`;
};

const describeSkip = (detail: SkippedServer): string => {
	const label = detail.reason.replace(/_/g, " ");
	const parts: string[] = [label];
	if (detail.incoming_query || detail.existing_query) {
		const queryParts: string[] = [];
		if (detail.incoming_query) {
			queryParts.push(`incoming=${detail.incoming_query}`);
		}
		if (detail.existing_query) {
			queryParts.push(`existing=${detail.existing_query}`);
		}
		if (queryParts.length) {
			parts.push(queryParts.join(", "));
		}
	}
	return `${detail.name} (${parts.join("; ")})`;
};

export const summarizeSkipped = (details: SkippedServer[]): string => {
	if (!details.length) return "";
	return details.map(describeSkip).join("; ");
};
