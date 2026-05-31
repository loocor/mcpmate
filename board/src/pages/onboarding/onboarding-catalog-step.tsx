import type { ReactNode } from "react";
import { useCallback, useEffect, useState } from "react";
import { Segment, type SegmentOption } from "../../components/ui/segment";
import type { CatalogTagFilter } from "../../lib/admin-discovery";
import {
	OnboardingCatalogSpinner,
	OnboardingStepHeader,
} from "./onboarding-setup-ui";

export function useLogoLoadFailures() {
	const [failedIds, setFailedIds] = useState<Set<string>>(() => new Set());
	const markFailed = useCallback((id: string) => {
		setFailedIds((prev) => {
			if (prev.has(id)) return prev;
			const next = new Set(prev);
			next.add(id);
			return next;
		});
	}, []);
	const hasFailed = useCallback((id: string) => failedIds.has(id), [failedIds]);
	return { markFailed, hasFailed };
}

export function useOnboardingDualTab(
	primaryTab: string,
	popularTab: string,
	primaryEmpty: boolean,
) {
	const [activeTab, setActiveTab] = useState(primaryTab);
	const [primaryTagFilter, setPrimaryTagFilter] = useState<CatalogTagFilter>("all");
	const [popularTagFilter, setPopularTagFilter] = useState<CatalogTagFilter>("all");

	useEffect(() => {
		if (primaryEmpty && activeTab === primaryTab) {
			setActiveTab(popularTab);
		}
	}, [activeTab, primaryEmpty, primaryTab, popularTab]);

	return {
		activeTab,
		setActiveTab,
		primaryTagFilter,
		setPrimaryTagFilter,
		popularTagFilter,
		setPopularTagFilter,
		isPrimaryTab: activeTab === primaryTab,
	};
}

export function buildOnboardingTabOptions(
	primary: { value: string; label: string; count: number },
	popular: { value: string; label: string; count: number },
): SegmentOption[] {
	return [
		{
			value: primary.value,
			label: primary.label,
			status: primary.count > 0 ? String(primary.count) : undefined,
		},
		{
			value: popular.value,
			label: popular.label,
			status: popular.count > 0 ? String(popular.count) : undefined,
		},
	];
}

export function catalogFilterEmptyMessage(
	catalogCount: number,
	loadErrorMessage: string,
	filteredEmptyMessage: string,
): string {
	return catalogCount === 0 ? loadErrorMessage : filteredEmptyMessage;
}

export function OnboardingDualTabStep({
	icon,
	title,
	description,
	tabOptions,
	activeTab,
	onTabChange,
	isPrimaryTab,
	primaryLoading,
	popularLoading,
	spinnerAccent = "emerald",
	primaryPanel,
	popularPanel,
}: {
	icon: ReactNode;
	title: string;
	description: string;
	tabOptions: SegmentOption[];
	activeTab: string;
	onTabChange: (value: string) => void;
	isPrimaryTab: boolean;
	primaryLoading: boolean;
	popularLoading: boolean;
	spinnerAccent?: "emerald" | "violet";
	primaryPanel: ReactNode;
	popularPanel: ReactNode;
}) {
	return (
		<div>
			<OnboardingStepHeader icon={icon} title={title} description={description} />
			<div className="mb-4">
				<Segment value={activeTab} onValueChange={onTabChange} options={tabOptions} />
			</div>
			{isPrimaryTab ? (
				primaryLoading ? (
					<OnboardingCatalogSpinner accent={spinnerAccent} />
				) : (
					primaryPanel
				)
			) : popularLoading ? (
				<OnboardingCatalogSpinner accent={spinnerAccent} />
			) : (
				popularPanel
			)}
		</div>
	);
}
