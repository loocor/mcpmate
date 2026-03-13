import { type ClassValue, clsx } from "clsx";
import { formatDistance } from "date-fns";
import { zhCN } from "date-fns/locale";
import { ja } from "date-fns/locale";
import { twMerge } from "tailwind-merge";

/**
 * Combines multiple class names with Tailwind support
 */
export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

/**
 * Formats a number of bytes to a human-readable string
 */
export function formatBytes(bytes: number, decimals = 2) {
	if (bytes === 0) return "0 Bytes";

	const k = 1024;
	const dm = decimals < 0 ? 0 : decimals;
	const sizes = ["Bytes", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];

	const i = Math.floor(Math.log(bytes) / Math.log(k));

	return parseFloat((bytes / k ** i).toFixed(dm)) + " " + sizes[i];
}

/**
 * Formats an uptime in seconds to a human-readable string
 */
export function formatUptime(seconds: number) {
	if (seconds < 60) return `${seconds}s`;

	const minutes = Math.floor(seconds / 60);
	if (minutes < 60) return `${minutes}m ${seconds % 60}s`;

	const hours = Math.floor(minutes / 60);
	if (hours < 24) return `${hours}h ${minutes % 60}m`;

	const days = Math.floor(hours / 24);
	return `${days}d ${hours % 24}h`;
}

/**
 * Formats a timestamp to a relative time
 */
export function formatRelativeTime(timestamp: string, locale?: string) {
	try {
		const date = new Date(timestamp);

		// Select locale based on language
		let dateLocale;
		if (locale?.startsWith("zh")) {
			dateLocale = zhCN;
		} else if (locale?.startsWith("ja")) {
			dateLocale = ja;
		} else {
			dateLocale = undefined; // Use default English
		}

		return formatDistance(date, new Date(), {
			addSuffix: true,
			locale: dateLocale,
		});
	} catch (error) {
		return "Invalid date";
	}
}

/**
 * Get a color variant based on status
 */
export function getStatusVariant(
	status: string,
): "success" | "warning" | "destructive" | "secondary" | "default" {
	const statusLower = status.toLowerCase();

	// Active/Ready states
	if (
		[
			"connected",
			"running",
			"healthy",
			"ready",
			"busy",
			"active",
			"enabled",
			"thinking",
			"fetch",
		].includes(statusLower)
	) {
		return "success";
	}

	// Warning/Initializing states
	if (
		[
			"disconnected",
			"initializing",
			"shutdown",
			"starting",
			"connecting",
			"pending",
			"disabled",
		].includes(statusLower)
	) {
		return "warning";
	}

	// Idle state - server is enabled but not running
	if (["idle"].includes(statusLower)) {
		return "secondary";
	}

	// Error states
	if (
		["error", "unhealthy", "stopped", "failed", "timeout"].includes(statusLower)
	) {
		return "destructive";
	}

	// Unknown or other states
	return "default";
}

/**
 * Formats a backup timestamp to a user-friendly local time format
 */
export function formatLocalDateTime(
    timestamp: string | number | Date | null | undefined,
    options?: Intl.DateTimeFormatOptions,
): string {
    if (timestamp === null || timestamp === undefined) return "-";
    try {
        const date = timestamp instanceof Date ? timestamp : new Date(timestamp);
        return date.toLocaleString(undefined, {
            year: "numeric",
            month: "2-digit",
            day: "2-digit",
            hour: "2-digit",
            minute: "2-digit",
            second: "2-digit",
            hour12: false,
            ...options,
        });
    } catch {
        return "Invalid date";
    }
}

// Backwards-compat wrapper: now always absolute local time
export function formatBackupTime(timestamp: string | null | undefined): string {
    return formatLocalDateTime(timestamp);
}

/**
 * Truncate a string if it's too long
 */
export function truncate(str: string | undefined | null, length: number) {
	if (!str || typeof str !== "string") return "N/A";
	if (str.length <= length) return str;
	return `${str.slice(0, length)}...`;
}
