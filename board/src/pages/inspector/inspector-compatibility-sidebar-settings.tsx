import { ExternalLink } from "lucide-react";
import {
	INSPECTOR_COMPATIBILITY_SPEC_BASELINE_NOTE,
	INSPECTOR_COMPATIBILITY_SPEC_OPTIONS,
	INSPECTOR_COMPATIBILITY_SPEC_VERSION_TOOLTIP,
	type InspectorCompatibilitySpecVersion,
} from "./inspector-feature-config";
import {
	InspectorSidebarChoiceGridControl,
	InspectorSidebarOptionTooltipBody,
	InspectorSidebarSectionHeader,
	InspectorSidebarSettingsShell,
	inspectorSidebarSegmentTooltipOptions,
} from "./inspector-sidebar-settings-ui";

type InspectorCompatibilitySidebarSettingsProps = {
	compatibilitySpecVersion: InspectorCompatibilitySpecVersion;
	onCompatibilitySpecVersionChange: (value: InspectorCompatibilitySpecVersion) => void;
};

export function InspectorCompatibilitySidebarSettings({
	compatibilitySpecVersion,
	onCompatibilitySpecVersionChange,
}: InspectorCompatibilitySidebarSettingsProps) {
	const activeSpec =
		INSPECTOR_COMPATIBILITY_SPEC_OPTIONS.find(
			(option) => option.value === compatibilitySpecVersion,
		) ?? INSPECTOR_COMPATIBILITY_SPEC_OPTIONS[0];

	return (
		<InspectorSidebarSettingsShell notes={INSPECTOR_COMPATIBILITY_SPEC_BASELINE_NOTE}>
			<section className="space-y-1">
				<InspectorSidebarSectionHeader
					title="Specification version"
					tooltip={
						<InspectorSidebarOptionTooltipBody
							summary={INSPECTOR_COMPATIBILITY_SPEC_VERSION_TOOLTIP}
							options={inspectorSidebarSegmentTooltipOptions(
								INSPECTOR_COMPATIBILITY_SPEC_OPTIONS,
							)}
						/>
					}
				/>
				<InspectorSidebarChoiceGridControl
					options={INSPECTOR_COMPATIBILITY_SPEC_OPTIONS.map((option) => ({
						value: option.value,
						label: option.segmentLabel,
					}))}
					value={compatibilitySpecVersion}
					onValueChange={onCompatibilitySpecVersionChange}
				/>
				<div className="space-y-1.5 pt-0.5">
					<p className="text-xs leading-relaxed text-muted-foreground">
						{activeSpec.description}
						{activeSpec.specUrl ? (
							<>
								{" "}
								<a
									href={activeSpec.specUrl}
									target="_blank"
									rel="noopener noreferrer"
									aria-label="View specification"
									className="inline-flex align-text-bottom text-inherit transition-opacity hover:opacity-80"
								>
									<ExternalLink className="h-3 w-3" aria-hidden />
								</a>
							</>
						) : null}
					</p>
					<p className="text-xs leading-relaxed text-foreground/80">
						{activeSpec.highlights}
					</p>
				</div>
			</section>
		</InspectorSidebarSettingsShell>
	);
}
