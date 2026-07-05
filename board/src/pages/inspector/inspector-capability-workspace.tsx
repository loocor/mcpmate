import { useMemo, useRef } from "react";
import { PencilLine } from "lucide-react";
import { CapabilityCombobox, type CapabilityKind } from "../../components/capability-combobox";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Label } from "../../components/ui/label";
import { cn } from "../../lib/utils";
import {
	INSPECTOR_CAPABILITY_FAMILIES,
	type InspectorCapabilityFamily,
	type InspectorCapabilityListItem,
} from "./inspector-feature-config";

type InspectorCapabilityWorkspaceProps = {
	activeFamily: InspectorCapabilityFamily | null;
	selectedItem: InspectorCapabilityListItem | null;
	items: InspectorCapabilityListItem[];
	onSelectItemKey: (key: string) => void;
	disabled?: boolean;
};

function familyToComboboxKind(
	family: InspectorCapabilityFamily | null,
): CapabilityKind | null {
	switch (family) {
		case "tools":
			return "tool";
		case "prompts":
			return "prompt";
		case "resources":
			return "resource";
		case "resource_templates":
			return "template";
		default:
			return null;
	}
}

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
	onSelectItemKey,
	disabled = false,
}: InspectorCapabilityWorkspaceProps) {
	const pickerRef = useRef<HTMLDivElement>(null);
	const familyMeta = INSPECTOR_CAPABILITY_FAMILIES.find(
		(entry) => entry.value === activeFamily,
	);
	const comboboxKind = familyToComboboxKind(activeFamily);

	const comboboxItems = useMemo(
		() =>
			items.map((item) => ({
				...item,
				name: item.title,
			})),
		[items],
	);

	if (!activeFamily) {
		return (
			<div className="flex h-full min-h-[320px] flex-col items-center justify-center rounded-md border border-dashed border-border bg-card/20 p-8 text-center">
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
			<div className="rounded-md border border-border bg-card/60 p-4">
				<div className="flex flex-wrap items-start justify-between gap-3">
					<div className="min-w-0 space-y-1">
						<div className="flex flex-wrap items-center gap-2">
							<p className="text-lg font-semibold text-foreground">
								{familyMeta.label}
							</p>
							{familyMeta.placeholder ? (
								<Badge variant="outline">2026 draft</Badge>
							) : null}
						</div>
						<p className="text-sm text-muted-foreground">
							List method: <span className="font-mono">{familyMeta.listMethod}</span>
						</p>
					</div>
					{selectedItem ? (
						<Button type="button" variant="outline" size="sm" className="gap-2" disabled>
							<PencilLine className="h-4 w-4" />
							Edit metadata
						</Button>
					) : null}
				</div>

				<div ref={pickerRef} className="mt-4 space-y-2">
					<Label>Capability</Label>
					{comboboxKind ? (
						<CapabilityCombobox
							kind={comboboxKind}
							items={comboboxItems}
							value={selectedItem?.key ?? ""}
							onChange={(key) => onSelectItemKey(key)}
							loading={false}
							placeholder={
								items.length > 0
									? "Search listed capabilities..."
									: "List this family in the sidebar first"
							}
							menuMatchTargetWidth
							menuWidthTargetRef={pickerRef}
							getKey={(item) => item.key}
							getLabel={(item) => item.title}
							getDescription={(item) => item.description}
							triggerClassName={cn(disabled && "pointer-events-none opacity-60")}
						/>
					) : (
						<p className="text-sm text-muted-foreground">
							Picker for {familyMeta.label} will arrive with the 2026 specification
							wiring.
						</p>
					)}
				</div>
			</div>

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
				<div className="flex flex-1 items-center justify-center rounded-md border border-dashed border-border bg-card/20 p-6 text-sm text-muted-foreground">
					{items.length > 0
						? "Choose a capability from the picker or sidebar list."
						: "Run List in the sidebar to populate this workspace."}
				</div>
			)}
		</div>
	);
}
