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

interface HttpHeadersProps {
	viewMode: "form" | "json";
	isStdio: boolean;
	labelTooltip?: string;
	headerFields: Array<{ id: string;[key: string]: unknown }>;
	removeHeader: (index: number) => void;
	appendHeader: (value: { key: string; value: string }) => void;
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

export function HttpHeaders({
	viewMode,
	isStdio,
	labelTooltip,
	headerFields,
	removeHeader,
	appendHeader,
	register,
	control,
	deleteConfirmStates,
	onDeleteClick,
	onGhostClick,
	onCreateSecret,
	secretOriginBase,
	getRowKeyAt,
	labelClassName,
}: HttpHeadersProps) {
	const { t } = useTranslation("servers");
	if (viewMode !== "form" || isStdio) return null;

	return (
		<FieldList
			label={t("manual.fields.headers.label", { defaultValue: "HTTP Headers" })}
			labelTooltip={labelTooltip}
			labelClassName={labelClassName}
			fields={headerFields}
			onRemove={removeHeader}
			deleteConfirmStates={deleteConfirmStates}
			onDeleteClick={onDeleteClick}
			pairLayout
			renderField={(field, index) => {
				if (field.id === "ghost") {
					return (
						<PairGhostRow
							keyPlaceholder={t("manual.fields.headers.ghostKey", {
								defaultValue: "Add a new header",
							})}
							valuePlaceholder={t("manual.fields.common.ghostValue", {
								defaultValue: "Add a new value",
							})}
							onAdd={() =>
								onGhostClick(() => appendHeader({ key: "", value: "" }))
							}
						/>
					);
				}

				const headerKey =
					getRowKeyAt?.(index) ??
					(typeof field.key === "string" ? field.key : undefined);
				const origin: SecretOrigin = {
					...secretOriginBase,
					field_group: "headers",
					field_key: headerKey,
					field_index: index,
					field_path: `headers.${index}.value`,
				};

				return (
					<>
						<Input
							{...register(`headers.${index}.key` as const)}
							placeholder={t("manual.fields.headers.keyPlaceholder", {
								defaultValue: "Header",
							})}
							className={FIELD_PAIR_KEY_INPUT_CLASS}
						/>
						<Controller
							name={`headers.${index}.value`}
							control={control}
							render={({ field: valueField }) => (
								<SecureStringField
									value={valueField.value ?? ""}
									onChange={valueField.onChange}
									onBlur={valueField.onBlur}
									name={valueField.name}
									headerKey={headerKey}
									pairLayout
									pairRemove={{
										onClick: () =>
											onDeleteClick(field.id, () => removeHeader(index)),
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
													`headers.${index}.value`,
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
