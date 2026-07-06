import { Copy } from "lucide-react";
import { writeClipboardText } from "../../lib/clipboard";
import { JsonEditableCodeBlock } from "../json-code-block";
import { Button } from "../ui/button";
import { Label } from "../ui/label";
import { cn } from "../../lib/utils";

interface ServerConfigJsonPanelProps {
	id: string;
	label: string;
	jsonText: string;
	jsonError: string | null;
	jsonEditingEnabled: boolean;
	className?: string;
	onJsonChange?: (text: string) => void;
	copyLabel?: string;
}

export function ServerConfigJsonPanel({
	id,
	label,
	jsonText,
	jsonError,
	jsonEditingEnabled,
	className,
	onJsonChange,
	copyLabel = "Copy JSON",
}: ServerConfigJsonPanelProps) {
	return (
		<div className={cn("flex min-h-0 flex-1 flex-col", className)}>
			<div className="flex min-h-0 flex-1 items-stretch gap-4">
				<Label htmlFor={id} className="w-20 shrink-0 pt-3 text-right">
					{label}
				</Label>
				<div className="flex min-h-0 flex-1 flex-col">
					<div className="group relative flex min-h-0 flex-1 flex-col overflow-hidden rounded-md border border-input bg-slate-50 dark:bg-slate-900">
						{jsonText ? (
							<div className="pointer-events-none absolute top-0 right-0 z-10 flex w-full justify-end p-2">
								<Button
									type="button"
									variant="outline"
									size="sm"
									className="pointer-events-auto h-7 w-7 bg-white/95 p-0 opacity-0 shadow-sm backdrop-blur-sm transition-opacity group-hover:opacity-100 dark:bg-slate-900/95"
									onClick={async (event) => {
										event.stopPropagation();
										await writeClipboardText(jsonText);
									}}
									title={copyLabel}
								>
									<Copy className="h-3.5 w-3.5" />
								</Button>
							</div>
						) : null}
						<JsonEditableCodeBlock
							id={id}
							code={jsonText}
							readOnly={!jsonEditingEnabled}
							onCodeChange={jsonEditingEnabled ? onJsonChange : undefined}
							aria-label={label}
						/>
					</div>
					{jsonError ? (
						<p className="mt-2 shrink-0 text-xs text-red-500">{jsonError}</p>
					) : null}
				</div>
			</div>
		</div>
	);
}
