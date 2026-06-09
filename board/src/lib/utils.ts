import { type ClassValue, clsx } from "clsx";
import { formatDistance } from "date-fns";
import { ja, zhCN } from "date-fns/locale";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]): string {
	return twMerge(clsx(inputs));
}

export function formatBytes(bytes: number, decimals = 2): string {
	if (bytes === 0) return "0 Bytes";

	const k = 1024;
	const dm = decimals < 0 ? 0 : decimals;
	const sizes = ["Bytes", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];

	const i = Math.floor(Math.log(bytes) / Math.log(k));

	return parseFloat((bytes / k ** i).toFixed(dm)) + " " + sizes[i];
}

export function formatUptime(seconds: number): string {
	if (seconds < 60) return `${seconds}s`;

	const minutes = Math.floor(seconds / 60);
	if (minutes < 60) return `${minutes}m ${seconds % 60}s`;

	const hours = Math.floor(minutes / 60);
	if (hours < 24) return `${hours}h ${minutes % 60}m`;

	const days = Math.floor(hours / 24);
	return `${days}d ${hours % 24}h`;
}

export function formatRelativeTime(timestamp: string, locale?: string): string {
	try {
		const date = new Date(timestamp);

		let dateLocale;
		if (locale?.startsWith("zh")) {
			dateLocale = zhCN;
		} else if (locale?.startsWith("ja")) {
			dateLocale = ja;
		} else {
			dateLocale = undefined;
		}

		return formatDistance(date, new Date(), {
			addSuffix: true,
			locale: dateLocale,
		});
	} catch {
		return "Invalid date";
	}
}

export function getStatusVariant(
	status: string,
): "success" | "warning" | "destructive" | "secondary" | "default" {
	const statusLower = status.toLowerCase();

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

	if (
		[
			"disconnected",
			"initializing",
			"shutdown",
			"starting",
			"connecting",
			"pending",
			"disabled",
			"fallback",
		].includes(statusLower)
	) {
		return "warning";
	}

	if (statusLower === "idle") {
		return "secondary";
	}

	if (
		["error", "unhealthy", "stopped", "failed", "timeout", "offline"].includes(statusLower)
	) {
		return "destructive";
	}

	return "default";
}

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

export function formatBackupTime(timestamp: string | null | undefined): string {
    return formatLocalDateTime(timestamp);
}

export function formatPathWithTilde(
	absolutePath: string | null | undefined,
	homeDir?: string | null,
): string {
	const raw = absolutePath?.trim() ?? "";
	const home = homeDir?.trim() ?? "";
	if (!raw || !home) {
		return raw;
	}

	const pathNorm = raw.replace(/\\/g, "/");
	const homeNorm = home.replace(/\\/g, "/").replace(/\/+$/, "");
	if (!homeNorm) {
		return raw;
	}

	if (pathNorm === homeNorm) {
		return "~";
	}

	const prefix = `${homeNorm}/`;
	if (pathNorm.startsWith(prefix)) {
		return `~/${pathNorm.slice(prefix.length)}`;
	}

	return raw;
}

export function truncate(str: string | undefined | null, length: number): string {
	if (!str || typeof str !== "string") return "N/A";
	if (str.length <= length) return str;
	return `${str.slice(0, length)}...`;
}

export function toTitleCase(value?: string | null): string {
	return (
		(value ?? "")
			.trim()
			.split(/[\s_-]+/)
			.filter(Boolean)
			.map((part) => part.charAt(0).toUpperCase() + part.slice(1).toLowerCase())
			.join(" ") ||
		value ||
		""
	);
}
