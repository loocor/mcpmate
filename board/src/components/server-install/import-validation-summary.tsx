import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";

import type { ImportStats } from "../../lib/api";
import {
	getSkippedQueryFieldLabel,
	getSkippedReasonLabel,
} from "../../lib/server-import-utils";
import { cn, toTitleCase } from "../../lib/utils";
import { Badge } from "../ui/badge";

export type ImportValidationStatus = "ready" | "skipped" | "failed";

export type ImportValidationEntry = {
	name: string;
	status: ImportValidationStatus;
	skipReason?: string;
	detail?: string;
	incomingQuery?: string | null;
	existingQuery?: string | null;
};

type StatusCounts = {
	ready: number;
	skipped: number;
	failed: number;
};

const BENIGN_SKIP_REASONS = new Set([
	"duplicate_fingerprint",
	"duplicate_name",
]);

const STATUS_BADGE_CLASS: Record<ImportValidationStatus, string> = {
	ready:
		"border-emerald-200 bg-emerald-50 text-emerald-700 dark:border-emerald-800/80 dark:bg-emerald-950/40 dark:text-emerald-300",
	skipped:
		"border-amber-200 bg-amber-50 text-amber-700 dark:border-amber-800/80 dark:bg-amber-950/40 dark:text-amber-300",
	failed:
		"border-red-200 bg-red-50 text-red-700 dark:border-red-800/80 dark:bg-red-950/40 dark:text-red-300",
};

function countStatuses(items: ImportValidationEntry[]): StatusCounts {
	return items.reduce<StatusCounts>(
		(counts, item) => {
			counts[item.status] += 1;
			return counts;
		},
		{ ready: 0, skipped: 0, failed: 0 },
	);
}

export function buildImportValidationItems({
	selectedNames,
	stats,
	hiddenPreviewReady,
}: {
	selectedNames: string[];
	stats: ImportStats | null;
	hiddenPreviewReady?: boolean;
}): ImportValidationEntry[] {
	if (!selectedNames.length) {
		return [];
	}

	if (hiddenPreviewReady) {
		return selectedNames.map((name) => ({ name, status: "ready" }));
	}

	if (!stats) {
		return [];
	}

	const skippedByName = new Map(
		stats.skippedDetails.map((detail) => [detail.name, detail]),
	);
	const failedSet = new Set(stats.failedServers);

	return selectedNames.map((name) => {
		if (failedSet.has(name)) {
			return {
				name,
				status: "failed",
				detail: stats.errorDetails?.[name],
			};
		}

		const skipped = skippedByName.get(name);
		if (skipped) {
			return {
				name,
				status: "skipped",
				skipReason: skipped.reason,
				incomingQuery: skipped.incoming_query,
				existingQuery: skipped.existing_query,
			};
		}

		return { name, status: "ready" };
	});
}

function resolveSummaryCopy(
	counts: StatusCounts,
	t: TFunction,
	options?: { hiddenPreviewReady?: boolean },
): { title: string; description: string } {
	if (options?.hiddenPreviewReady) {
		return {
			title: t("wizard.result.pendingImportReadyTitle", {
				defaultValue: "Ready to publish",
			}),
			description: t("wizard.result.pendingImportReadyDescription", {
				defaultValue:
					"OAuth authorization is complete. Import will publish this server and make it visible in your Servers list.",
			}),
		};
	}

	const { ready, skipped, failed } = counts;

	if (ready > 0 && skipped === 0 && failed === 0) {
		return {
			title: t("wizard.result.readyStatusTitle", {
				defaultValue: "Import Ready",
			}),
			description: t("wizard.result.readyStatusDescription", {
				defaultValue:
					"The server configuration is ready to be imported. Review the information below and click Import when ready.",
			}),
		};
	}

	if (skipped > 0 && ready === 0 && failed === 0) {
		return {
			title: t("wizard.result.alreadyInstalledTitle", {
				defaultValue: "Already Installed",
			}),
			description: t("wizard.result.alreadyInstalledDescription", {
				defaultValue:
					"Every selected server already exists. You can use it immediately—no import required.",
			}),
		};
	}

	if (failed > 0 && ready === 0 && skipped === 0) {
		return {
			title: t("wizard.result.validationFailedTitle", {
				defaultValue: "Import Validation Failed",
			}),
			description: t("wizard.result.validationFailedDescription", {
				defaultValue:
					"Resolve the blocking issues below and run validation again.",
			}),
		};
	}

	if (ready > 0 && skipped > 0 && failed === 0) {
		return {
			title: t("wizard.result.validatedWithWarningsTitle", {
				defaultValue: "Import Validated With Warnings",
			}),
			description: t("wizard.result.summary.descriptionReadySkipped", {
				ready,
				skipped,
				defaultValue:
					"{{ready}} will import and {{skipped}} are already installed and will be skipped.",
			}),
		};
	}

	if (ready > 0 && failed > 0 && skipped === 0) {
		return {
			title: t("wizard.result.summary.titleReadyFailed", {
				defaultValue: "Import Partially Ready",
			}),
			description: t("wizard.result.summary.descriptionReadyFailed", {
				ready,
				failed,
				defaultValue:
					"{{ready}} will import. Resolve validation failures for the remaining {{failed}} before importing.",
			}),
		};
	}

	if (skipped > 0 && failed > 0 && ready === 0) {
		return {
			title: t("wizard.result.validationFailedTitle", {
				defaultValue: "Import Validation Failed",
			}),
			description: t("wizard.result.summary.descriptionSkippedFailed", {
				skipped,
				failed,
				defaultValue:
					"{{skipped}} are already installed and {{failed}} failed validation.",
			}),
		};
	}

	return {
		title: t("wizard.result.summary.titleMixed", {
			defaultValue: "Import Review",
		}),
		description: t("wizard.result.summary.descriptionReadySkippedFailed", {
			ready,
			skipped,
			failed,
			defaultValue:
				"{{ready}} will import, {{skipped}} will be skipped, and {{failed}} failed validation.",
		}),
	};
}

function getEntryDetailText(
	entry: ImportValidationEntry,
	t: TFunction,
): string | undefined {
	if (entry.status === "failed") {
		return entry.detail;
	}
	if (entry.status === "skipped" && entry.skipReason) {
		return getSkippedReasonLabel(entry.skipReason, t);
	}
	return undefined;
}

function shouldShowItemDetail(entry: ImportValidationEntry): boolean {
	if (entry.status === "failed") {
		return Boolean(entry.detail);
	}
	if (entry.status !== "skipped") {
		return false;
	}
	if (!entry.skipReason || BENIGN_SKIP_REASONS.has(entry.skipReason)) {
		return Boolean(entry.incomingQuery || entry.existingQuery);
	}
	return true;
}

type ImportValidationSummaryProps = {
	items: ImportValidationEntry[];
	hiddenPreviewReady?: boolean;
	className?: string;
};

export function ImportValidationSummary({
	items,
	hiddenPreviewReady = false,
	className,
}: ImportValidationSummaryProps) {
	const { t } = useTranslation("servers");

	if (!items.length) {
		return null;
	}

	const counts = countStatuses(items);
	const { title, description } = resolveSummaryCopy(counts, t, {
		hiddenPreviewReady,
	});

	const statusBadgeLabel: Record<ImportValidationStatus, string> = {
		ready: t("wizard.result.summary.badgeReady", {
			defaultValue: "Ready",
		}),
		skipped: t("wizard.result.summary.badgeSkipped", {
			defaultValue: "Skipped",
		}),
		failed: t("wizard.result.summary.badgeFailed", {
			defaultValue: "Failed",
		}),
	};

	return (
		<div
			className={cn(
				"rounded-md border border-slate-200/80 bg-slate-50/80 px-3 py-3 dark:border-slate-700/80 dark:bg-slate-900/40",
				className,
			)}
		>
			<p className="text-sm font-medium text-foreground">{title}</p>
			<p className="mt-1 text-sm text-muted-foreground">{description}</p>
			<ul className="mt-3 divide-y divide-slate-200/80 dark:divide-slate-700/80">
				{items.map((entry) => {
					const queryParts: string[] = [];
					if (entry.incomingQuery) {
						queryParts.push(
							`${getSkippedQueryFieldLabel("incoming", t)}=${entry.incomingQuery}`,
						);
					}
					if (entry.existingQuery) {
						queryParts.push(
							`${getSkippedQueryFieldLabel("existing", t)}=${entry.existingQuery}`,
						);
					}

					const detailText = getEntryDetailText(entry, t);
					const showDetail = shouldShowItemDetail(entry);

					return (
						<li
							key={entry.name}
							className="flex items-start justify-between gap-3 py-1.5 first:pt-0 last:pb-0"
						>
							<div className="min-w-0">
								<p className="text-sm font-medium text-foreground">
									{toTitleCase(entry.name)}
								</p>
								{showDetail && detailText ? (
									<p className="mt-0.5 text-xs text-muted-foreground">
										{detailText}
									</p>
								) : null}
								{queryParts.length > 0 ? (
									<p className="mt-0.5 text-xs text-muted-foreground">
										{queryParts.join(" · ")}
									</p>
								) : null}
							</div>
							<Badge
								variant="outline"
								className={cn(
									"shrink-0 font-medium",
									STATUS_BADGE_CLASS[entry.status],
								)}
							>
								{statusBadgeLabel[entry.status]}
							</Badge>
						</li>
					);
				})}
			</ul>
		</div>
	);
}
