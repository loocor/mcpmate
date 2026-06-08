import { useQuery } from "@tanstack/react-query";
import { KeyRound, Plus, Search } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { secretsApi } from "../../lib/api";
import { suggestSecretAliasFromOrigin } from "../../lib/secret-alias";
import type { SecretOrigin } from "../../lib/types";
import { cn } from "../../lib/utils";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "../ui/popover";

interface SecretPickerButtonProps {
	onSelect: (placeholder: string) => void;
	onCreateNew?: (origin: SecretOrigin) => void;
	className?: string;
	origin?: SecretOrigin;
	/** Keep visible for read-only redacted fields. */
	forceVisible?: boolean;
}

export function SecretPickerButton({
	onSelect,
	onCreateNew,
	className,
	origin,
	forceVisible = false,
}: SecretPickerButtonProps) {
	const { t } = useTranslation("servers");
	const navigate = useNavigate();
	const [open, setOpen] = useState(false);
	const [query, setQuery] = useState("");

	const secretsQuery = useQuery({
		queryKey: ["secrets"],
		queryFn: secretsApi.list,
		enabled: open,
	});

	const storeStatusQuery = useQuery({
		queryKey: ["secrets", "status"],
		queryFn: secretsApi.status,
	});
	const storeReady = storeStatusQuery.data?.status === "ready";

	const secrets = useMemo(() => {
		const normalizedQuery = query.trim().toLowerCase();
		return [...(secretsQuery.data ?? [])]
			.sort((left, right) => left.alias.localeCompare(right.alias))
			.filter((secret) => {
				if (!normalizedQuery) return true;
				return (
					secret.alias.toLowerCase().includes(normalizedQuery) ||
					secret.placeholder.toLowerCase().includes(normalizedQuery) ||
					secret.kind.toLowerCase().includes(normalizedQuery) ||
					(secret.label ?? "").toLowerCase().includes(normalizedQuery)
				);
			});
	}, [query, secretsQuery.data]);

	if (!storeReady) return null;

	return (
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverTrigger asChild>
				<Button
					type="button"
					variant="ghost"
					size="icon"
					className={cn(
						forceVisible
							? "opacity-100"
							: "opacity-0 transition-opacity group-focus-within/secret-field:opacity-100 data-[state=open]:opacity-100",
						className,
					)}
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
								defaultValue:
									"Insert a write-only placeholder into this runtime field.",
							})}
						</p>
					</div>
					<div className="relative">
						<Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
						<Input
							value={query}
							onChange={(event) => setQuery(event.target.value)}
							placeholder={t("manual.secrets.search", {
								defaultValue: "Search secrets...",
							})}
							className="h-9 pl-9"
						/>
					</div>
					<div className="max-h-56 space-y-1 overflow-y-auto">
						{secretsQuery.isLoading ? (
							<div className="px-2 py-3 text-sm text-muted-foreground">
								{t("manual.secrets.loading", {
									defaultValue: "Loading secrets",
								})}
							</div>
						) : secrets.length === 0 ? (
							<div className="px-2 py-3 text-sm text-muted-foreground">
								{t("manual.secrets.empty", {
									defaultValue: "No secrets stored",
								})}
							</div>
						) : (
							secrets.map((secret) => (
								<button
									key={secret.alias}
									type="button"
									className="flex w-full items-center justify-between rounded-md px-2 py-2 text-left text-sm hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
									onClick={() => {
										onSelect(secret.placeholder);
										setOpen(false);
									}}
								>
									<span className="min-w-0">
										<span className="block truncate font-medium">
											{secret.alias}
										</span>
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
					<div className="border-t pt-2">
						<button
							type="button"
							className="flex w-full items-center gap-2 rounded-md px-2 py-2 text-left text-sm font-medium hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
							onClick={() => {
								setOpen(false);
								if (origin && onCreateNew) {
									onCreateNew(origin);
									return;
								}
								const params = new URLSearchParams({ editor: "create" });
								if (origin) {
									for (const [key, value] of Object.entries(origin)) {
										if (value === null || value === undefined || value === "") {
											continue;
										}
										params.set(`origin_${key}`, String(value));
									}
									const suggestedAlias = suggestSecretAliasFromOrigin(
										origin,
										(secretsQuery.data ?? []).map((secret) => secret.alias),
									);
									if (suggestedAlias) {
										params.set("suggested_alias", suggestedAlias);
									}
								}
								navigate(`/secrets?${params.toString()}`);
							}}
						>
							<Plus className="h-4 w-4" />
							{t("manual.secrets.createInline", { defaultValue: "New secret" })}
						</button>
					</div>
				</div>
			</PopoverContent>
		</Popover>
	);
}
