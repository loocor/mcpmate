import { useTranslation } from "react-i18next";
import type { UseFormRegister } from "react-hook-form";
import { Input } from "../../ui/input";
import { FieldList } from "../field-list";
import { SecretPickerButton } from "../secret-picker-button";
import type { SecretOrigin } from "../../../lib/types";
import type { ManualServerFormValues } from "../types";

interface HttpHeadersProps {
	viewMode: "form" | "json";
	isStdio: boolean;
	headerFields: Array<{ id: string; [key: string]: unknown }>;
	removeHeader: (index: number) => void;
	appendHeader: (value: { key: string; value: string }) => void;
	register: UseFormRegister<ManualServerFormValues>;
	deleteConfirmStates: Record<string, boolean>;
	onDeleteClick: (fieldId: string, removeFn: () => void) => void;
	onGhostClick: (addFn: () => void) => void;
	onSecretSelect?: (fieldName: string, placeholder: string) => void;
	secretOriginBase?: SecretOrigin;
}

export function HttpHeaders({
	viewMode,
	isStdio,
	headerFields,
	removeHeader,
	appendHeader,
	register,
	deleteConfirmStates,
	onDeleteClick,
	onGhostClick,
	onSecretSelect,
	secretOriginBase,
}: HttpHeadersProps) {
	const { t } = useTranslation("servers");
	if (viewMode !== "form" || isStdio) return null;

	return (
		<FieldList
			label={t("manual.fields.headers.label", { defaultValue: "HTTP Headers" })}
			fields={headerFields}
			onRemove={removeHeader}
			deleteConfirmStates={deleteConfirmStates}
			onDeleteClick={onDeleteClick}
			renderField={(field, index) => {
				if (field.id === "ghost") {
					return (
						<div className="grid grid-cols-2 gap-2">
							<Input
								placeholder={t("manual.fields.headers.ghostKey", {
									defaultValue: "Add a new header",
								})}
								onClick={() =>
									onGhostClick(() => appendHeader({ key: "", value: "" }))
								}
								className="border-dashed border-slate-300 bg-slate-50 hover:bg-slate-100 dark:border-slate-600 dark:bg-slate-800 dark:hover:bg-slate-700 cursor-pointer"
								readOnly
							/>
							<Input
								placeholder={t("manual.fields.common.ghostValue", {
									defaultValue: "Add a new value",
								})}
								onClick={() =>
									onGhostClick(() => appendHeader({ key: "", value: "" }))
								}
								className="border-dashed border-slate-300 bg-slate-50 hover:bg-slate-100 dark:border-slate-600 dark:bg-slate-800 dark:hover:bg-slate-700 cursor-pointer"
								readOnly
							/>
						</div>
					);
				}
				return (
					<div className="group/secret-field grid grid-cols-2 gap-2">
						<Input
							{...register(`headers.${index}.key` as const)}
							placeholder={t("manual.fields.headers.keyPlaceholder", {
								defaultValue: "Header",
							})}
						/>
						<div className="relative">
							<Input
								{...register(`headers.${index}.value` as const)}
								placeholder={t("manual.fields.common.valuePlaceholder", {
									defaultValue: "Value",
								})}
								className="pr-20"
							/>
							<SecretPickerButton
								className="absolute right-1 top-1/2 h-7 w-7 -translate-y-1/2"
								origin={{
									...secretOriginBase,
									field_group: "headers",
									field_key:
										typeof field.key === "string" ? field.key : undefined,
									field_index: index,
									field_path: `headers.${index}.value`,
								}}
								onSelect={(placeholder) =>
									onSecretSelect?.(`headers.${index}.value`, placeholder)
								}
							/>
						</div>
					</div>
				);
			}}
		/>
	);
}
