import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { secretsApi } from "../../lib/api";
import { usePageTranslations } from "../../lib/i18n/usePageTranslations";
import { kindOptionsForEditor } from "../../lib/secret-origin-hints";
import { notifyError, notifySuccess, stringifyError } from "../../lib/notify";
import type { SecretMetadata, SecretOrigin } from "../../lib/types";
import { SecretEditorDrawer } from "./secret-editor-drawer";
import {
	buildCreateEditorStateFromOrigin,
	SECRET_KIND_VALUES,
	type SecretEditorState,
} from "./secret-editor-state";

export interface InlineSecretCreateOpenRequest {
	origin: SecretOrigin;
	/** When set, saved placeholder is passed to `onCreated` for form field insertion. */
	fieldName?: string;
}

export interface UseInlineSecretCreateOptions {
	onCreated?: (fieldName: string | undefined, secret: SecretMetadata) => void;
}

export function useInlineSecretCreate(options: UseInlineSecretCreateOptions = {}) {
	usePageTranslations("secrets");
	const { t } = useTranslation("secrets");
	const queryClient = useQueryClient();
	const [editor, setEditor] = useState<SecretEditorState | null>(null);
	const [pendingFieldName, setPendingFieldName] = useState<string | undefined>();

	const kindOptions = useMemo(
		() =>
			SECRET_KIND_VALUES.map((value) => ({
				value,
				label: t(`kind.${value}`, { defaultValue: value }),
			})),
		[t],
	);

	const editorKindOptions = useMemo(
		() => kindOptionsForEditor(kindOptions, editor),
		[kindOptions, editor],
	);

	const dismiss = useCallback(() => {
		setEditor(null);
	}, []);

	const finalizeClose = useCallback(() => {
		setPendingFieldName(undefined);
		setEditor(null);
	}, []);

	const open = useCallback(
		async ({ origin, fieldName }: InlineSecretCreateOpenRequest) => {
			const secrets = await queryClient.fetchQuery({
				queryKey: ["secrets"],
				queryFn: secretsApi.list,
			});
			setEditor(
				buildCreateEditorStateFromOrigin(
					origin,
					secrets.map((secret) => secret.alias),
					(key, defaultValue) => t(key, { defaultValue }),
				),
			);
			setPendingFieldName(fieldName);
		},
		[queryClient, t],
	);

	const saveMutation = useMutation({
		mutationFn: async (state: SecretEditorState) => {
			return secretsApi.create({
				alias: state.alias.trim(),
				kind: state.kind,
				label: state.label.trim() || null,
				value: state.value,
				origin: state.origin,
			});
		},
		onSuccess: async (secret) => {
			const fieldName = pendingFieldName;
			dismiss();
			await queryClient.invalidateQueries({ queryKey: ["secrets"] });
			notifySuccess(
				t("notifications.saveSuccess", { defaultValue: "Secret saved" }),
			);
			options.onCreated?.(fieldName, secret);
		},
		onError: (error) => {
			notifyError(
				t("notifications.saveError", { defaultValue: "Failed to save secret" }),
				stringifyError(error),
			);
		},
	});

	const save = useCallback(() => {
		if (!editor) return;
		saveMutation.mutate(editor);
	}, [editor, saveMutation]);

	return {
		editor,
		setEditor,
		kindOptions: editorKindOptions,
		open,
		close: dismiss,
		finalizeClose,
		save,
		isSaving: saveMutation.isPending,
	};
}

export type InlineSecretCreateController = ReturnType<typeof useInlineSecretCreate>;

export function InlineSecretCreateDrawer({
	controller,
	nested = false,
}: {
	controller: InlineSecretCreateController;
	nested?: boolean;
}) {
	return (
		<SecretEditorDrawer
			editor={controller.editor}
			kindOptions={controller.kindOptions}
			onChange={controller.setEditor}
			onClose={controller.finalizeClose}
			onSave={controller.save}
			isSaving={controller.isSaving}
			nested={nested}
		/>
	);
}

/** Shorthand for {@link InlineSecretCreateDrawer}. */
export function InlineSecretCreate({
	controller,
	nested = false,
}: {
	controller: InlineSecretCreateController;
	nested?: boolean;
}) {
	return <InlineSecretCreateDrawer controller={controller} nested={nested} />;
}

/**
 * Field-bound inline create: open drawer from `SecretOrigin`, insert placeholder on save.
 * Use from Server forms today; any future origin-aware field can reuse the same API.
 */
export function useInlineSecretCreateField(
	onInsert: (fieldName: string, placeholder: string) => void,
) {
	const controller = useInlineSecretCreate({
		onCreated: (fieldName, secret) => {
			if (fieldName) {
				onInsert(fieldName, secret.placeholder);
			}
		},
	});

	const onCreateSecret = useCallback(
		(fieldName: string, origin: SecretOrigin) => {
			void controller.open({ origin, fieldName });
		},
		[controller.open],
	);

	return { onCreateSecret, controller };
}

export function useSecretEditorKindOptions(
	editor: SecretEditorState | null,
): Array<{ value: SecretEditorState["kind"]; label: string }> {
	const { t } = useTranslation("secrets");
	const kindOptions = useMemo(
		() =>
			SECRET_KIND_VALUES.map((value) => ({
				value,
				label: t(`kind.${value}`, { defaultValue: value }),
			})),
		[t],
	);
	return useMemo(
		() => kindOptionsForEditor(kindOptions, editor),
		[kindOptions, editor],
	);
}
