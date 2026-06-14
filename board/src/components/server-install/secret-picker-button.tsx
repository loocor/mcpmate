import { useQuery } from "@tanstack/react-query";
import { KeyRound, Plus, Search, ShieldAlert } from "lucide-react";
import { useMemo, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { secretsApi } from "../../lib/api";
import { useSecretStoreStatusQuery } from "../../lib/hooks/use-secret-store-status";
import { suggestSecretAliasFromOrigin } from "../../lib/secret-alias";
import { isUserCreatableSecretKind } from "../../lib/secret-origin-hints";
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

	const storeStatusQuery = useSecretStoreStatusQuery();
	const storeReady = storeStatusQuery.data?.status === "ready";
	const storeUnavailable =
		storeStatusQuery.isError ||
		(storeStatusQuery.data != null && storeStatusQuery.data.status !== "ready");

	const secretsQuery = useQuery({
		queryKey: ["secrets"],
		queryFn: secretsApi.list,
		enabled: open && storeReady,
	});

	const secrets = useMemo(() => {
		const normalizedQuery = query.trim().toLowerCase();
		return [...(secretsQuery.data ?? [])]
			.sort((left, right) => left.alias.localeCompare(right.alias))
			.filter((secret) => isUserCreatableSecretKind(secret.kind))
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

	if (!storeReady && !storeUnavailable) return null;

	const triggerVisibilityClassName = forceVisible
		? "opacity-100"
		: "opacity-0 transition-opacity group-focus-within/secret-field:opacity-100 data-[state=open]:opacity-100";
	const triggerLabel = storeUnavailable
		? t("manual.secrets.unavailablePick", {
				defaultValue: "Secure Store unavailable",
			})
		: t("manual.secrets.pick", { defaultValue: "Use secret" });
	const title = storeUnavailable
		? t("manual.secrets.unavailableTitle", {
				defaultValue: "Secure Store unavailable",
			})
		: t("manual.secrets.title", {
				defaultValue: "Use Secure Store",
			});
	const description = storeUnavailable
		? t("manual.secrets.unavailableDescription", {
				defaultValue:
					"Secret placeholders cannot be selected until secure storage is restored.",
			})
		: t("manual.secrets.description", {
				defaultValue: "Insert a write-only placeholder into this runtime field.",
			});
	const actionLabel = storeUnavailable
		? t("manual.secrets.openSecuritySettings", {
				defaultValue: "Security settings",
			})
		: t("manual.secrets.createInline", { defaultValue: "New secret" });

	const handleFooterAction = () => {
		setOpen(false);
		if (storeUnavailable) {
			navigate("/settings?tab=security");
			return;
		}
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
	};

	let bodyContent: ReactNode;
	if (storeUnavailable) {
		bodyContent = (
			<div className="flex min-h-32 flex-col items-center justify-center rounded-md border border-dashed bg-muted/30 px-4 py-6 text-center">
				<div className="mb-2 flex h-9 w-9 items-center justify-center rounded-full bg-amber-50 text-amber-600">
					<ShieldAlert className="h-5 w-5" />
				</div>
				<p className="text-sm font-medium">
					{t("manual.secrets.unavailableListTitle", {
						defaultValue: "Secret access needs attention",
					})}
				</p>
				<p className="mt-1 text-xs text-muted-foreground">
					{t("manual.secrets.unavailableListDescription", {
						defaultValue:
							"Open Security settings to restore Secure Store access.",
					})}
				</p>
			</div>
		);
	} else {
		bodyContent = (
			<>
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
			</>
		);
	}

	return (
		<Popover open={open} onOpenChange={setOpen}>
			<PopoverTrigger asChild>
				<Button
					type="button"
					variant="ghost"
					size="icon"
					className={cn(
						triggerVisibilityClassName,
						storeUnavailable ? "text-destructive hover:text-destructive" : null,
						className,
					)}
					aria-label={triggerLabel}
				>
					<KeyRound className="h-4 w-4" />
				</Button>
			</PopoverTrigger>
			<PopoverContent align="end" className="w-80 p-3">
				<div className="space-y-3">
					<div>
						<div className="text-sm font-medium">{title}</div>
						<p className="mt-1 text-xs text-muted-foreground">{description}</p>
					</div>
					{bodyContent}
					<div className="border-t pt-2">
						<button
							type="button"
							className="flex w-full items-center gap-2 rounded-md px-2 py-2 text-left text-sm font-medium hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
							onClick={handleFooterAction}
						>
							{storeUnavailable ? (
								<KeyRound className="h-4 w-4" />
							) : (
								<Plus className="h-4 w-4" />
							)}
							{actionLabel}
						</button>
					</div>
				</div>
			</PopoverContent>
		</Popover>
	);
}
