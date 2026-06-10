const EDITABLE_SELECTOR =
	"input, textarea, select, [contenteditable=''], [contenteditable='true']";

export function shouldBlockDesktopDropNavigation(
	dataTransfer: DataTransfer | null,
	target: EventTarget | null,
): boolean {
	const types = dataTransfer?.types;
	if (!types?.length) {
		return false;
	}
	if (
		"contains" in types
			? (types as unknown as DOMStringList).contains("Files")
			: (types as readonly string[]).includes("Files")
	) {
		return true;
	}
	const el = target as { closest?: (s: string) => unknown } | null;
	return (
		typeof el?.closest !== "function" || el.closest(EDITABLE_SELECTOR) === null
	);
}
