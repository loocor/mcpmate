import type { FieldErrors, UseFormRegister } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { Avatar, AvatarFallback, AvatarImage } from "../../ui/avatar";
import { Input } from "../../ui/input";
import { Label } from "../../ui/label";
import { Textarea } from "../../ui/textarea";
import type { ManualFormStateJson, ManualServerFormValues } from "../types";

interface MetaFieldsProps {
	formStateRef: React.MutableRefObject<ManualFormStateJson>;
	register: UseFormRegister<ManualServerFormValues>;
	errors: FieldErrors<ManualServerFormValues>;
	iconUrl?: string;
	metaIconUrlId: string;
	metaDescriptionId: string;
	metaVersionId: string;
	metaWebsiteUrlId: string;
	metaRepositoryUrlId: string;
	metaRepositorySourceId: string;
	metaRepositorySubfolderId: string;
	metaRepositoryId: string;
}

export function MetaFields({
	formStateRef,
	register,
	errors,
	iconUrl,
	metaIconUrlId,
	metaDescriptionId,
	metaVersionId,
	metaWebsiteUrlId,
	metaRepositoryUrlId,
	metaRepositorySourceId,
	metaRepositorySubfolderId,
	metaRepositoryId,
}: MetaFieldsProps) {
	const fallback = (formStateRef.current.name || "S").slice(0, 1).toUpperCase();
	const previewIconUrl = (iconUrl ?? formStateRef.current.meta.icons?.[0]?.src ?? "").trim();
	const { t } = useTranslation("servers");

	return (
		<div className="space-y-4">
			<div className="flex items-center gap-3">
				<Label htmlFor={metaIconUrlId} className="w-20 shrink-0 text-right">
					{t("manual.fields.meta.icon.label", { defaultValue: "Icon" })}
				</Label>
				<div className="flex min-w-0 flex-1 items-center gap-3">
					<Avatar className="h-10 w-10 shrink-0 bg-slate-100 text-slate-700 dark:bg-slate-800 dark:text-slate-200">
						{previewIconUrl ? (
							<AvatarImage
								src={previewIconUrl}
								alt={t("manual.fields.meta.iconAlt", {
									defaultValue: "Server icon",
								})}
							/>
						) : null}
						<AvatarFallback>{fallback}</AvatarFallback>
					</Avatar>
					<div className="min-w-0 flex-1">
						<Input
							id={metaIconUrlId}
							{...register("meta_icon_url")}
							placeholder={t("manual.fields.meta.icon.placeholder", {
								defaultValue: "https://example.com/icon.png",
							})}
						/>
						{errors.meta_icon_url ? (
							<p className="mt-1 text-xs text-red-500">
								{t(errors.meta_icon_url.message ?? "", {
									defaultValue: errors.meta_icon_url.message,
								})}
							</p>
						) : null}
					</div>
				</div>
			</div>

			<div className="flex items-center gap-3">
				<Label htmlFor={metaVersionId} className="w-20 shrink-0 text-right">
					{t("manual.fields.meta.version.label", { defaultValue: "Version" })}
				</Label>
				<div className="flex-1">
					<Input
						id={metaVersionId}
						{...register("meta_version")}
						placeholder={t("manual.fields.meta.version.placeholder", {
							defaultValue: "e.g., 1.0.0",
						})}
					/>
				</div>
			</div>

			<div className="flex items-center gap-3">
				<Label htmlFor={metaWebsiteUrlId} className="w-20 shrink-0 text-right">
					{t("manual.fields.meta.website.label", { defaultValue: "Website" })}
				</Label>
				<div className="flex-1">
					<Input
						id={metaWebsiteUrlId}
						{...register("meta_website_url")}
						placeholder={t("manual.fields.meta.website.placeholder", {
							defaultValue: "https://example.com",
						})}
					/>
					{errors.meta_website_url ? (
						<p className="mt-1 text-xs text-red-500">
							{t(errors.meta_website_url.message ?? "", {
								defaultValue: errors.meta_website_url.message,
							})}
						</p>
					) : null}
				</div>
			</div>

			<div className="flex items-center gap-3">
				<Label htmlFor={metaRepositoryUrlId} className="w-20 shrink-0 text-right">
					{t("manual.fields.meta.repo.url.label", {
						defaultValue: "Repository URL",
					})}
				</Label>
				<div className="flex-1">
					<Input
						id={metaRepositoryUrlId}
						{...register("meta_repository_url")}
						placeholder={t("manual.fields.meta.repo.url.placeholder", {
							defaultValue: "https://github.com/org/repo",
						})}
					/>
					{errors.meta_repository_url ? (
						<p className="mt-1 text-xs text-red-500">
							{t(errors.meta_repository_url.message ?? "", {
								defaultValue: errors.meta_repository_url.message,
							})}
						</p>
					) : null}
				</div>
			</div>

			<div className="flex items-center gap-3">
				<Label htmlFor={metaRepositorySourceId} className="w-20 shrink-0 text-right">
					{t("manual.fields.meta.repo.source.label", {
						defaultValue: "Repository Source",
					})}
				</Label>
				<div className="flex-1">
					<Input
						id={metaRepositorySourceId}
						{...register("meta_repository_source")}
						placeholder={t("manual.fields.meta.repo.source.placeholder", {
							defaultValue: "e.g., github",
						})}
					/>
				</div>
			</div>

			<div className="flex items-center gap-3">
				<Label htmlFor={metaRepositorySubfolderId} className="w-20 shrink-0 text-right">
					{t("manual.fields.meta.repo.subfolder.label", {
						defaultValue: "Repository Subfolder",
					})}
				</Label>
				<div className="flex-1">
					<Input
						id={metaRepositorySubfolderId}
						{...register("meta_repository_subfolder")}
						placeholder={t("manual.fields.meta.repo.subfolder.placeholder", {
							defaultValue: "Optional subfolder",
						})}
					/>
				</div>
			</div>

			<div className="flex items-center gap-3">
				<Label htmlFor={metaRepositoryId} className="w-20 shrink-0 text-right">
					{t("manual.fields.meta.repo.id.label", {
						defaultValue: "Repository Entry ID (Metadata)",
					})}
				</Label>
				<div className="flex-1">
					<Input
						id={metaRepositoryId}
						{...register("meta_repository_id")}
						placeholder={t("manual.fields.meta.repo.id.placeholder", {
							defaultValue: "Optional metadata identifier",
						})}
					/>
				</div>
			</div>

			<div className="flex items-start gap-3">
				<Label htmlFor={metaDescriptionId} className="w-20 shrink-0 pt-3 text-right">
					{t("manual.fields.meta.description.label", {
						defaultValue: "Description",
					})}
				</Label>
				<div className="flex-1">
					<Textarea
						id={metaDescriptionId}
						{...register("meta_description")}
						placeholder={t("manual.fields.meta.description.placeholder", {
							defaultValue: "Short description",
						})}
						rows={3}
					/>
				</div>
			</div>
		</div>
	);
}
