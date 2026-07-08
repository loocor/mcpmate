import {
	INSPECTOR_PACKAGE_SAFETY_FACT_SOURCES,
	INSPECTOR_PACKAGE_SAFETY_FACT_SOURCE_TOOLTIP,
	INSPECTOR_PACKAGE_SAFETY_SCAN_DEPTHS,
	INSPECTOR_PACKAGE_SAFETY_SCAN_DEPTH_TOOLTIP,
	INSPECTOR_PACKAGE_SAFETY_SETTINGS_NOTE,
	type InspectorPackageSafetyScanDepth,
} from "./inspector-feature-config";
import {
	InspectorSidebarOptionTooltipBody,
	InspectorSidebarSectionHeader,
	InspectorSidebarSegmentControl,
	InspectorSidebarSettingsShell,
	inspectorSidebarSegmentTooltipOptions,
} from "./inspector-sidebar-settings-ui";

type InspectorPackageSafetySidebarSettingsProps = {
	packageSafetyScanDepth: InspectorPackageSafetyScanDepth;
	onPackageSafetyScanDepthChange: (value: InspectorPackageSafetyScanDepth) => void;
};

export function InspectorPackageSafetySidebarSettings({
	packageSafetyScanDepth,
	onPackageSafetyScanDepthChange,
}: InspectorPackageSafetySidebarSettingsProps) {
	const factSource = INSPECTOR_PACKAGE_SAFETY_FACT_SOURCES[0];

	return (
		<InspectorSidebarSettingsShell notes={INSPECTOR_PACKAGE_SAFETY_SETTINGS_NOTE}>
			<section className="space-y-1">
				<InspectorSidebarSectionHeader
					title="Fact source"
					tooltip={
						<InspectorSidebarOptionTooltipBody
							summary={INSPECTOR_PACKAGE_SAFETY_FACT_SOURCE_TOOLTIP}
							options={inspectorSidebarSegmentTooltipOptions(
								INSPECTOR_PACKAGE_SAFETY_FACT_SOURCES,
							)}
						/>
					}
				/>
				<div className="rounded-md border border-border bg-muted/60 px-3 py-2">
					<p className="text-sm font-medium text-foreground">{factSource.label}</p>
					<p className="mt-1 text-xs leading-relaxed text-muted-foreground">
						{factSource.description}
					</p>
				</div>
			</section>

			<section className="space-y-1">
				<InspectorSidebarSectionHeader
					title="Scan depths"
					tooltip={
						<InspectorSidebarOptionTooltipBody
							summary={INSPECTOR_PACKAGE_SAFETY_SCAN_DEPTH_TOOLTIP}
							options={inspectorSidebarSegmentTooltipOptions(
								INSPECTOR_PACKAGE_SAFETY_SCAN_DEPTHS,
							)}
						/>
					}
				/>
				<InspectorSidebarSegmentControl
					options={INSPECTOR_PACKAGE_SAFETY_SCAN_DEPTHS.map((option) => ({
						value: option.value,
						label: option.segmentLabel,
					}))}
					value={packageSafetyScanDepth}
					onValueChange={onPackageSafetyScanDepthChange}
				/>
			</section>
		</InspectorSidebarSettingsShell>
	);
}
