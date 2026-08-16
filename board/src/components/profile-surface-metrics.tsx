import { cn } from "../lib/utils";
import { StatsCard, STATS_CARD_GRID_CLASS } from "./page-layout";

export type ProfileSurfaceMetric = {
	id: string;
	label: string;
	value: string;
	description?: string;
};

type ProfileSurfaceMetricsProps = {
	metrics: ProfileSurfaceMetric[];
	description: string;
	onSelect?: () => void;
	className?: string;
};

export function ProfileSurfaceMetrics({
	metrics,
	description,
	onSelect,
	className,
}: ProfileSurfaceMetricsProps) {
	return (
		<div className={cn(STATS_CARD_GRID_CLASS, className)}>
			{metrics.map((metric) => (
				<StatsCard
					key={metric.id}
					title={metric.label}
					value={metric.value}
					description={metric.description ?? description}
					className="h-full"
					onHeaderClick={onSelect}
				/>
			))}
		</div>
	);
}
