import { Controller } from "react-hook-form";
import type { Control, FieldErrors } from "react-hook-form";
import { Label } from "../../ui/label";
import type { ManualServerFormValues } from "../types";
import { useTranslation } from "react-i18next";
import { SecureStringField } from "../secure-string-field";
import type { SecretOrigin } from "../../../lib/types";
import { cn } from "../../../lib/utils";
import { SERVER_INSTALL_FORM_ROW_LABEL_CLASS } from "../field-list";

interface CommandFieldProps {
	kind: ManualServerFormValues["kind"];
	control: Control<ManualServerFormValues>;
	errors: FieldErrors<ManualServerFormValues>;
	commandId: string;
	urlId: string;
	viewMode: "form" | "json";
	onCreateSecret?: (fieldName: string, origin: SecretOrigin) => void;
	secretOriginBase?: SecretOrigin;
	labelClassName?: string;
}

export function CommandField({
	kind,
	control,
	errors,
	commandId,
	urlId,
	viewMode,
	onCreateSecret,
	secretOriginBase,
	labelClassName,
}: CommandFieldProps) {
	const { t } = useTranslation("servers");
	if (viewMode !== "form") return null;

	const isStdio = kind === "stdio";

	return isStdio ? (
		<div key={`stdio-${kind}`} className="flex items-center gap-3">
			<Label
				htmlFor={commandId}
				className={cn(SERVER_INSTALL_FORM_ROW_LABEL_CLASS, labelClassName)}
			>
				{t("manual.fields.command.label", { defaultValue: "Command" })}
			</Label>
			<div className="min-w-0 flex-1">
				<Controller
					name="command"
					control={control}
					render={({ field }) => (
						<SecureStringField
							id={commandId}
							value={field.value ?? ""}
							onChange={field.onChange}
							onBlur={field.onBlur}
							name={field.name}
							placeholder={t("manual.fields.command.placeholder", {
								defaultValue: "e.g., uvx my-mcp",
							})}
							origin={{
								...secretOriginBase,
								field_group: "stdio",
								field_key: "command",
								field_path: "command",
							}}
							onCreateSecret={
								onCreateSecret
									? (origin) => onCreateSecret("command", origin)
									: undefined
							}
						/>
					)}
				/>
				{errors.command && (
					<p className="text-xs text-red-500">
						{t(errors.command.message ?? "", {
							defaultValue: errors.command.message,
						})}
					</p>
				)}
			</div>
		</div>
	) : (
		<div key={`url-${kind}`} className="flex items-center gap-3">
			<Label
				htmlFor={urlId}
				className={cn(SERVER_INSTALL_FORM_ROW_LABEL_CLASS, labelClassName)}
			>
				{t("manual.fields.url.label", { defaultValue: "Server URL" })}
			</Label>
			<div className="min-w-0 flex-1">
				<Controller
					name="url"
					control={control}
					render={({ field }) => (
						<SecureStringField
							id={urlId}
							value={field.value ?? ""}
							onChange={field.onChange}
							onBlur={field.onBlur}
							name={field.name}
							placeholder={t("manual.fields.url.placeholder", {
								defaultValue: "https://example.com/mcp",
							})}
							origin={{
								...secretOriginBase,
								field_group: "streamable_http",
								field_key: "url",
								field_path: "url",
							}}
							onCreateSecret={
								onCreateSecret
									? (origin) => onCreateSecret("url", origin)
									: undefined
							}
						/>
					)}
				/>
				{errors.url && (
					<p className="text-xs text-red-500">
						{t(errors.url.message ?? "", {
							defaultValue: errors.url.message,
						})}
					</p>
				)}
			</div>
		</div>
	);
}
