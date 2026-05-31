import { useQuery } from "@tanstack/react-query";
import React from "react";
import { systemApi } from "../lib/api";

export const METRICS_HISTORY_STORAGE_KEY = "mcp_metrics_history_v3";

export type MetricsHistoryPoint = {
	time: string;
	mcpmateCpuPercent: number;
	mcpmateMemoryPercent: number;
	systemCpuPercent: number;
	systemMemoryPercent: number;
	mcpmateMemoryMb: number;
	systemMemoryMb: number;
};

export type MetricsHistory = MetricsHistoryPoint[];

const METRICS_CHART_Y_PEAK_NEAR_FULL = 85;
const METRICS_CHART_Y_MIN_TOP = 5;

export function computeMetricsChartYAxisMax(history: MetricsHistoryPoint[]): number {
	if (history.length === 0) {
		return 100;
	}
	let peak = 0;
	for (const point of history) {
		peak = Math.max(peak, point.mcpmateCpuPercent, point.mcpmateMemoryPercent);
	}
	if (!Number.isFinite(peak) || peak < 0) {
		return 100;
	}
	if (peak === 0) {
		return METRICS_CHART_Y_MIN_TOP;
	}
	if (peak >= METRICS_CHART_Y_PEAK_NEAR_FULL) {
		return 100;
	}
	return Math.min(100, peak * 5);
}

export function metricsYAxisTickDecimalPlaces(axisMax: number): number {
	if (axisMax < 2) {
		return 2;
	}
	if (axisMax <= 25) {
		return 1;
	}
	return 0;
}

export function formatMetricsYAxisTick(value: number, axisMax: number): string {
	if (!Number.isFinite(value)) {
		return "";
	}
	const decimals = metricsYAxisTickDecimalPlaces(axisMax);
	return `${Number(value.toFixed(decimals))}%`;
}

function parseStoredHistory(raw: string | null): MetricsHistory {
	if (!raw) {
		return [];
	}
	try {
		const parsed = JSON.parse(raw);
		if (!Array.isArray(parsed)) {
			return [];
		}
		return parsed.filter((entry: unknown): entry is MetricsHistoryPoint => {
			if (!entry || typeof entry !== "object") {
				return false;
			}
			const candidate = entry as Record<string, unknown>;
			return (
				typeof candidate.time === "string" &&
				typeof candidate.mcpmateCpuPercent === "number" &&
				typeof candidate.mcpmateMemoryPercent === "number" &&
				typeof candidate.systemCpuPercent === "number" &&
				typeof candidate.systemMemoryPercent === "number" &&
				typeof candidate.mcpmateMemoryMb === "number" &&
				typeof candidate.systemMemoryMb === "number"
			);
		});
	} catch {
		return [];
	}
}

function finiteNumberOrNull(value: unknown): number | null {
	return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function clampPercent(value: number): number {
	return Math.min(100, Math.max(0, value));
}

function percentOf(value: number | null, total: number | null): number {
	if (value === null || total === null || total <= 0) {
		return 0;
	}
	return clampPercent((value / total) * 100);
}

function firstFiniteNumber(values: unknown[]): number | null {
	for (const value of values) {
		const numberValue = finiteNumberOrNull(value);
		if (numberValue !== null) {
			return numberValue;
		}
	}
	return null;
}

export function useMetricsHistory(): {
	history: MetricsHistory;
	latestPoint: MetricsHistoryPoint | null;
	isLoading: boolean;
} {
	const [history, setHistory] = React.useState<MetricsHistory>(() => {
		if (typeof window === "undefined") {
			return [];
		}
		return parseStoredHistory(window.localStorage.getItem(METRICS_HISTORY_STORAGE_KEY));
	});

	const metricsQuery = useQuery({
		queryKey: ["systemMetrics"],
		queryFn: systemApi.getMetrics,
		refetchInterval: 30_000,
		retry: false,
		refetchOnWindowFocus: false,
	});

	React.useEffect(() => {
		const metrics = metricsQuery.data;
		if (!metrics || typeof window === "undefined") {
			return;
		}

		const timestamp = metrics.timestamp ? new Date(metrics.timestamp) : new Date();

		const mcpmateCpuPercent = clampPercent(
			firstFiniteNumber([
				metrics.cpu_usage_percent,
				metrics.cpu_usage,
				metrics.system_cpu_usage,
			]) ?? 0,
		);

		const systemCpuValue = finiteNumberOrNull(metrics.system_cpu_usage);
		const systemCpuPercent =
			systemCpuValue !== null ? clampPercent(systemCpuValue) : mcpmateCpuPercent;

		const systemMemoryTotalBytes = finiteNumberOrNull(metrics.system_memory_total);
		const mcpmateMemoryBytes = firstFiniteNumber([
			metrics.memory_usage,
			metrics.memory_usage_bytes,
			metrics.system_memory_usage,
		]);

		const systemMemoryUsageBytes =
			finiteNumberOrNull(metrics.system_memory_usage) ?? mcpmateMemoryBytes;

		const mcpmateMemoryPercent = percentOf(mcpmateMemoryBytes, systemMemoryTotalBytes);
		const systemMemoryPercent = percentOf(systemMemoryUsageBytes, systemMemoryTotalBytes);

		const mcpmateMemoryMb =
			mcpmateMemoryBytes !== null ? mcpmateMemoryBytes / (1024 * 1024) : 0;
		const systemMemoryMb =
			systemMemoryUsageBytes !== null
				? systemMemoryUsageBytes / (1024 * 1024)
				: mcpmateMemoryMb;

		const point: MetricsHistoryPoint = {
			time: timestamp.toLocaleTimeString([], {
				hour: "2-digit",
				minute: "2-digit",
			}),
			mcpmateCpuPercent,
			mcpmateMemoryPercent,
			systemCpuPercent,
			systemMemoryPercent,
			mcpmateMemoryMb,
			systemMemoryMb,
		};
		setHistory((prev) => {
			const next = [...prev, point];
			const trimmed = next.slice(-60);
			try {
				window.localStorage.setItem(
					METRICS_HISTORY_STORAGE_KEY,
					JSON.stringify(trimmed),
				);
			} catch {
				/* noop */
			}
			return trimmed;
		});
		try {
			window.localStorage.removeItem("mcp_metrics_history");
			window.localStorage.removeItem("mcp_metrics_history_v2");
		} catch {
			/* noop */
		}
	}, [metricsQuery.data]);

	const latestPoint = history.length > 0 ? history[history.length - 1] : null;
	const isLoading = metricsQuery.isLoading && history.length === 0;

	return { history, latestPoint, isLoading };
}

export function useIsDarkMode(): boolean {
	const [isDarkMode, setIsDarkMode] = React.useState(() => {
		if (typeof document === "undefined") {
			return false;
		}
		return document.documentElement.classList.contains("dark");
	});

	React.useEffect(() => {
		if (typeof window === "undefined") {
			return;
		}
		const media = window.matchMedia("(prefers-color-scheme: dark)");
		const update = () => {
			setIsDarkMode(document.documentElement.classList.contains("dark") || media.matches);
		};
		update();
		const listener = (event: MediaQueryListEvent) => {
			setIsDarkMode(event.matches);
		};
		media.addEventListener("change", listener);
		return () => media.removeEventListener("change", listener);
	}, []);

	return isDarkMode;
}
