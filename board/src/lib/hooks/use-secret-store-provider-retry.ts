import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { TFunction } from "i18next";
import { secretsApi } from "../api";
import { notifyError, notifySuccess, stringifyError } from "../notify";
import type { SwitchableSecretStoreProviderMode } from "../types";
import {
	invalidateSecretStoreCatalog,
	invalidateSecretStoreStatus,
} from "./use-secret-store-status";

interface UseSecretStoreProviderRetryOptions {
	invalidateCatalog?: boolean;
}

export function useSecretStoreProviderRetryMutation(
	t: TFunction,
	options: UseSecretStoreProviderRetryOptions = {},
) {
	const queryClient = useQueryClient();
	const { invalidateCatalog = false } = options;

	return useMutation({
		mutationFn: (mode: SwitchableSecretStoreProviderMode) =>
			secretsApi.switchProvider(mode),
		onSuccess: async (newStatus) => {
			await invalidateSecretStoreStatus(queryClient);

			if (newStatus.status === "ready") {
				if (invalidateCatalog) {
					await invalidateSecretStoreCatalog(queryClient);
				}
				notifySuccess(
					t("guidance.notifications.retrySuccess", {
						defaultValue: "Secure store status refreshed",
					}),
				);
				return;
			}

			notifyError(
				t("guidance.notifications.retryStillUnavailable", {
					defaultValue: "Secure store is still unavailable",
				}),
				newStatus.issue?.message ??
					t("guidance.generic.description", {
						defaultValue:
							"Secret storage is not ready. Create and update operations stay disabled until the issue is resolved.",
					}),
			);
		},
		onError: (error: unknown) => {
			notifyError(
				t("guidance.notifications.retryError", {
					defaultValue: "Failed to retry secure storage",
				}),
				stringifyError(error),
			);
		},
	});
}
