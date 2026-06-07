import { Controller } from "react-hook-form";
import type { Control, FieldErrors } from "react-hook-form";
import { Input } from "../../ui/input";
import { Label } from "../../ui/label";
import type { ManualServerFormValues } from "../types";
import { useTranslation } from "react-i18next";
import { SecretPickerButton } from "../secret-picker-button";
import type { SecretOrigin } from "../../../lib/types";

interface CommandFieldProps {
	kind: ManualServerFormValues["kind"];
	control: Control<ManualServerFormValues>;
	errors: FieldErrors<ManualServerFormValues>;
	commandId: string;
	urlId: string;
	commandInputRef: React.MutableRefObject<HTMLInputElement | null>;
	urlInputRef: React.MutableRefObject<HTMLInputElement | null>;
	viewMode: "form" | "json";
	onSecretSelect?: (fieldName: string, placeholder: string) => void;
	onCreateSecret?: (fieldName: string, origin: SecretOrigin) => void;
	secretOriginBase?: SecretOrigin;
}

export function CommandField({
	kind,
	control,
	errors,
	commandId,
	urlId,
	commandInputRef,
	urlInputRef,
	viewMode,
	onSecretSelect,
	onCreateSecret,
	secretOriginBase,
}: CommandFieldProps) {
	const { t } = useTranslation("servers");
	if (viewMode !== "form") return null;

	const isStdio = kind === "stdio";

	return isStdio ? (
		<div key={`stdio-${kind}`} className="flex items-center gap-4">
			<Label htmlFor={commandId} className="w-20 text-right">
				{t("manual.fields.command.label", { defaultValue: "Command" })}
			</Label>
			<div className="flex-1">
				<Controller
					name="command"
					control={control}
					render={({ field }) => (
						<div className="group/secret-field relative">
							<Input
								id={commandId}
								{...field}
								ref={(el) => {
									field.ref(el);
									commandInputRef.current = el;
								}}
								placeholder={t("manual.fields.command.placeholder", {
									defaultValue: "e.g., uvx my-mcp",
								})}
								className="pr-10"
							/>
							<SecretPickerButton
								className="absolute right-1 top-1/2 h-8 w-8 -translate-y-1/2"
								origin={{
									...secretOriginBase,
									field_group: "stdio",
									field_key: "command",
									field_path: "command",
								}}
								onCreateNew={
									onCreateSecret
										? (origin) => onCreateSecret("command", origin)
										: undefined
								}
								onSelect={(placeholder) =>
									onSecretSelect?.("command", placeholder)
								}
							/>
						</div>
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
		<div key={`url-${kind}`} className="flex items-center gap-4">
			<Label htmlFor={urlId} className="w-20 text-right">
				{t("manual.fields.url.label", { defaultValue: "Server URL" })}
			</Label>
			<div className="flex-1">
				<Controller
					name="url"
					control={control}
					render={({ field }) => (
						<div className="group/secret-field relative">
							<Input
								id={urlId}
								{...field}
								ref={(el) => {
									field.ref(el);
									urlInputRef.current = el;
								}}
								placeholder={t("manual.fields.url.placeholder", {
									defaultValue: "https://example.com/mcp",
								})}
								className="pr-10"
							/>
							<SecretPickerButton
								className="absolute right-1 top-1/2 h-8 w-8 -translate-y-1/2"
								origin={{
									...secretOriginBase,
									field_group: "streamable_http",
									field_key: "url",
									field_path: "url",
								}}
								onCreateNew={
									onCreateSecret
										? (origin) => onCreateSecret("url", origin)
										: undefined
								}
								onSelect={(placeholder) => onSecretSelect?.("url", placeholder)}
							/>
						</div>
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
