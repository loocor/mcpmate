import { BookCopy, FileText, Globe, Link2, Minus, PenLine, Unlink } from "lucide-react";
import type { MouseEvent } from "react";
import { useTranslation } from "react-i18next";
import { EntityCard } from "../../../components/entity-card";
import { Switch } from "../../../components/ui/switch";
import {
	Tooltip,
	TooltipContent,
	TooltipProvider,
	TooltipTrigger,
} from "../../../components/ui/tooltip";
import type { ClientInfo } from "../../../lib/types";
import {
	getClientAttentionClasses,
	getGovernanceStatus,
	type ClientGovernanceStatus,
} from "../client-governance";

type ClientAttachmentState = "attached" | "detached" | "not_applicable";

function normalizeAttachmentState(
	attachmentState?: ClientInfo["attachment_state"],
): ClientAttachmentState {
	switch (attachmentState) {
		case "attached":
		case "detached":
			return attachmentState;
		default:
			return "not_applicable";
	}
}

function getGovernanceStatusLabel(status: ClientGovernanceStatus): string {
	switch (status) {
		case "allowed":
			return "Allowed";
		case "pending":
			return "Pending";
		case "denied":
			return "Denied";
	}
}

function renderAttachmentIcon(state: ClientAttachmentState) {
	switch (state) {
		case "attached":
			return <Link2 className="h-4 w-4" aria-hidden />;
		case "detached":
			return <Unlink className="h-4 w-4" aria-hidden />;
		default:
			return <Minus className="h-4 w-4" aria-hidden />;
	}
}

function getAttachmentIconClass(state: ClientAttachmentState): string {
	switch (state) {
		case "attached":
			return "text-emerald-500";
		case "detached":
			return "text-slate-400";
		default:
			return "text-slate-300";
	}
}

interface ClientCardProps {
	client: ClientInfo;
	onNavigate: (identifier: string) => void;
	onGovernanceChange: (identifier: string, approved: boolean) => void;
	isGovernancePending: boolean;
}

export function ClientCard({
	client,
	onNavigate,
	onGovernanceChange,
	isGovernancePending,
}: ClientCardProps) {
	const { t } = useTranslation("clients");

	const displayName =
		client.display_name ||
		client.identifier ||
		t("entity.fallbackName", { defaultValue: "Client" });
	const identifier = client.identifier || "—";
	const avatarInitial =
		(displayName.trim() || identifier).charAt(0).toUpperCase() || "C";
	const description =
		client.description ?? client.template?.description ?? undefined;
	const homepageUrl =
		client.homepage_url ?? client.template?.homepage_url ?? null;
	const governanceStatus = getGovernanceStatus(client);
	const attentionClasses = getClientAttentionClasses(governanceStatus);
	const attachmentState = normalizeAttachmentState(client.attachment_state);
	let attachmentLabel: string;
	switch (attachmentState) {
		case "attached":
			attachmentLabel = t("entity.attachment.attached", { defaultValue: "Attached" });
			break;
		case "detached":
			attachmentLabel = t("entity.attachment.detached", { defaultValue: "Detached" });
			break;
		default:
			attachmentLabel = t("entity.attachment.notApplicable", {
				defaultValue: "Config N/A",
			});
	}
	const statItems = [
		{
			label: t("entity.stats.servers", { defaultValue: "Servers" }),
			value: (client.mcp_servers_count ?? 0).toString(),
		},
		{
			label: t("entity.stats.governance", { defaultValue: "Governance" }),
			value: getGovernanceStatusLabel(governanceStatus),
		},
		{
			label: t("entity.stats.detected", { defaultValue: "Detected" }),
			value: client.detected
				? t("states.yes", { defaultValue: "Yes" })
				: t("states.no", { defaultValue: "No" }),
		},
		{
			label: t("entity.stats.attachment", { defaultValue: "Attachment" }),
			value: attachmentLabel,
		},
	];

	const attachmentIcon = renderAttachmentIcon(attachmentState);
	const attachmentIconClass = getAttachmentIconClass(attachmentState);

	const yesLabel = t("states.yes", { defaultValue: "Yes" });
	const noLabel = t("states.no", { defaultValue: "No" });

	const attachmentTooltip =
		attachmentState === "not_applicable"
			? t("entity.tooltip.attachmentUnavailable", {
				defaultValue: "Attachment: Config N/A.",
			})
			: t("entity.tooltip.attachmentState", {
				status: attachmentLabel,
				defaultValue: "Attachment: {{status}}",
			});

	const defaultPolicyTooltip = t("entity.tooltip.defaultPolicy", {
		answer: client.governed_by_default_policy === true ? yesLabel : noLabel,
		defaultValue: "Default policy: {{answer}}",
	});

	const explicitRecordTooltip = t("entity.tooltip.explicitRecord", {
		answer: client.governed_by_default_policy === true ? noLabel : yesLabel,
		defaultValue: "Explicit record: {{answer}}",
	});

	const writableTooltip = t("entity.tooltip.writableConfig", {
		answer: client.writable_config === true ? yesLabel : noLabel,
		defaultValue: "Writable config: {{answer}}",
	});

	const iconButtonClass =
		"inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-md border-0 bg-transparent p-0 transition-opacity focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50";

	const quickLinks = (
		[
			{
				label: t("detail.overview.labels.homepage", {
					defaultValue: "Homepage",
				}),
				url: homepageUrl,
				icon: Globe,
			},
		] as const
	).filter((link) => !!link.url);

	const handleQuickLink = (event: MouseEvent, url: string) => {
		event.stopPropagation();
		if (!url) return;
		try {
			window.open(url, "_blank", "noopener,noreferrer");
		} catch {
			/* noop */
		}
	};

	return (
		<EntityCard
			key={`client-card-${identifier}`}
			id={identifier}
			title={displayName}
			description={description}
			avatar={{
				src: client.logo_url ?? undefined,
				alt: displayName,
				fallback: avatarInitial,
			}}
			avatarShape="rounded"
			stats={statItems}
			className={`${governanceStatus === "pending" ? "opacity-75" : ""} ${attentionClasses.cardClassName}`.trim()}
			titleClassName={attentionClasses.titleClassName}
			topRightBadge={
				quickLinks.length > 0 ? (
					<>
						{quickLinks.map((link) => (
							<button
								key={`${identifier}-${link.label}`}
								type="button"
								className="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full border border-transparent bg-transparent text-slate-400 transition hover:text-slate-600 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/50 dark:text-slate-500 dark:hover:text-slate-300"
								onClick={(event) => handleQuickLink(event, link.url!)}
								title={link.label}
							>
								<link.icon className="h-4 w-4" aria-hidden />
								<span className="sr-only">{link.label}</span>
							</button>
						))}
					</>
				) : undefined
			}
			bottomLeft={
				<TooltipProvider delayDuration={200}>
					<div className="-ml-2 flex max-w-full flex-nowrap items-center gap-2 overflow-hidden text-foreground">
						<Tooltip>
							<TooltipTrigger asChild>
								<button
									type="button"
									className={`${iconButtonClass} ${attachmentIconClass}`}
									aria-label={attachmentTooltip}
									onClick={(e) => e.stopPropagation()}
								>
									{attachmentIcon}
								</button>
							</TooltipTrigger>
							<TooltipContent side="top">{attachmentTooltip}</TooltipContent>
						</Tooltip>
						<Tooltip>
							<TooltipTrigger asChild>
								<button
									type="button"
									className={`${iconButtonClass} text-foreground ${client.governed_by_default_policy === true ? "opacity-100" : "opacity-25"}`}
									aria-label={defaultPolicyTooltip}
									onClick={(e) => e.stopPropagation()}
								>
									<BookCopy className="h-4 w-4" aria-hidden />
								</button>
							</TooltipTrigger>
							<TooltipContent side="top">{defaultPolicyTooltip}</TooltipContent>
						</Tooltip>
						<Tooltip>
							<TooltipTrigger asChild>
								<button
									type="button"
									className={`${iconButtonClass} text-foreground ${client.governed_by_default_policy === true ? "opacity-25" : "opacity-100"}`}
									aria-label={explicitRecordTooltip}
									onClick={(e) => e.stopPropagation()}
								>
									<FileText className="h-4 w-4" aria-hidden />
								</button>
							</TooltipTrigger>
							<TooltipContent side="top">{explicitRecordTooltip}</TooltipContent>
						</Tooltip>
						<Tooltip>
							<TooltipTrigger asChild>
								<button
									type="button"
									className={`${iconButtonClass} text-foreground ${client.writable_config === true ? "opacity-100" : "opacity-25"}`}
									aria-label={writableTooltip}
									onClick={(e) => e.stopPropagation()}
								>
									<PenLine className="h-4 w-4" aria-hidden />
								</button>
							</TooltipTrigger>
							<TooltipContent side="top">{writableTooltip}</TooltipContent>
						</Tooltip>
					</div>
				</TooltipProvider>
			}
			bottomRight={
				<Switch
					checked={governanceStatus === "allowed"}
					onCheckedChange={(checked) =>
						onGovernanceChange(identifier, checked)
					}
					onClick={(e) => e.stopPropagation()}
					disabled={isGovernancePending || governanceStatus === "pending"}
				/>
			}
			onClick={() => onNavigate(identifier)}
		/>
	);
}
