import type { LucideIcon } from "lucide-react";
import { RefreshCw, ShieldAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import {
	resolveSecretStoreIssueGuidance,
	type SwitchableSecretStoreProviderMode,
} from "../../lib/secret-store-guidance";
import type { SecretStoreStatusData } from "../../lib/types";
import { Alert, AlertDescription, AlertTitle } from "../ui/alert";
import { Button } from "../ui/button";

interface SecretStoreIssueAlertProps {
	status: SecretStoreStatusData;
	onRetryStatus?: () => void;
	onRetryProvider?: (mode: SwitchableSecretStoreProviderMode) => void;
	isRetrying?: boolean;
	settingsHref?: string;
	hideSettingsLink?: boolean;
	icon?: LucideIcon;
}

export function SecretStoreIssueAlert({
	status,
	onRetryStatus,
	onRetryProvider,
	isRetrying = false,
	settingsHref = "/settings?tab=security",
	hideSettingsLink = false,
	icon: Icon = ShieldAlert,
}: SecretStoreIssueAlertProps) {
	const { t } = useTranslation("secrets");
	const guidance = resolveSecretStoreIssueGuidance(status, t);
	if (!guidance) {
		return null;
	}

	const showRetryStatus =
		guidance.actions.includes("retry_status") && onRetryStatus;
	const showRetryProvider =
		guidance.actions.includes("retry_provider") &&
		guidance.retryProviderMode &&
		onRetryProvider;
	const showSettingsLink =
		!hideSettingsLink &&
		guidance.actions.includes("open_security_settings");

	return (
		<Alert variant="destructive">
			<Icon className="h-4 w-4" />
			<AlertTitle>{guidance.title}</AlertTitle>
			<AlertDescription className="space-y-3">
				<p>{guidance.description}</p>
				{showRetryStatus || showRetryProvider || showSettingsLink ? (
					<div className="flex flex-wrap gap-2">
						{showRetryProvider ? (
							<RetryButton
								disabled={isRetrying}
								spinning={isRetrying}
								onClick={() => {
									if (guidance.retryProviderMode) {
										onRetryProvider(guidance.retryProviderMode);
									}
								}}
								label={t("guidance.actions.retryProvider", {
									defaultValue: "Retry secure storage",
								})}
							/>
						) : null}
						{showRetryStatus ? (
							<RetryButton
								disabled={isRetrying}
								spinning={isRetrying}
								onClick={onRetryStatus}
								label={t("guidance.actions.retryStatus", {
									defaultValue: "Retry status check",
								})}
							/>
						) : null}
						{showSettingsLink ? (
							<Button type="button" variant="outline" size="sm" asChild>
								<Link to={settingsHref}>
									{t("guidance.actions.openSecuritySettings", {
										defaultValue: "Open Security settings",
									})}
								</Link>
							</Button>
						) : null}
					</div>
				) : null}
			</AlertDescription>
		</Alert>
	);
}

function RetryButton({
	disabled,
	spinning,
	onClick,
	label,
}: {
	disabled: boolean;
	spinning: boolean;
	onClick: () => void;
	label: string;
}) {
	return (
		<Button
			type="button"
			variant="outline"
			size="sm"
			disabled={disabled}
			onClick={onClick}
		>
			<RefreshCw className={`mr-2 h-4 w-4 ${spinning ? "animate-spin" : ""}`} />
			{label}
		</Button>
	);
}
