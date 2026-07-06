import {
	INSPECTOR_CAPABILITY_FAMILIES,
	type InspectorCapabilityFamily,
	type InspectorCapabilityListItem,
} from "./inspector-feature-config";

type InspectorCapabilityWorkspaceProps = {
	activeFamily: InspectorCapabilityFamily | null;
	selectedItem: InspectorCapabilityListItem | null;
	items: InspectorCapabilityListItem[];
};

function schemaPreview(value: Record<string, unknown> | undefined, emptyLabel: string) {
	if (!value || Object.keys(value).length === 0) {
		return (
			<p className="text-sm text-muted-foreground">{emptyLabel}</p>
		);
	}
	return (
		<pre className="max-h-64 overflow-auto whitespace-pre-wrap break-words rounded-md border border-border bg-background p-3 font-mono text-xs text-muted-foreground">
			{JSON.stringify(value, null, 2)}
		</pre>
	);
}

export function InspectorCapabilityWorkspace({
	activeFamily,
	selectedItem,
	items,
}: InspectorCapabilityWorkspaceProps) {
	const familyMeta = INSPECTOR_CAPABILITY_FAMILIES.find(
		(entry) => entry.value === activeFamily,
	);

	if (!activeFamily) {
		return (
			<div className="flex min-h-0 flex-1 flex-col items-center justify-center rounded-md border border-dashed border-border bg-card/20 p-8 text-center">
				<p className="text-base font-medium text-foreground">Select a capability family</p>
				<p className="mt-2 max-w-md text-sm text-muted-foreground">
					Expand a family in the sidebar, run List, then choose an item to inspect its
					schema step by step.
				</p>
			</div>
		);
	}

	if (!familyMeta) {
		return null;
	}

	return (
		<div className="flex min-h-0 flex-1 flex-col gap-4">
			{selectedItem ? (
				<div className="grid min-h-0 flex-1 gap-4 lg:grid-cols-2">
					<div className="space-y-3 rounded-md border border-border bg-card/40 p-4">
						<div>
							<p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
								Title
							</p>
							<p className="mt-1 text-base font-medium text-foreground">
								{selectedItem.title}
							</p>
						</div>
						<div>
							<p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
								Description
							</p>
							<p className="mt-1 text-sm leading-relaxed text-muted-foreground">
								{selectedItem.description || "No description provided."}
							</p>
						</div>
						<div>
							<p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
								Key
							</p>
							<p className="mt-1 font-mono text-sm text-foreground">{selectedItem.key}</p>
						</div>
					</div>
					<div className="space-y-4">
						<div className="space-y-2">
							<p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
								Input schema
							</p>
							{schemaPreview(selectedItem.inputSchema, "No input schema listed.")}
						</div>
						<div className="space-y-2">
							<p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
								Output schema
							</p>
							{schemaPreview(selectedItem.outputSchema, "No output schema listed.")}
						</div>
					</div>
				</div>
			) : (
				<div className="flex min-h-0 flex-1 items-center justify-center rounded-md border border-dashed border-border bg-card/20 p-6 text-sm text-muted-foreground">
					{items.length > 0
						? "Choose a capability from the sidebar list."
						: "Run List in the sidebar to populate this workspace."}
				</div>
			)}
		</div>
	);
}
