import { useTranslation } from "react-i18next";
import type { UseFormRegister } from "react-hook-form";
import { Input } from "../../ui/input";
import { FieldList } from "../field-list";
import { SecretPickerButton } from "../secret-picker-button";
import type { SecretOrigin } from "../../../lib/types";
import type { ManualServerFormValues } from "../types";

interface UrlParamsProps {
	viewMode: "form" | "json";
	isStdio: boolean;
	urlParamFields: Array<{ id: string; [key: string]: unknown }>;
	removeUrlParam: (index: number) => void;
	appendUrlParam: (value: { key: string; value: string }) => void;
	register: UseFormRegister<ManualServerFormValues>;
	deleteConfirmStates: Record<string, boolean>;
	onDeleteClick: (fieldId: string, removeFn: () => void) => void;
	onGhostClick: (addFn: () => void) => void;
	onSecretSelect?: (fieldName: string, placeholder: string) => void;
	secretOriginBase?: SecretOrigin;
}

export function UrlParams({
	viewMode,
	isStdio,
	urlParamFields,
	removeUrlParam,
	appendUrlParam,
	register,
	deleteConfirmStates,
	onDeleteClick,
	onGhostClick,
	onSecretSelect,
	secretOriginBase,
}: UrlParamsProps) {
	const { t } = useTranslation("servers");
	if (viewMode !== "form" || isStdio) return null;

	return (
		<FieldList
			label={t("manual.fields.urlParams.label", {
				defaultValue: "URL Parameters",
			})}
			fields={urlParamFields}
			onRemove={removeUrlParam}
			deleteConfirmStates={deleteConfirmStates}
			onDeleteClick={onDeleteClick}
			renderField={(field, index) => {
				if (field.id === "ghost") {
					return (
						<div className="grid grid-cols-2 gap-2">
							<Input
								placeholder={t("manual.fields.urlParams.ghostKey", {
									defaultValue: "Parameter name",
								})}
								onClick={() =>
									onGhostClick(() => appendUrlParam({ key: "", value: "" }))
								}
								className="border-dashed border-slate-300 bg-slate-50 hover:bg-slate-100 dark:border-slate-600 dark:bg-slate-800 dark:hover:bg-slate-700 cursor-pointer"
								readOnly
							/>
							<Input
								placeholder={t("manual.fields.urlParams.ghostValue", {
									defaultValue: "Value",
								})}
								onClick={() =>
									onGhostClick(() => appendUrlParam({ key: "", value: "" }))
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
							{...register(`urlParams.${index}.key` as const)}
							placeholder={t("manual.fields.urlParams.keyPlaceholder", {
								defaultValue: "Parameter",
							})}
						/>
						<div className="relative">
							<Input
								{...register(`urlParams.${index}.value` as const)}
								placeholder={t("manual.fields.common.valuePlaceholder", {
									defaultValue: "Value",
								})}
								className="pr-20"
							/>
							<SecretPickerButton
								className="absolute right-1 top-1/2 h-7 w-7 -translate-y-1/2"
								origin={{
									...secretOriginBase,
									field_group: "url_params",
									field_key:
										typeof field.key === "string" ? field.key : undefined,
									field_index: index,
									field_path: `urlParams.${index}.value`,
								}}
								onSelect={(placeholder) =>
									onSecretSelect?.(`urlParams.${index}.value`, placeholder)
								}
							/>
						</div>
					</div>
				);
			}}
		/>
	);
}
