import { useTranslation } from "react-i18next";

export function useBulkSelectionLabels() {
	const { t } = useTranslation();

	return {
		modeToggleLabel: t("bulkSelection.bulkModeEnter", {
			defaultValue: "Bulk select",
		}),
		modeExitLabel: t("bulkSelection.bulkModeExit", {
			defaultValue: "Exit bulk select",
		}),
		bulkModeDescription: (count: number) =>
			t("bulkSelection.bulkModeDescription", {
				count,
				defaultValue: "{{count}} selected for bulk actions",
			}),
	};
}
