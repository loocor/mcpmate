import { Copy } from "lucide-react";
import { writeClipboardText } from "../../lib/clipboard";
import { Button } from "../ui/button";
import { Label } from "../ui/label";
import { Textarea } from "../ui/textarea";

interface ServerConfigJsonPanelProps {
	id: string;
	label: string;
	jsonText: string;
	jsonError: string | null;
	jsonEditingEnabled: boolean;
	onJsonChange?: (text: string) => void;
	copyLabel?: string;
}

export function ServerConfigJsonPanel({
	id,
	label,
	jsonText,
	jsonError,
	jsonEditingEnabled,
	onJsonChange,
	copyLabel = "Copy JSON",
}: ServerConfigJsonPanelProps) {
	return (
		<div className="flex min-h-0 flex-1 flex-col">
			<div className="flex min-h-0 flex-1 items-stretch gap-4">
				<Label htmlFor={id} className="w-20 shrink-0 pt-3 text-right">
					{label}
				</Label>
				<div className="flex min-h-0 flex-1 flex-col">
					<div className="group relative flex min-h-0 flex-1 flex-col overflow-hidden rounded-md border border-input">
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
						<Textarea
							id={id}
							value={jsonText}
							onChange={
								jsonEditingEnabled && onJsonChange
									? (event) => onJsonChange(event.target.value)
									: undefined
							}
							readOnly={!jsonEditingEnabled}
							aria-readonly={!jsonEditingEnabled}
							className="min-h-0 flex-1 resize-none overflow-y-auto border-0 font-mono text-sm focus:outline-none focus:ring-0"
							style={{
								background: "transparent",
								caretColor: jsonEditingEnabled ? "currentColor" : "transparent",
								userSelect: "text",
								WebkitUserSelect: "text",
								MozUserSelect: "text",
								msUserSelect: "text",
							}}
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
