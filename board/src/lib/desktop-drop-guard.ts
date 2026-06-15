import {
	SERVER_UNI_IMPORT_TRANSFER_TYPES,
	hasDataTransferType,
} from "./server-uni-import-transfer";

const EDITABLE_SELECTOR =
	"input, textarea, select, [contenteditable=''], [contenteditable='true']";
const DESKTOP_DROP_TARGET_SELECTOR = "[data-desktop-drop-target='server-import']";

function hasIngestibleDropType(types: DataTransfer["types"]): boolean {
	return SERVER_UNI_IMPORT_TRANSFER_TYPES.some((type) =>
		hasDataTransferType(types, type),
	);
}

function targetMatchesSelector(
	target: EventTarget | null,
	selector: string,
): boolean {
	const el = target as { closest?: (s: string) => unknown } | null;
	return typeof el?.closest === "function" && el.closest(selector) !== null;
}

export function shouldBlockDesktopDropNavigation(
	dataTransfer: DataTransfer | null,
	target: EventTarget | null,
): boolean {
	const types = dataTransfer?.types;
	if (!types?.length) {
		return false;
	}
	if (
		targetMatchesSelector(target, DESKTOP_DROP_TARGET_SELECTOR) &&
		hasIngestibleDropType(types)
	) {
		return false;
	}
	if (hasDataTransferType(types, "Files")) {
		return true;
	}
	return !targetMatchesSelector(target, EDITABLE_SELECTOR);
}
