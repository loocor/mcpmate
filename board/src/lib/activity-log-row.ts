import type { ReactNode } from "react";

export type ActivityLogTableSize = "big" | "middle" | "small";

export type ActivityLogTableHeaders = {
	expandColumn: string;
	timestamp: string;
	action: string;
	category: string;
	status: string;
	target: string;
	duration: string;
};

export type ActivityLogRow = {
	key: string;
	eventId?: string;
	timestampMs: number;
	action: ReactNode;
	category: ReactNode;
	status: ReactNode;
	target: string;
	durationMs: number | null;
	details?: ReactNode;
	expandable?: boolean;
};
