import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { buildEnableDisableBulkActions } from "./build-bulk-actions";
import type { BulkSelectionController } from "./types";

type EnableDisableBulkMutation = {
	isPending: boolean;
	mutate: (args: { enable: boolean; ids: string[] }) => void;
};

export function useEnableDisableBulkActions(
	bulk: BulkSelectionController,
	visibleIds: string[],
	mutation: EnableDisableBulkMutation,
) {
	const { t } = useTranslation();
	const { isPending, mutate } = mutation;

	return useMemo(
		() =>
			buildEnableDisableBulkActions({
				bulk,
				visibleIds,
				isPending,
				onEnable: () => mutate({ enable: true, ids: bulk.selectedIds }),
				onDisable: () => mutate({ enable: false, ids: bulk.selectedIds }),
				t,
			}),
		[bulk, visibleIds, isPending, mutate, t],
	);
}
