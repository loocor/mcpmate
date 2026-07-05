import type { InspectorCapabilityKind } from "../lib/inspector-capability";
import { parseInspectorResponsePreview } from "../lib/inspector-response-preview";
import { cn } from "../lib/utils";
import { JsonCodeBlock } from "./json-code-block";
import { LazyImage } from "./lazy-image";

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
