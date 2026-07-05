import { Controller } from "react-hook-form";
import { useTranslation } from "react-i18next";
import type { Control, UseFormRegister } from "react-hook-form";
import { Input } from "../../ui/input";
import {
	FIELD_PAIR_KEY_INPUT_CLASS,
	FIELD_PAIR_VALUE_CELL_CLASS,
	FieldList,
	PairGhostRow,
} from "../field-list";
import { SecureStringField } from "../secure-string-field";
import type { SecretOrigin } from "../../../lib/types";
import type { ManualServerFormValues } from "../types";

interface UrlParamsProps {
	viewMode: "form" | "json";
	isStdio: boolean;
	urlParamFields: Array<{ id: string;[key: string]: unknown }>;
	removeUrlParam: (index: number) => void;
	appendUrlParam: (value: { key: string; value: string }) => void;
	register: UseFormRegister<ManualServerFormValues>;
	control: Control<ManualServerFormValues>;
	deleteConfirmStates: Record<string, boolean>;
	onDeleteClick: (fieldId: string, removeFn: () => void) => void;
	onGhostClick: (addFn: () => void) => void;
	onCreateSecret?: (fieldName: string, origin: SecretOrigin) => void;
	secretOriginBase?: SecretOrigin;
	getRowKeyAt?: (index: number) => string | undefined;
	labelClassName?: string;
}

export function UrlParams({
	viewMode,
	isStdio,
	urlParamFields,
	removeUrlParam,
	appendUrlParam,
	register,
	control,
	deleteConfirmStates,
	onDeleteClick,
	onGhostClick,
	onCreateSecret,
	secretOriginBase,
	getRowKeyAt,
	labelClassName,
}: UrlParamsProps) {
	const { t } = useTranslation("servers");
	if (viewMode !== "form" || isStdio) return null;

	return (
		<FieldList
			label={t("manual.fields.urlParams.label", {
				defaultValue: "URL Parameters",
			})}
			labelClassName={labelClassName}
			fields={urlParamFields}
			onRemove={removeUrlParam}
			deleteConfirmStates={deleteConfirmStates}
			onDeleteClick={onDeleteClick}
			pairLayout
			renderField={(field, index) => {
				if (field.id === "ghost") {
					return (
						<PairGhostRow
							keyPlaceholder={t("manual.fields.urlParams.ghostKey", {
								defaultValue: "Parameter name",
							})}
							valuePlaceholder={t("manual.fields.urlParams.ghostValue", {
								defaultValue: "Value",
							})}
							onAdd={() =>
								onGhostClick(() => appendUrlParam({ key: "", value: "" }))
							}
						/>
					);
				}

				const paramKey =
					getRowKeyAt?.(index) ??
					(typeof field.key === "string" ? field.key : undefined);
				const origin: SecretOrigin = {
					...secretOriginBase,
					field_group: "url_params",
					field_key: paramKey,
					field_index: index,
					field_path: `urlParams.${index}.value`,
				};

				return (
					<>
						<Input
							{...register(`urlParams.${index}.key` as const)}
							placeholder={t("manual.fields.urlParams.keyPlaceholder", {
								defaultValue: "Parameter",
							})}
							className={FIELD_PAIR_KEY_INPUT_CLASS}
						/>
						<Controller
							name={`urlParams.${index}.value`}
							control={control}
							render={({ field: valueField }) => (
								<SecureStringField
									value={valueField.value ?? ""}
									onChange={valueField.onChange}
									onBlur={valueField.onBlur}
									name={valueField.name}
									headerKey={paramKey}
									pairLayout
									pairRemove={{
										onClick: () =>
											onDeleteClick(field.id, () => removeUrlParam(index)),
										confirmed: Boolean(deleteConfirmStates[field.id]),
									}}
									placeholder={t("manual.fields.common.valuePlaceholder", {
										defaultValue: "Value",
									})}
									className={FIELD_PAIR_VALUE_CELL_CLASS}
									origin={origin}
									onCreateSecret={
										onCreateSecret
											? (pickedOrigin) =>
												onCreateSecret(
													`urlParams.${index}.value`,
													pickedOrigin ?? origin,
												)
											: undefined
									}
								/>
							)}
						/>
					</>
				);
			}}
		/>
	);
}
