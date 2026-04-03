import { AlertTriangle, KeyRound, ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "./ui/badge";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "./ui/tooltip";

interface ServerAuthBadgeProps {
	authMode?: string | null;
	oauthStatus?: string | null;
	showLabel?: boolean;
}

export function ServerAuthBadge({
	authMode,
	oauthStatus,
	showLabel = true,
}: ServerAuthBadgeProps) {
	const { t } = useTranslation("servers");

	const normalizedMode = (authMode ?? "").toLowerCase();
	const normalizedStatus = (oauthStatus ?? "").toLowerCase();

	const content = (() => {
		if (normalizedMode === "header") {
			return {
				kind: "badge" as const,
				label: t("entity.connectionTags.headerAuth", {
					defaultValue: "Header auth",
				}),
				className:
					"border-slate-200 text-slate-600 dark:border-slate-700 dark:text-slate-300",
				icon: <KeyRound className="h-3 w-3" />,
			};
		}

		if (normalizedMode === "oauth") {
			if (normalizedStatus === "expired" || normalizedStatus === "disconnected") {
				return {
					kind: "warning" as const,
					label: t("entity.connectionTags.oauthWarning", {
						defaultValue: "Authorization expired — reauthorize in Edit",
					}),
				};
			}

			return {
				kind: "badge" as const,
				label: t("entity.connectionTags.oauth", {
					defaultValue: "OAuth",
				}),
				className:
					"border-emerald-200 text-emerald-700 dark:border-emerald-800 dark:text-emerald-300",
				icon: <ShieldCheck className="h-3 w-3" />,
			};
		}

		return null;
	})();

	if (!content) {
		return null;
	}

	if (content.kind === "warning") {
		return (
			<TooltipProvider>
				<Tooltip>
					<TooltipTrigger asChild>
						<span className="inline-flex items-center">
							<AlertTriangle className="h-4 w-4 text-red-500 animate-pulse" />
						</span>
					</TooltipTrigger>
					<TooltipContent>
						<p>{content.label}</p>
					</TooltipContent>
				</Tooltip>
			</TooltipProvider>
		);
	}

	return (
		<Badge variant="outline" className={`gap-1.5 ${content.className}`}>
			{content.icon}
			{showLabel ? content.label : null}
		</Badge>
	);
}
