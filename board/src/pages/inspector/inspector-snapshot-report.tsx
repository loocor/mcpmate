import { AlertTriangle, CheckCircle2, HelpCircle, Info, ShieldCheck } from "lucide-react";
import type { ReactNode } from "react";
import { CardListScrollBody } from "../../components/card-list-scroll-body";
import { Badge, type BadgeProps } from "../../components/ui/badge";
import { cn } from "../../lib/utils";

type InspectorSnapshotReportProps = {
	payload: Record<string, unknown> | null;
	loadedAt?: string;
};

function isRecord(value: unknown): value is Record<string, unknown> {
	return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function asRecord(value: unknown): Record<string, unknown> | null {
	return isRecord(value) ? value : null;
}

function asArray(value: unknown): Record<string, unknown>[] {
	return Array.isArray(value) ? value.filter(isRecord) : [];
}

function stringValue(value: unknown): string | null {
	return typeof value === "string" && value.trim().length > 0 ? value : null;
}

function numberValue(value: unknown): number | null {
	return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function displayValue(value: unknown): string {
	if (value == null) return "n/a";
	if (typeof value === "string") return value;
	if (typeof value === "number" || typeof value === "boolean") return String(value);
	if (Array.isArray(value) || typeof value === "object") {
		return JSON.stringify(value, null, 2);
	}
	return String(value);
}

function statusBadgeVariant(status: string | null): BadgeProps["variant"] {
	switch (status) {
		case "implemented":
		case "completed":
		case "info":
			return "success";
		case "partial":
		case "medium":
			return "warning";
		case "low":
			return "outline";
		case "not_advertised":
		case "unknown":
			return "outline";
		case "high":
			return "destructive";
		default:
			return "outline";
	}
}

function compatibilityStatusPriority(status: string | null): number {
	switch (status) {
		case "partial":
			return 0;
		case "not_advertised":
			return 1;
		case "unknown":
			return 2;
		case "implemented":
			return 3;
		default:
			return 4;
	}
}

function compatibilityRowTone(status: string | null): string {
	switch (status) {
		case "implemented":
			return "border-emerald-500/30 bg-emerald-500/[0.04]";
		case "partial":
			return "border-amber-500/50 bg-amber-500/[0.07]";
		case "not_advertised":
			return "border-destructive/40 bg-destructive/[0.05]";
		case "unknown":
			return "border-slate-400/40 bg-slate-500/[0.05]";
		default:
			return "border-border bg-card/40";
	}
}

function compatibilityDiffTone(status: string | null): string {
	switch (status) {
		case "implemented":
			return "border-emerald-500/20 bg-emerald-500/[0.03]";
		case "partial":
			return "border-amber-500/40 bg-amber-500/[0.08]";
		case "not_advertised":
			return "border-destructive/30 bg-destructive/[0.06]";
		case "unknown":
			return "border-slate-400/30 bg-slate-500/[0.06]";
		default:
			return "border-border bg-background/60";
	}
}

function compatibilityStatusLabel(status: string | null): string {
	switch (status) {
		case "not_advertised":
			return "missing";
		case "implemented":
		case "partial":
		case "unknown":
			return status;
		default:
			return status ?? "unknown";
	}
}

function requiredLabel(required: unknown): string {
	return required === false ? "optional" : "yes";
}

function coverageBarClassName(status: string | null): string {
	switch (status) {
		case "implemented":
			return "bg-emerald-500";
		case "partial":
			return "bg-amber-500";
		default:
			return "bg-muted-foreground/50";
	}
}

function differenceSummary(status: string | null): string {
	switch (status) {
		case "implemented":
			return "No visible gap in this snapshot.";
		case "partial":
			return "Partially implemented; inspect the observed coverage.";
		case "not_advertised":
			return "No advertised evidence was found.";
		default:
			return "Current evidence cannot prove support.";
	}
}

function observationRatio(
	observed: Record<string, unknown> | null,
): { count: number; total: number } | null {
	const count = numberValue(observed?.count);
	const total = numberValue(observed?.total);
	if (count == null || total == null || total <= 0) return null;
	return { count, total };
}

function ObservationMeter({
	observed,
	status,
}: {
	observed: Record<string, unknown> | null;
	status: string | null;
}): ReactNode {
	const ratio = observationRatio(observed);
	if (!ratio) return null;
	const pct = Math.max(0, Math.min(100, Math.round((ratio.count / ratio.total) * 100)));
	return (
		<div className="mt-3 space-y-1.5">
			<div className="flex items-center justify-between text-[11px] text-muted-foreground">
				<span>Observed coverage</span>
				<span className="font-mono">
					{ratio.count}/{ratio.total}
				</span>
			</div>
			<div className="h-1.5 overflow-hidden rounded-full bg-muted">
				<div
					className={cn(
						"h-full rounded-full",
						coverageBarClassName(status),
					)}
					style={{ width: `${pct}%` }}
				/>
			</div>
		</div>
	);
}

function InventoryPanel({
	inventory,
	className,
}: {
	inventory: Record<string, unknown> | null;
	className?: string;
}): ReactNode {
	const entries = Object.entries(inventory ?? {});
	return (
		<div className={cn("rounded-md border border-border bg-card/40 p-4", className)}>
			<p className="text-sm font-semibold text-foreground">Inventory</p>
			{entries.length > 0 ? (
				<dl className="mt-3 grid gap-2 text-xs">
					{entries.map(([key, value]) => (
						<div key={key} className="grid grid-cols-[minmax(5rem,0.7fr)_minmax(0,1fr)] gap-2">
							<dt className="text-muted-foreground">{key.replaceAll("_", " ")}</dt>
							<dd className="whitespace-pre-wrap break-words font-mono text-foreground">
								{displayValue(value)}
							</dd>
						</div>
					))}
				</dl>
			) : (
				<p className="mt-2 text-sm text-muted-foreground">No inventory facts.</p>
			)}
		</div>
	);
}

function CompatibilityRequirementRow({
	requirement,
	index,
}: {
	requirement: Record<string, unknown>;
	index: number;
}): ReactNode {
	const status = stringValue(requirement.status);
	const expected = asRecord(requirement.expected);
	const observedRequirement = asRecord(requirement.observed);
	const diff = asRecord(requirement.diff);
	const title = stringValue(requirement.title) ?? "Requirement";
	const category = stringValue(requirement.category) ?? "General";
	const requirementId = stringValue(requirement.id);
	const expectedText =
		stringValue(diff?.left) ??
		stringValue(expected?.description) ??
		"No requirement description.";
	const observedText =
		stringValue(observedRequirement?.detail) ??
		stringValue(diff?.right) ??
		"No observed detail.";
	const hasActionableDifference =
		status === "partial" || status === "not_advertised" || status === "unknown";

	return (
		<div
			className={cn(
				"rounded-md border-l-4 border-y border-r p-4",
				compatibilityRowTone(status),
			)}
		>
			<div className="grid gap-4 xl:grid-cols-[minmax(12rem,0.85fr)_minmax(0,1fr)_minmax(0,1fr)_minmax(8rem,0.45fr)]">
				<div className="min-w-0">
					<div className="flex items-start gap-2">
						{statusIcon(status)}
						<div className="min-w-0">
							<p className="font-medium leading-snug text-foreground">{title}</p>
							<p className="mt-1 text-xs text-muted-foreground">
								{requirementId ?? `requirement-${index}`}
							</p>
						</div>
					</div>
					<div className="mt-3 flex flex-wrap gap-2">
						<Badge variant={statusBadgeVariant(status)}>
							{compatibilityStatusLabel(status)}
						</Badge>
						<Badge variant="outline">{category}</Badge>
					</div>
				</div>

				<div className="rounded-md border border-border bg-background/70 p-3">
					<p className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
						{stringValue(diff?.left_label) ?? "Spec requirement"}
					</p>
					<p className="mt-2 text-sm leading-relaxed text-foreground">
						{expectedText}
					</p>
					<p className="mt-3 text-xs text-muted-foreground">
						Required: {requiredLabel(expected?.required)} · Version{" "}
						{stringValue(expected?.version) ?? "unknown"}
					</p>
				</div>

				<div
					className={cn(
						"rounded-md border p-3",
						compatibilityDiffTone(status),
					)}
				>
					<div className="flex items-center justify-between gap-2">
						<p className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
							{stringValue(diff?.right_label) ?? "Observed server"}
						</p>
						{hasActionableDifference ? (
							<span className="rounded-sm bg-background/70 px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-foreground">
								Review
							</span>
						) : null}
					</div>
					<p className="mt-2 text-sm leading-relaxed text-foreground">
						{observedText}
					</p>
					<ObservationMeter observed={observedRequirement} status={status} />
				</div>

				<div className="rounded-md border border-border bg-background/60 p-3">
					<p className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
						Difference
					</p>
					<p className="mt-2 text-sm leading-relaxed text-foreground">
						{differenceSummary(status)}
					</p>
				</div>
			</div>
		</div>
	);
}

function statusIcon(status: string | null) {
	switch (status) {
		case "implemented":
		case "completed":
			return <CheckCircle2 className="h-4 w-4 text-emerald-500" />;
		case "partial":
		case "medium":
		case "high":
			return <AlertTriangle className="h-4 w-4 text-amber-500" />;
		case "unknown":
			return <HelpCircle className="h-4 w-4 text-muted-foreground" />;
		default:
			return <Info className="h-4 w-4 text-muted-foreground" />;
	}
}

function SummaryMetric({
	label,
	value,
	tone,
}: {
	label: string;
	value: unknown;
	tone?: "default" | "good" | "warn" | "bad";
}) {
	return (
		<div
			className={cn(
				"rounded-md border border-border bg-background/60 px-3 py-2",
				tone === "good" && "border-emerald-500/30 bg-emerald-500/5",
				tone === "warn" && "border-amber-500/30 bg-amber-500/5",
				tone === "bad" && "border-destructive/30 bg-destructive/5",
			)}
		>
			<p className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
				{label}
			</p>
			<p className="mt-1 text-lg font-semibold text-foreground">{String(value ?? 0)}</p>
		</div>
	);
}

function RawSnapshot({ payload }: { payload: Record<string, unknown> }) {
	return (
		<div className="flex min-h-0 flex-col gap-2">
			<p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
				Raw snapshot
			</p>
			<CardListScrollBody className="min-h-[18rem]">
				<pre className="p-3 font-mono text-xs leading-relaxed text-muted-foreground">
					{JSON.stringify(payload, null, 2)}
				</pre>
			</CardListScrollBody>
		</div>
	);
}

export function InspectorCompatibilitySnapshotReport({
	payload,
	loadedAt,
}: InspectorSnapshotReportProps) {
	if (!payload) {
		return (
			<div className="flex min-h-0 flex-1 items-center justify-center rounded-md border border-dashed border-border bg-card/20 p-6 text-sm text-muted-foreground">
				Run comparison to render requirement status, observed counts, and raw evidence.
			</div>
		);
	}

	const spec = asRecord(payload.spec);
	const observed = asRecord(payload.observed);
	const counts = asRecord(observed?.counts);
	const summary = asRecord(payload.summary);
	const requirements = asArray(payload.requirements);
	const target = asRecord(payload.target);
	const sortedRequirements = [...requirements].sort((left, right) => {
		const leftPriority = compatibilityStatusPriority(stringValue(left.status));
		const rightPriority = compatibilityStatusPriority(stringValue(right.status));
		if (leftPriority !== rightPriority) return leftPriority - rightPriority;
		return (stringValue(left.title) ?? "").localeCompare(stringValue(right.title) ?? "");
	});
	const attentionCount = sortedRequirements.filter((requirement) => {
		const status = stringValue(requirement.status);
		return status === "partial" || status === "not_advertised" || status === "unknown";
	}).length;

	return (
		<div className="grid min-h-0 flex-1 gap-4 xl:grid-cols-[minmax(0,1.15fr)_minmax(24rem,0.85fr)]">
			<div className="flex min-h-0 flex-col gap-4">
				<div className="grid gap-3 md:grid-cols-5">
					<SummaryMetric label="Implemented" value={summary?.implemented} tone="good" />
					<SummaryMetric label="Partial" value={summary?.partial} tone="warn" />
					<SummaryMetric label="Missing" value={summary?.not_advertised} />
					<SummaryMetric label="Unknown" value={summary?.unknown} />
					<SummaryMetric label="Total" value={summary?.total} />
				</div>

				<div className="grid gap-3 md:grid-cols-2">
					<div className="rounded-md border border-border bg-card/40 p-4">
						<p className="text-sm font-semibold text-foreground">Spec baseline</p>
						<p className="mt-2 text-sm text-muted-foreground">
							Selected {stringValue(spec?.selected_version) ?? "unknown"} · Current{" "}
							{stringValue(spec?.current_version) ?? "unknown"} · Best fit{" "}
							{stringValue(payload.inferred_best_fit_version) ?? "unknown"}
						</p>
						{loadedAt ? (
							<p className="mt-2 text-xs text-muted-foreground">Loaded at {loadedAt}</p>
						) : null}
					</div>
					<div className="rounded-md border border-border bg-card/40 p-4">
						<p className="text-sm font-semibold text-foreground">Observed surfaces</p>
						<p className="mt-2 text-sm text-muted-foreground">
							Tools {numberValue(counts?.tools) ?? 0} · Prompts{" "}
							{numberValue(counts?.prompts) ?? 0} · Resources{" "}
							{numberValue(counts?.resources) ?? 0} · Templates{" "}
							{numberValue(counts?.resource_templates) ?? 0}
						</p>
						<p className="mt-2 break-all text-xs text-muted-foreground">
							Target: {stringValue(target?.name) ?? stringValue(target?.server_id) ?? "unknown"}
						</p>
					</div>
				</div>

				<CardListScrollBody className="min-h-[22rem]">
					{sortedRequirements.length > 0 ? (
						<div className="space-y-3 p-3">
							<div className="flex flex-wrap items-center justify-between gap-3 rounded-md border border-border bg-background/60 px-3 py-2">
								<p className="text-sm font-semibold text-foreground">
									Requirement comparison matrix
								</p>
								<p className="text-xs text-muted-foreground">
									{attentionCount} item(s) need review; rows are sorted by actionability.
								</p>
							</div>
							{sortedRequirements.map((requirement, index) => {
								const requirementKey =
									stringValue(requirement.id) ??
									stringValue(requirement.title) ??
									`requirement-${index}`;
								return (
									<CompatibilityRequirementRow
										key={requirementKey}
										requirement={requirement}
										index={index}
									/>
								);
							})}
						</div>
					) : (
						<div className="flex min-h-[18rem] items-center justify-center p-6 text-center text-sm text-muted-foreground">
							No compatibility requirements were returned for this snapshot.
						</div>
					)}
				</CardListScrollBody>
			</div>
			<RawSnapshot payload={payload} />
		</div>
	);
}

export function InspectorPackageSafetySnapshotReport({
	payload,
	loadedAt,
}: InspectorSnapshotReportProps) {
	if (!payload) {
		return (
			<div className="flex min-h-0 flex-1 items-center justify-center rounded-md border border-dashed border-border bg-card/20 p-6 text-sm text-muted-foreground">
				Start scan to render local-rule findings, recommendations, and raw evidence.
			</div>
		);
	}

	const input = asRecord(payload.input);
	const scanner = asRecord(payload.scanner);
	const inventory = asRecord(payload.inventory);
	const summary = asRecord(payload.summary);
	const findings = asArray(payload.findings);
	const recommendations = asArray(payload.recommendations);

	return (
		<div className="grid min-h-0 flex-1 gap-4 xl:grid-cols-[minmax(0,1.15fr)_minmax(24rem,0.85fr)]">
			<div className="flex min-h-0 flex-col gap-4">
				<div className="grid gap-3 md:grid-cols-5">
					<SummaryMetric label="High" value={summary?.high} tone="bad" />
					<SummaryMetric label="Medium" value={summary?.medium} tone="warn" />
					<SummaryMetric label="Low" value={summary?.low} />
					<SummaryMetric label="Info" value={summary?.info} />
					<SummaryMetric label="Total" value={summary?.total} />
				</div>

				<div className="grid gap-3 lg:grid-cols-2 2xl:grid-cols-3">
					<div className="rounded-md border border-border bg-card/40 p-4">
						<p className="text-sm font-semibold text-foreground">Scanner</p>
						<p className="mt-2 text-sm text-muted-foreground">
							{stringValue(scanner?.provider) ?? "unknown"} ·{" "}
							{stringValue(scanner?.status) ?? "unknown"}
						</p>
					</div>
					<div className="rounded-md border border-border bg-card/40 p-4">
						<p className="text-sm font-semibold text-foreground">Input</p>
						<p className="mt-2 text-sm text-muted-foreground">
							{stringValue(input?.source) ?? "unknown"} ·{" "}
							{stringValue(input?.scan_depth) ?? "unknown"}
						</p>
						{loadedAt ? (
							<p className="mt-2 text-xs text-muted-foreground">Loaded at {loadedAt}</p>
						) : null}
					</div>
					<InventoryPanel
						inventory={inventory}
						className="lg:col-span-2 2xl:col-span-1"
					/>
				</div>

				<CardListScrollBody className="min-h-[20rem]">
					{findings.length > 0 ? (
						<div className="divide-y divide-border">
							{findings.map((finding, index) => {
								const severity = stringValue(finding.severity);
								const findingKey =
									stringValue(finding.id) ??
									stringValue(finding.title) ??
									`finding-${index}`;
								return (
									<div key={findingKey} className="p-4">
										<div className="flex items-start gap-3">
											{statusIcon(severity)}
											<div className="min-w-0 flex-1">
												<div className="flex flex-wrap items-center gap-2">
													<p className="font-medium text-foreground">
														{stringValue(finding.title) ?? "Finding"}
													</p>
													<Badge variant={statusBadgeVariant(severity)}>
														{severity ?? "info"}
													</Badge>
												</div>
												<p className="mt-2 text-sm leading-relaxed text-muted-foreground">
													{stringValue(finding.detail) ?? "No finding detail."}
												</p>
												<p className="mt-2 text-xs leading-relaxed text-muted-foreground">
													{stringValue(finding.recommendation) ?? "No recommendation."}
												</p>
											</div>
										</div>
									</div>
								);
							})}
						</div>
					) : (
						<div className="flex min-h-[16rem] items-center justify-center p-6 text-center text-sm text-muted-foreground">
							No local-rule findings were returned for this scan.
						</div>
					)}
				</CardListScrollBody>

				{recommendations.length > 0 ? (
					<div className="rounded-md border border-border bg-card/40 p-4">
						<div className="mb-3 flex items-center gap-2">
							<ShieldCheck className="h-4 w-4 text-muted-foreground" />
							<p className="text-sm font-semibold text-foreground">Recommendations</p>
						</div>
						<ul className="space-y-2 text-sm text-muted-foreground">
							{recommendations.map((recommendation, index) => {
								const recommendationKey =
									stringValue(recommendation.id) ??
									stringValue(recommendation.message) ??
									`recommendation-${index}`;
								return (
									<li key={recommendationKey}>
										{stringValue(recommendation.message) ?? "Review this finding."}
									</li>
								);
							})}
						</ul>
					</div>
				) : null}
			</div>
			<RawSnapshot payload={payload} />
		</div>
	);
}
