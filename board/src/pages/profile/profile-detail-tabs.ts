export function resolveProfileDetailTab(
	activeTab: string,
	profileMode: string | undefined,
): string {
	if (profileMode === undefined) {
		return activeTab;
	}

	if (profileMode === "workflow" && activeTab === "capabilities") {
		return "workflow";
	}

	if (
		profileMode !== "workflow" &&
		(activeTab === "workflow" || activeTab === "materials")
	) {
		return "overview";
	}

	return activeTab;
}

export function resolveProfileDetailReviewTab(
	activeTab: string,
	hasReviewItem: boolean,
	hasExplicitTab: boolean,
): string {
	if (hasReviewItem && !hasExplicitTab) {
		return "capabilities";
	}

	return activeTab;
}
