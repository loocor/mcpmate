import { useTranslation } from "react-i18next";
import type { UseFormRegister } from "react-hook-form";
import { Input } from "../../ui/input";
import { FieldList } from "../field-list";
import { SecretPickerButton } from "../secret-picker-button";
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
	deleteConfirmStates: Record<string, boolean>;
	onDeleteClick: (fieldId: string, removeFn: () => void) => void;
	onGhostClick: (addFn: () => void) => void;
	onSecretSelect?: (fieldName: string, placeholder: string) => void;
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
	deleteConfirmStates,
	onDeleteClick,
	onGhostClick,
	onSecretSelect,
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
					return (
						<div className="group/secret-field relative">
							<Input
								{...register(`args.${index}.value` as const)}
								placeholder={t("manual.fields.args.placeholder", {
									defaultValue: `Argument ${index + 1}`,
									count: index + 1,
								})}
								className="pr-20"
							/>
							<SecretPickerButton
								className="absolute right-9 top-1/2 h-7 w-7 -translate-y-1/2"
								origin={{
									...secretOriginBase,
									field_group: "args",
									field_index: index,
									field_path: `args.${index}.value`,
								}}
								onCreateNew={
									onCreateSecret
										? (origin) =>
												onCreateSecret(`args.${index}.value`, origin)
										: undefined
								}
								onSelect={(placeholder) =>
									onSecretSelect?.(`args.${index}.value`, placeholder)
								}
							/>
						</div>
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
				renderField={(field, index) => {
					if (field.id === "ghost") {
						return (
							<div className="grid grid-cols-2 gap-2">
								<Input
									placeholder={t("manual.fields.env.ghostKey", {
										defaultValue: "Add a new key",
									})}
									onClick={() =>
										onGhostClick(() => appendEnv({ key: "", value: "" }))
									}
									className={GHOST_INPUT_CLASS}
									readOnly
								/>
								<Input
									placeholder={t("manual.fields.common.ghostValue", {
										defaultValue: "Add a new value",
									})}
									onClick={() =>
										onGhostClick(() => appendEnv({ key: "", value: "" }))
									}
									className={GHOST_INPUT_CLASS}
									readOnly
								/>
							</div>
						);
					}
					return (
						<div className="group/secret-field grid grid-cols-2 gap-2">
							<Input
								{...register(`env.${index}.key` as const)}
								placeholder={t("manual.fields.env.keyPlaceholder", {
									defaultValue: "KEY",
								})}
							/>
							<Input
								{...register(`env.${index}.value` as const)}
								placeholder={t("manual.fields.common.valuePlaceholder", {
									defaultValue: "Value",
								})}
								className="pr-20"
							/>
							<SecretPickerButton
								className="absolute right-9 top-1/2 h-7 w-7 -translate-y-1/2"
								origin={{
									...secretOriginBase,
									field_group: "env",
									field_key:
										getEnvRowKeyAt?.(index) ??
										(typeof field.key === "string" ? field.key : undefined),
									field_index: index,
									field_path: `env.${index}.value`,
								}}
								onCreateNew={
									onCreateSecret
										? (origin) => onCreateSecret(`env.${index}.value`, origin)
										: undefined
								}
								onSelect={(placeholder) =>
									onSecretSelect?.(`env.${index}.value`, placeholder)
								}
							/>
						</div>
					);
				}}
			/>
		</div>
	);
}
