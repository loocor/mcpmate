import { ExternalLink } from "lucide-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
	CapsuleStripeList,
	CapsuleStripeListItem,
} from "../../components/capsule-stripe-list";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "../../components/ui/card";
import { ScrollArea } from "../../components/ui/scroll-area";
import type {
	OpenSourceDocument,
	OpenSourcePackage,
	OpenSourceSection,
} from "../../types/open-source";

interface AboutLicensesSectionProps {
	document: OpenSourceDocument;
}

function formatGeneratedAt(value?: string) {
	if (!value) {
		return null;
	}

	const parsed = new Date(value);
	if (Number.isNaN(parsed.getTime())) {
		return value;
	}

	return new Intl.DateTimeFormat(undefined, {
		year: "numeric",
		month: "short",
		day: "2-digit",
		hour: "2-digit",
		minute: "2-digit",
	}).format(parsed);
}

function PackageRow({ pkg }: { pkg: OpenSourcePackage }) {
	const { t } = useTranslation();
	const repositoryLink = pkg.repository ?? pkg.homepage ?? "";
	const linkLabel = pkg.repository
		? t("settings:about.repository", { defaultValue: "Repository" })
		: pkg.homepage
			? t("settings:about.homepage", { defaultValue: "Homepage" })
			: t("settings:about.repository", { defaultValue: "Repository" });
	const licenseLabel = pkg.license ?? "UNKNOWN";
	return (
		<CapsuleStripeListItem className="items-center py-1.5">
			<div className="min-w-0">
				<p className="text-sm font-medium text-slate-900 dark:text-slate-100">
					{pkg.name}
					{pkg.version && (
						<span className="ml-1 text-[10px] font-normal text-slate-500 dark:text-slate-400">
							v{pkg.version}
						</span>
					)}
				</p>
				{pkg.description && (
					<p className="mt-1 line-clamp-1 text-xs text-slate-500 dark:text-slate-400">
						{pkg.description}
					</p>
				)}
			</div>
			<div className="flex min-w-[120px] max-w-[200px] flex-col items-end justify-center gap-1 text-right text-[11px] leading-tight text-slate-600 dark:text-slate-400">
				<span className="block w-full truncate font-semibold uppercase tracking-wide text-emerald-600 dark:text-emerald-400">
					{licenseLabel}
				</span>
				{repositoryLink && (
					<a
						href={repositoryLink}
						target="_blank"
						rel="noreferrer"
						className="inline-flex items-center gap-1 whitespace-nowrap font-medium text-emerald-600 hover:underline dark:text-emerald-400"
					>
						{linkLabel}
						<ExternalLink className="h-3 w-3" aria-hidden />
					</a>
				)}
			</div>
		</CapsuleStripeListItem>
	);
}

function Section({ section }: { section: OpenSourceSection }) {
	const { t } = useTranslation();

	return (
		<section className="space-y-3">
			<header>
				<h3 className="text-base font-semibold text-slate-900 dark:text-slate-100">
					{section.label}
				</h3>
				<p className="text-xs text-slate-500 dark:text-slate-400">
					{t("settings:about.components", {
						defaultValue: "{{count}} components",
						count: section.packages.length,
					})}
				</p>
			</header>
			<CapsuleStripeList>
				{section.packages.map((pkg) => (
					<PackageRow
						key={`${section.id}-${pkg.name}-${pkg.version ?? "unknown"}`}
						pkg={pkg}
					/>
				))}
			</CapsuleStripeList>
		</section>
	);
}

export function AboutLicensesSection({ document }: AboutLicensesSectionProps) {
	const { t } = useTranslation();

	const generatedAtDisplay = useMemo(
		() => formatGeneratedAt(document.generatedAt),
		[document.generatedAt],
	);

	const nonEmptySections = useMemo(
		() => document.sections.filter((section) => section.packages.length > 0),
		[document.sections],
	);

	return (
		<Card className="flex h-full flex-col">
			<CardHeader className="space-y-0">
				<CardTitle>
					{t("settings:about.title", { defaultValue: "About MCPMate" })}{" "}
					<sup className="text-sm font-normal text-slate-500 dark:text-slate-400">
						{t("wip", { defaultValue: "WIP" })}
					</sup>
				</CardTitle>
				<CardDescription>
					{t("settings:about.description", {
						defaultValue:
							"Open-source acknowledgements for the MCPMate preview build.",
					})}
				</CardDescription>
				{generatedAtDisplay && (
					<p className="text-xs text-slate-500 dark:text-slate-400">
						{t("settings:about.lastUpdated", {
							defaultValue: "Last updated: {{date}}",
							date: generatedAtDisplay,
						})}
					</p>
				)}
			</CardHeader>
			<CardContent className="flex-1 p-4 pt-0">
				{nonEmptySections.length === 0 ? (
					<p className="text-sm text-slate-500 dark:text-slate-400">
						{t("settings:about.noPackages", {
							defaultValue:
								"No third-party packages detected during the latest update.",
						})}
					</p>
				) : (
					<div className="border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-900/50 h-[500px] overflow-hidden">
						<ScrollArea className="h-full p-4">
							<div className="space-y-4">
								{nonEmptySections.map((section) => (
									<Section key={section.id} section={section} />
								))}
							</div>
						</ScrollArea>
					</div>
				)}
			</CardContent>
		</Card>
	);
}
