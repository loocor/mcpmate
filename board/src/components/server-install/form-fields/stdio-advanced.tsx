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
import { GHOST_INPUT_CLASS } from "../types";
import type { ManualServerFormValues } from "../types";

interface StdioAdvancedProps {
	viewMode: "form" | "json";
	isStdio: boolean;
	argFields: Array<{ id: string;[key: string]: unknown }>;
	envFields: Array<{ id: string;[key: string]: unknown }>;
	removeArg: (index: number) => void;
	removeEnv: (index: number) => void;
	appendArg: (value: { value: string }) => void;
	appendEnv: (value: { key: string; value: string }) => void;
	register: UseFormRegister<ManualServerFormValues>;
	control: Control<ManualServerFormValues>;
	deleteConfirmStates: Record<string, boolean>;
	onDeleteClick: (fieldId: string, removeFn: () => void) => void;
	onGhostClick: (addFn: () => void) => void;
	onCreateSecret?: (fieldName: string, origin: SecretOrigin) => void;
	secretOriginBase?: SecretOrigin;
	getEnvRowKeyAt?: (index: number) => string | undefined;
}

export function StdioAdvanced({
	viewMode,
	isStdio,
	argFields,
	envFields,
	removeArg,
	removeEnv,
	appendArg,
	appendEnv,
	register,
	control,
	deleteConfirmStates,
	onDeleteClick,
	onGhostClick,
	onCreateSecret,
	secretOriginBase,
	getEnvRowKeyAt,
}: StdioAdvancedProps) {
	const { t } = useTranslation("servers");
	if (viewMode !== "form" || !isStdio) return null;

	return (
		<div className="space-y-4">
			<FieldList
				label={t("manual.fields.args.label", { defaultValue: "Arguments" })}
				fields={argFields}
				onRemove={removeArg}
				deleteConfirmStates={deleteConfirmStates}
				onDeleteClick={onDeleteClick}
				embeddedRowActions
				renderField={(field, index) => {
					if (field.id === "ghost") {
						return (
							<Input
								placeholder={t("manual.fields.args.ghost", {
									defaultValue: "Add a new argument",
								})}
								onClick={() => onGhostClick(() => appendArg({ value: "" }))}
								className={GHOST_INPUT_CLASS}
								readOnly
							/>
						);
					}
					const origin: SecretOrigin = {
						...secretOriginBase,
						field_group: "args",
						field_index: index,
						field_path: `args.${index}.value`,
					};

					return (
						<Controller
							name={`args.${index}.value`}
							control={control}
							render={({ field: valueField }) => (
								<SecureStringField
									value={valueField.value ?? ""}
									onChange={valueField.onChange}
									onBlur={valueField.onBlur}
									name={valueField.name}
									pairRemove={{
										onClick: () =>
											onDeleteClick(field.id, () => removeArg(index)),
										confirmed: Boolean(deleteConfirmStates[field.id]),
									}}
									placeholder={t("manual.fields.args.placeholder", {
										defaultValue: `Argument ${index + 1}`,
										count: index + 1,
									})}
									className="w-full"
									origin={origin}
									onCreateSecret={
										onCreateSecret
											? (pickedOrigin) =>
												onCreateSecret(
													`args.${index}.value`,
													pickedOrigin ?? origin,
												)
											: undefined
									}
								/>
							)}
						/>
					);
				}}
			/>
			<FieldList
				label={t("manual.fields.env.label", {
					defaultValue: "Environment Variables",
				})}
				fields={envFields}
				onRemove={removeEnv}
				deleteConfirmStates={deleteConfirmStates}
				onDeleteClick={onDeleteClick}
				pairLayout
				renderField={(field, index) => {
					if (field.id === "ghost") {
						return (
							<PairGhostRow
								keyPlaceholder={t("manual.fields.env.ghostKey", {
									defaultValue: "Add a new key",
								})}
								valuePlaceholder={t("manual.fields.common.ghostValue", {
									defaultValue: "Add a new value",
								})}
								onAdd={() =>
									onGhostClick(() => appendEnv({ key: "", value: "" }))
								}
							/>
						);
					}

					const envKey =
						getEnvRowKeyAt?.(index) ??
						(typeof field.key === "string" ? field.key : undefined);
					const origin: SecretOrigin = {
						...secretOriginBase,
						field_group: "env",
						field_key: envKey,
						field_index: index,
						field_path: `env.${index}.value`,
					};

					return (
						<>
							<Input
								{...register(`env.${index}.key` as const)}
								placeholder={t("manual.fields.env.keyPlaceholder", {
									defaultValue: "KEY",
								})}
								className={FIELD_PAIR_KEY_INPUT_CLASS}
							/>
							<Controller
								name={`env.${index}.value`}
								control={control}
								render={({ field: valueField }) => (
									<SecureStringField
										value={valueField.value ?? ""}
										onChange={valueField.onChange}
										onBlur={valueField.onBlur}
										name={valueField.name}
										headerKey={envKey}
										pairLayout
										pairRemove={{
											onClick: () =>
												onDeleteClick(field.id, () => removeEnv(index)),
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
														`env.${index}.value`,
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
		</div>
	);
}
