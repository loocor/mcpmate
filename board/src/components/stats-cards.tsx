import type { ReactNode } from "react";
import { StatsCard } from "./page-layout";

export interface StatCardData {
	title: string;
	value: string | number;
	description: string;
	icon?: ReactNode;
	action?: ReactNode;
	tone?: "default" | "warning" | "destructive";
	tooltip?: string;
}

interface StatsCardsProps {
	cards: StatCardData[];
}

export function StatsCards({ cards }: StatsCardsProps) {
	return (
		<>
			{cards.map((card) => (
				<StatsCard
					key={card.title}
					title={card.title}
					value={card.value}
					description={card.description}
					icon={card.icon}
					action={card.action}
					tone={card.tone}
					tooltip={card.tooltip}
				/>
			))}
		</>
	);
}
