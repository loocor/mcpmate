import { useCallback } from "react";
import type { UseFormGetValues, UseFormSetValue } from "react-hook-form";
import { insertSecretPlaceholderIntoFieldValue } from "../../../lib/inline-secret-value";
import type { ManualServerFormValues } from "../types";

const KEY_VALUE_FIELD_PATTERN = /^(headers|env|urlParams)\.(\d+)\.value$/;

export function resolveSecretFieldHeaderKey(
	fieldName: string,
	getValues: UseFormGetValues<ManualServerFormValues>,
): string | null {
	const match = fieldName.match(KEY_VALUE_FIELD_PATTERN);
	if (!match) {
		return null;
	}
	const [, group, index] = match;
	const keyPath =
		`${group}.${index}.key` as keyof ManualServerFormValues;
	return (getValues(keyPath) as string | undefined) ?? null;
}

export function buildSecretInsertNextValue(
	fieldName: string,
	placeholder: string,
	getValues: UseFormGetValues<ManualServerFormValues>,
): string {
	const current = String(
		getValues(fieldName as keyof ManualServerFormValues) ?? "",
	);
	return insertSecretPlaceholderIntoFieldValue(current, placeholder, {
		headerKey: resolveSecretFieldHeaderKey(fieldName, getValues),
	});
}

export function useSecretFieldInsert(
	getValues: UseFormGetValues<ManualServerFormValues>,
	setValue: UseFormSetValue<ManualServerFormValues>,
) {
	return useCallback(
		(fieldName: string, placeholder: string) => {
			const nextValue = buildSecretInsertNextValue(
				fieldName,
				placeholder,
				getValues,
			);
			setValue(fieldName as keyof ManualServerFormValues, nextValue as never, {
				shouldDirty: true,
				shouldValidate: true,
			});
		},
		[getValues, setValue],
	);
}
