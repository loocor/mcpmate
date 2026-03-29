import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { ProfileTokenUsageChart } from "../../profile/components/profile-token-usage-chart";
import { useProfileTokenChartSource } from "../../../lib/api";
import { useAppStore } from "../../../lib/store";
import { cn } from "../../../lib/utils";

export interface ConfigurationProfileTokenChartProps {
	profileId: string;
	enabledByComponentId: ReadonlyMap<string, boolean>;
	profileServerCount?: number;
	/** When true, link click calls `stopPropagation` first (e.g. selectable profile rows). */
	stopPropagationOnNavigate?: boolean;
	className?: string;
}

/**
 * Profile token donut (same as profile grid cards) with navigation to profile detail.
 */
export function ConfigurationProfileTokenChart({
	profileId,
	enabledByComponentId,
	profileServerCount,
	stopPropagationOnNavigate,
	className,
}: ConfigurationProfileTokenChartProps) {
	const { t } = useTranslation("clients");
	const profileTokenEstimateMethod = useAppStore(
		(state) => state.dashboardSettings.profileTokenEstimateMethod,
	);
	const tokenSource = useProfileTokenChartSource(profileId, enabledByComponentId);

	return (
		<Link
			to={`/profiles/${profileId}`}
			onClick={stopPropagationOnNavigate ? (e) => e.stopPropagation() : undefined}
			className={cn(
				"inline-flex max-w-full shrink-0 items-center rounded-md focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background",
				className,
			)}
			aria-label={t("detail.configuration.labels.openProfileDetail", {
				defaultValue: "Open profile details",
			})}
		>
			<ProfileTokenUsageChart
				layout="chartOnly"
				chartSizePx={56}
				ledgerItems={tokenSource.ledgerItems}
				fallbackEstimate={tokenSource.fallbackEstimate}
				isLoading={tokenSource.isLoading}
				isError={tokenSource.isError}
				enabledByComponentId={enabledByComponentId}
				estimateMethod={profileTokenEstimateMethod}
				profileServerCount={profileServerCount}
				className="-mr-1"
			/>
		</Link>
	);
}
