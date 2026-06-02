import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { KeyRound, Plus } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { secretsApi } from "../../lib/api";
import { notifyError, notifySuccess, stringifyError } from "../../lib/notify";
import type { SecretKind } from "../../lib/types";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import {
	Popover,
	PopoverContent,
	PopoverTrigger,
} from "../ui/popover";
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "../ui/select";

const SECRET_KIND_OPTIONS: Array<{ value: SecretKind; label: string }> = [
	{ value: "generic", label: "Generic" },
	{ value: "token", label: "Token" },
	{ value: "api_key", label: "API key" },
	{ value: "password", label: "Password" },
	{ value: "oauth_access_token", label: "OAuth access" },
	{ value: "oauth_refresh_token", label: "OAuth refresh" },
	{ value: "url_credential", label: "URL credential" },
	{ value: "header_value", label: "Header value" },
];

interface SecretPickerButtonProps {
	onSelect: (placeholder: string) => void;
	className?: string;
}

export function SecretPickerButton({
	onSelect,
	className,
}: SecretPickerButtonProps) {
	const { t } = useTranslation("servers");
	const queryClient = useQueryClient();
	const [open, setOpen] = useState(false);
	const [alias, setAlias] = useState("");
	const [value, setValue] = useState("");
	const [kind, setKind] = useState<SecretKind>("token");

	const secretsQuery = useQuery({
		queryKey: ["secrets"],
		queryFn: secretsApi.list,
		enabled: open,
	});

	const secrets = useMemo(
		() => [...(secretsQuery.data ?? [])].sort((left, right) => left.alias.localeCompare(right.alias)),
		[secretsQuery.data],
	);

	const createMutation = useMutation({
		mutationFn: () =>
			secretsApi.create({
				alias: alias.trim(),
				kind,
				label: null,
				value,
			}),
		onSuccess: async (secret) => {
			await queryClient.invalidateQueries({ queryKey: ["secrets"] });
			onSelect(secret.placeholder);
			setAlias("");
			setValue("");
			setKind("token");
			setOpen(false);
			notifySuccess(
				t("manual.secrets.createSuccess", { defaultValue: "Secret saved" }),
			);
		},
		onError: (error) => {
			notifyError(
				t("manual.secrets.createError", { defaultValue: "Failed to save secret" }),
				stringifyError(error),
			);
		},
	});

	const canCreate = alias.trim().length > 0 && value.length > 0;

	return (
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverTrigger asChild>
				<Button
					type="button"
					variant="ghost"
					size="icon"
					className={className}
					aria-label={t("manual.secrets.pick", { defaultValue: "Use secret" })}
				>
					<KeyRound className="h-4 w-4" />
				</Button>
			</PopoverTrigger>
			<PopoverContent align="end" className="w-80 p-3">
				<div className="space-y-3">
					<div>
						<div className="text-sm font-medium">
							{t("manual.secrets.title", { defaultValue: "Use Secure Store" })}
						</div>
						<p className="mt-1 text-xs text-muted-foreground">
							{t("manual.secrets.description", {
								defaultValue: "Insert a write-only placeholder into this runtime field.",
							})}
						</p>
					</div>
					<div className="max-h-48 space-y-1 overflow-y-auto">
						{secretsQuery.isLoading ? (
							<div className="px-2 py-3 text-sm text-muted-foreground">
								{t("manual.secrets.loading", { defaultValue: "Loading secrets" })}
							</div>
						) : secrets.length === 0 ? (
							<div className="px-2 py-3 text-sm text-muted-foreground">
								{t("manual.secrets.empty", { defaultValue: "No secrets stored" })}
							</div>
						) : (
							secrets.map((secret) => (
								<button
									key={secret.alias}
									type="button"
									className="flex w-full items-center justify-between rounded-md px-2 py-2 text-left text-sm hover:bg-accent"
									onClick={() => {
										onSelect(secret.placeholder);
										setOpen(false);
									}}
								>
									<span className="min-w-0">
										<span className="block truncate font-medium">{secret.alias}</span>
										<span className="block truncate font-mono text-xs text-muted-foreground">
											{secret.placeholder}
										</span>
									</span>
									<span className="ml-2 shrink-0 rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
										{secret.kind}
									</span>
								</button>
							))
						)}
					</div>
					<div className="border-t pt-3">
						<div className="mb-2 text-xs font-medium uppercase text-muted-foreground">
							{t("manual.secrets.createInline", { defaultValue: "Create New" })}
						</div>
						<div className="grid gap-2">
							<div className="grid gap-1.5">
								<Label htmlFor="inline-secret-alias">
									{t("manual.secrets.alias", { defaultValue: "Alias" })}
								</Label>
								<Input
									id="inline-secret-alias"
									value={alias}
									onChange={(event) => setAlias(event.target.value)}
									placeholder="server/github/token"
								/>
							</div>
							<div className="grid gap-1.5">
								<Label>
									{t("manual.secrets.kind", { defaultValue: "Kind" })}
								</Label>
								<Select value={kind} onValueChange={(next) => setKind(next as SecretKind)}>
									<SelectTrigger>
										<SelectValue />
									</SelectTrigger>
									<SelectContent>
										{SECRET_KIND_OPTIONS.map((option) => (
											<SelectItem key={option.value} value={option.value}>
												{option.label}
											</SelectItem>
										))}
									</SelectContent>
								</Select>
							</div>
							<div className="grid gap-1.5">
								<Label htmlFor="inline-secret-value">
									{t("manual.secrets.value", { defaultValue: "Value" })}
								</Label>
								<Input
									id="inline-secret-value"
									type="password"
									value={value}
									onChange={(event) => setValue(event.target.value)}
									placeholder={t("manual.secrets.valuePlaceholder", {
										defaultValue: "Secret value",
									})}
								/>
							</div>
							<Button
								type="button"
								size="sm"
								onClick={() => createMutation.mutate()}
								disabled={!canCreate || createMutation.isPending}
							>
								<Plus className="mr-2 h-4 w-4" />
								{t("manual.secrets.saveAndUse", { defaultValue: "Save and Use" })}
							</Button>
						</div>
					</div>
				</div>
			</PopoverContent>
		</Popover>
	);
}
