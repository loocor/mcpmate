import { useTranslation } from "react-i18next";
import { Input } from "../../ui/input";
import { FieldList } from "../field-list";

interface HttpHeadersProps {
	viewMode: "form" | "json";
	isStdio: boolean;
	headerFields: Array<{ id: string; [key: string]: unknown }>;
	removeHeader: (index: number) => void;
	appendHeader: (value: { key: string; value: string }) => void;
	register: any;
	deleteConfirmStates: Record<string, boolean>;
	onDeleteClick: (fieldId: string, removeFn: () => void) => void;
	onGhostClick: (addFn: () => void) => void;
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
					<div className="grid grid-cols-2 gap-2">
						<Input
							{...register(`headers.${index}.key` as const)}
							placeholder={t("manual.fields.headers.keyPlaceholder", {
								defaultValue: "Header",
							})}
						/>
						<Input
							{...register(`headers.${index}.value` as const)}
							placeholder={t("manual.fields.common.valuePlaceholder", {
								defaultValue: "Value",
							})}
						/>
					</div>
				);
			}}
		/>
	);
}
