import { useTranslation } from "react-i18next";
import { EntityCard } from "../../../components/entity-card";
import { Badge } from "../../../components/ui/badge";
import { Switch } from "../../../components/ui/switch";
import { useProfileTokenChartSource } from "../../../lib/api";
import { useAppStore } from "../../../lib/store";
import type { ConfigSuit } from "../../../lib/types";
import { ProfileTokenUsageChart } from "./profile-token-usage-chart";

export interface ProfileSuitGridCardProps {
	suit: ConfigSuit;
	statItems: Array<{ label: string; value: string | number }>;
	displayName: string;
	avatarInitial: string;
	isDefaultAnchor: boolean;
	isTogglePending: boolean;
	onNavigate: () => void;
	onToggle: () => void;
	enabledByComponentId: Map<string, boolean>;
	/** When loaded and zero, chart shows gray ring with "-" (omit while stats loading). */
	profileServerCount?: number;
}

export function ProfileSuitGridCard({
	suit,
	statItems,
	displayName,
	avatarInitial,
	isDefaultAnchor,
	isTogglePending,
	onNavigate,
	onToggle,
	enabledByComponentId,
	profileServerCount,
}: ProfileSuitGridCardProps) {
	const { t } = useTranslation();
	const profileTokenEstimateMethod = useAppStore(
		(state) => state.dashboardSettings.profileTokenEstimateMethod,
	);
	const tokenSource = useProfileTokenChartSource(suit.id, enabledByComponentId);

	return (
		<EntityCard
			id={suit.id}
			title={displayName}
			description={suit.description}
			avatar={{
				fallback: avatarInitial,
			}}
			topRightBadge={
				<ProfileTokenUsageChart
					layout="chartOnly"
					ledgerItems={tokenSource.ledgerItems}
					fallbackEstimate={tokenSource.fallbackEstimate}
					isLoading={tokenSource.isLoading}
					isError={tokenSource.isError}
					enabledByComponentId={enabledByComponentId}
					estimateMethod={profileTokenEstimateMethod}
					profileServerCount={profileServerCount}
					className="-mr-1"
				/>
			}
			stats={statItems}
			bottomLeft={
				<Badge variant={suit.is_active ? "default" : "secondary"}>
					{t(`profiles:suitTypes.${suit.suit_type}`, {
						defaultValue: suit.suit_type,
					})}
				</Badge>
			}
			bottomRight={
				<Switch
					checked={suit.is_active}
					onCheckedChange={onToggle}
					disabled={isTogglePending || isDefaultAnchor}
					onClick={(e) => e.stopPropagation()}
				/>
			}
			onClick={onNavigate}
		/>
	);
}
