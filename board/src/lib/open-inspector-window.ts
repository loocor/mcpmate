const INSPECTOR_WINDOW_FEATURES = "noopener,noreferrer";

export function buildInspectorWindowUrl(path = "/inspector"): string {
	if (typeof window === "undefined") {
		return path;
	}
	return new URL(path, window.location.origin).toString();
}

export function openInspectorWindow(path = "/inspector"): Window | null {
	if (typeof window === "undefined") {
		return null;
	}
	return window.open(
		buildInspectorWindowUrl(path),
		"_blank",
		INSPECTOR_WINDOW_FEATURES,
	);
}

export function shouldOpenInspectorInSameTab(event: {
	metaKey: boolean;
	ctrlKey: boolean;
	shiftKey: boolean;
	altKey: boolean;
	button: number;
}): boolean {
	return (
		event.metaKey ||
		event.ctrlKey ||
		event.shiftKey ||
		event.altKey ||
		event.button !== 0
	);
}
