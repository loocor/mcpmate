import type { InspectorCapabilityKind } from "../lib/inspector-capability";
import {
	buildInspectorJsonOutline,
	parseInspectorResponsePreview,
	type InspectorJsonOutlineRow,
} from "../lib/inspector-response-preview";
import { cn } from "../lib/utils";
import { JsonCodeBlock } from "./json-code-block";
import { LazyImage } from "./lazy-image";

const JSON_OUTLINE_TYPE_CLASSNAMES: Record<InspectorJsonOutlineRow["type"], string> = {
	array: "border-blue-200 bg-blue-50 text-blue-700 dark:border-blue-900/70 dark:bg-blue-950/40 dark:text-blue-300",
	boolean: "border-amber-200 bg-amber-50 text-amber-700 dark:border-amber-900/70 dark:bg-amber-950/40 dark:text-amber-300",
	null: "border-slate-200 bg-slate-50 text-slate-500 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-400",
	number: "border-emerald-200 bg-emerald-50 text-emerald-700 dark:border-emerald-900/70 dark:bg-emerald-950/40 dark:text-emerald-300",
	object: "border-indigo-200 bg-indigo-50 text-indigo-700 dark:border-indigo-900/70 dark:bg-indigo-950/40 dark:text-indigo-300",
	string: "border-rose-200 bg-rose-50 text-rose-700 dark:border-rose-900/70 dark:bg-rose-950/40 dark:text-rose-300",
	truncated: "border-slate-200 bg-slate-50 text-slate-500 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-400",
	undefined: "border-slate-200 bg-slate-50 text-slate-500 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-400",
	unknown: "border-slate-200 bg-slate-50 text-slate-500 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-400",
};

export function InspectorJsonOutline({
	value,
	className,
}: {
	value: unknown;
	className?: string;
}) {
	const rows = buildInspectorJsonOutline(value);

	return (
		<div
			className={cn(
				"min-w-0 overflow-auto rounded-md border border-slate-200 bg-white text-xs dark:border-slate-800 dark:bg-slate-950",
				className,
			)}
		>
			<div className="min-w-max divide-y divide-slate-100 dark:divide-slate-900">
				{rows.map((row) => (
					<div
						key={row.id}
						className="grid grid-cols-[minmax(12rem,1fr)_auto_minmax(8rem,0.8fr)] items-center gap-3 px-3 py-2"
						style={{ paddingLeft: `${12 + row.depth * 18}px` }}
					>
						<span className="font-mono font-medium text-slate-800 dark:text-slate-200">
							{row.label}
						</span>
						<span
							className={cn(
								"rounded border px-1.5 py-0.5 font-mono text-[10px] uppercase leading-none",
								JSON_OUTLINE_TYPE_CLASSNAMES[row.type],
							)}
						>
							{row.type}
						</span>
						<span className="truncate font-mono text-slate-500 dark:text-slate-400">
							{row.summary}
						</span>
					</div>
				))}
			</div>
		</div>
	);
}

export function InspectorResponsePreview({
	result,
	kind,
	className,
}: {
	result: unknown;
	kind: InspectorCapabilityKind;
	className?: string;
}) {
	const blocks = parseInspectorResponsePreview(result, kind);

	if (blocks.length === 0) {
		return (
			<div className={cn("text-sm text-muted-foreground", className)}>
				No previewable MCP content.
			</div>
		);
	}

	return (
		<div className={cn("min-w-0 space-y-3 overflow-y-auto", className)}>
			{blocks.map((block, index) => {
				if (block.kind === "image") {
					return (
						<div
							key={`image-${index}`}
							className="rounded-md bg-slate-50 p-2 dark:bg-slate-900"
						>
							<LazyImage
								src={block.src}
								alt={block.alt ?? ""}
								cacheKey={block.src.slice(0, 128)}
								className="block max-w-full"
								imgClassName="block h-auto max-w-full rounded-md border border-slate-200 object-contain dark:border-slate-800"
								fallback={
									<div className="rounded-md border border-dashed border-slate-300 px-3 py-6 text-center text-xs text-muted-foreground dark:border-slate-700">
										{block.mimeType ?? "image"}
									</div>
								}
							/>
						</div>
					);
				}

				if (block.format === "markdown") {
					return (
						<JsonCodeBlock
							key={`markdown-${index}`}
							code={block.text}
							language="markdown"
						/>
					);
				}

				return (
					<pre
						key={`text-${index}`}
						className="m-0 whitespace-pre-wrap break-words rounded-md bg-slate-50 p-3 text-sm text-slate-700 dark:bg-slate-900 dark:text-slate-200"
					>
						{block.text}
					</pre>
				);
			})}
		</div>
	);
}
