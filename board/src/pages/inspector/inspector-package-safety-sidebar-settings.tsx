import {
	INSPECTOR_PACKAGE_SAFETY_DATABASES,
	INSPECTOR_PACKAGE_SAFETY_DATABASE_TOOLTIP,
	INSPECTOR_PACKAGE_SAFETY_FACT_SOURCES,
	INSPECTOR_PACKAGE_SAFETY_FACT_SOURCE_TOOLTIP,
	INSPECTOR_PACKAGE_SAFETY_SCAN_DEPTHS,
	INSPECTOR_PACKAGE_SAFETY_SCAN_DEPTH_TOOLTIP,
	INSPECTOR_PACKAGE_SAFETY_SETTINGS_NOTE,
	type InspectorPackageSafetyDatabase,
	type InspectorPackageSafetyFactSource,
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
	packageSafetyFactSource: InspectorPackageSafetyFactSource;
	onPackageSafetyFactSourceChange: (value: InspectorPackageSafetyFactSource) => void;
	packageSafetyDatabase: InspectorPackageSafetyDatabase;
	onPackageSafetyDatabaseChange: (value: InspectorPackageSafetyDatabase) => void;
	packageSafetyScanDepth: InspectorPackageSafetyScanDepth;
	onPackageSafetyScanDepthChange: (value: InspectorPackageSafetyScanDepth) => void;
};

export function InspectorPackageSafetySidebarSettings({
	packageSafetyFactSource,
	onPackageSafetyFactSourceChange,
	packageSafetyDatabase,
	onPackageSafetyDatabaseChange,
	packageSafetyScanDepth,
	onPackageSafetyScanDepthChange,
}: InspectorPackageSafetySidebarSettingsProps) {
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
				<InspectorSidebarSegmentControl
					options={INSPECTOR_PACKAGE_SAFETY_FACT_SOURCES.map((option) => ({
						value: option.value,
						label: option.segmentLabel,
						ariaLabel: option.label,
					}))}
					value={packageSafetyFactSource}
					onValueChange={onPackageSafetyFactSourceChange}
				/>
			</section>

			<section className="space-y-1">
				<InspectorSidebarSectionHeader
					title="Advisory database"
					tooltip={
						<InspectorSidebarOptionTooltipBody
							summary={INSPECTOR_PACKAGE_SAFETY_DATABASE_TOOLTIP}
							options={inspectorSidebarSegmentTooltipOptions(
								INSPECTOR_PACKAGE_SAFETY_DATABASES,
							)}
						/>
					}
				/>
				<InspectorSidebarSegmentControl
					options={INSPECTOR_PACKAGE_SAFETY_DATABASES.map((option) => ({
						value: option.value,
						label: option.segmentLabel,
						ariaLabel: option.label,
					}))}
					value={packageSafetyDatabase}
					onValueChange={onPackageSafetyDatabaseChange}
				/>
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
