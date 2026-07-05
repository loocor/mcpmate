import type { ReactNode } from "react";
import { InspectorCapabilityAccordionSidebar } from "./inspector-capability-accordion-sidebar";
import { InspectorCompatibilitySidebarSettings } from "./inspector-compatibility-sidebar-settings";
import { InspectorLlmEvaluationSidebarSettings } from "./inspector-llm-evaluation-sidebar-settings";
import { InspectorPackageSafetySidebarSettings } from "./inspector-package-safety-sidebar-settings";
import { InspectorSidebarSectionHeader } from "./inspector-sidebar-settings-ui";
import { INSPECTOR_CAPABILITY_FAMILIES } from "./inspector-feature-config";
import type {
	InspectorCapabilityFamily,
	InspectorCapabilityFamilyOption,
	InspectorCapabilityFamilyState,
	InspectorCompatibilitySpecVersion,
	InspectorFeatureTab,
	InspectorLlmEvaluationFocus,
	InspectorPackageSafetyDatabase,
	InspectorPackageSafetyFactSource,
	InspectorPackageSafetyScanDepth,
} from "./inspector-feature-config";

type InspectorFeatureSidebarPanelProps = {
	featureTab: InspectorFeatureTab;
	onFeatureTabActivate: (tab: InspectorFeatureTab) => void;
	hasSelectedTarget: boolean;
	capabilityFamilyStates: Record<InspectorCapabilityFamily, InspectorCapabilityFamilyState>;
	capabilityFamilies: InspectorCapabilityFamilyOption[];
	activeCapabilityFamily: InspectorCapabilityFamily | null;
	onActiveCapabilityFamilyChange: (family: InspectorCapabilityFamily | null) => void;
	onCapabilityList: (family: InspectorCapabilityFamily) => void;
	onCapabilityClear: (family: InspectorCapabilityFamily) => void;
	onCapabilitySelectItem: (family: InspectorCapabilityFamily, key: string) => void;
	capabilitySearch: string;
	onCapabilitySearchChange: (value: string) => void;
	capabilitySearchOpen: boolean;
	onCapabilitySearchOpenChange: (open: boolean) => void;
	capabilityControlsDisabled?: boolean;
	compatibilitySpecVersion: InspectorCompatibilitySpecVersion;
	onCompatibilitySpecVersionChange: (value: InspectorCompatibilitySpecVersion) => void;
	packageSafetyFactSource: InspectorPackageSafetyFactSource;
	onPackageSafetyFactSourceChange: (value: InspectorPackageSafetyFactSource) => void;
	packageSafetyDatabase: InspectorPackageSafetyDatabase;
	onPackageSafetyDatabaseChange: (value: InspectorPackageSafetyDatabase) => void;
	packageSafetyScanDepth: InspectorPackageSafetyScanDepth;
	onPackageSafetyScanDepthChange: (value: InspectorPackageSafetyScanDepth) => void;
	llmEvaluationFocus: InspectorLlmEvaluationFocus[];
	onLlmEvaluationFocusChange: (value: InspectorLlmEvaluationFocus[]) => void;
	llmEvaluationProviderId: string;
	onLlmEvaluationProviderIdChange: (value: string) => void;
};

function InspectorFeatureSettingsArea({
	featureTab,
	onFeatureTabActivate,
	className,
	children,
}: {
	featureTab: InspectorFeatureTab;
	onFeatureTabActivate: (tab: InspectorFeatureTab) => void;
	className?: string;
	children: ReactNode;
}) {
	const activate = () => onFeatureTabActivate(featureTab);

	return (
		<div
			className={className}
			onPointerDownCapture={activate}
			onFocusCapture={activate}
		>
			{children}
		</div>
	);
}

export function InspectorFeatureSidebarPanel({
	featureTab,
	onFeatureTabActivate,
	hasSelectedTarget,
	capabilityFamilyStates,
	capabilityFamilies,
	activeCapabilityFamily,
	onActiveCapabilityFamilyChange,
	onCapabilityList,
	onCapabilityClear,
	onCapabilitySelectItem,
	capabilitySearch,
	onCapabilitySearchChange,
	capabilitySearchOpen,
	onCapabilitySearchOpenChange,
	capabilityControlsDisabled = false,
	compatibilitySpecVersion,
	onCompatibilitySpecVersionChange,
	packageSafetyFactSource,
	onPackageSafetyFactSourceChange,
	packageSafetyDatabase,
	onPackageSafetyDatabaseChange,
	packageSafetyScanDepth,
	onPackageSafetyScanDepthChange,
	llmEvaluationFocus,
	onLlmEvaluationFocusChange,
	llmEvaluationProviderId,
	onLlmEvaluationProviderIdChange,
}: InspectorFeatureSidebarPanelProps) {
	if (featureTab === "inspect") {
		if (!hasSelectedTarget) {
			return (
				<InspectorFeatureSettingsArea
					featureTab={featureTab}
					onFeatureTabActivate={onFeatureTabActivate}
					className="flex min-h-0 flex-1 flex-col"
				>
					<InspectorSidebarSectionHeader
						title="Capabilities"
						tooltip={
							<p>
								MCP capability families advertised by the connected server.
								Select a server and run List to discover each family.
							</p>
						}
					/>
					<div className="flex min-h-0 flex-1 flex-col gap-0.5">
						{INSPECTOR_CAPABILITY_FAMILIES.map((family) => (
							<div
								key={family.value}
								className="pl-0.5 py-1 text-xs text-muted-foreground"
							>
								{family.label}
							</div>
						))}
					</div>
					<p className="shrink-0 pt-3 text-xs leading-relaxed text-muted-foreground">
						Select a server to connect, then inspect its capabilities.
					</p>
				</InspectorFeatureSettingsArea>
			);
		}

		return (
			<InspectorFeatureSettingsArea
				featureTab={featureTab}
				onFeatureTabActivate={onFeatureTabActivate}
				className="flex min-h-0 flex-1 flex-col"
			>
				<InspectorCapabilityAccordionSidebar
					familyStates={capabilityFamilyStates}
					families={capabilityFamilies}
					activeFamily={activeCapabilityFamily}
					onActiveFamilyChange={onActiveCapabilityFamilyChange}
					onList={onCapabilityList}
					onClear={onCapabilityClear}
					onSelectItem={onCapabilitySelectItem}
					capabilitySearch={capabilitySearch}
					onCapabilitySearchChange={onCapabilitySearchChange}
					capabilitySearchOpen={capabilitySearchOpen}
					onCapabilitySearchOpenChange={onCapabilitySearchOpenChange}
					disabled={capabilityControlsDisabled}
				/>
			</InspectorFeatureSettingsArea>
		);
	}

	if (featureTab === "compatibility") {
		return (
			<InspectorFeatureSettingsArea
				featureTab={featureTab}
				onFeatureTabActivate={onFeatureTabActivate}
				className="flex min-h-0 flex-1 flex-col"
			>
				<InspectorCompatibilitySidebarSettings
					compatibilitySpecVersion={compatibilitySpecVersion}
					onCompatibilitySpecVersionChange={onCompatibilitySpecVersionChange}
				/>
			</InspectorFeatureSettingsArea>
		);
	}

	if (featureTab === "package_safety") {
		return (
			<InspectorFeatureSettingsArea
				featureTab={featureTab}
				onFeatureTabActivate={onFeatureTabActivate}
				className="flex min-h-0 flex-1 flex-col"
			>
				<InspectorPackageSafetySidebarSettings
					packageSafetyFactSource={packageSafetyFactSource}
					onPackageSafetyFactSourceChange={onPackageSafetyFactSourceChange}
					packageSafetyDatabase={packageSafetyDatabase}
					onPackageSafetyDatabaseChange={onPackageSafetyDatabaseChange}
					packageSafetyScanDepth={packageSafetyScanDepth}
					onPackageSafetyScanDepthChange={onPackageSafetyScanDepthChange}
				/>
			</InspectorFeatureSettingsArea>
		);
	}

	return (
		<InspectorFeatureSettingsArea
			featureTab={featureTab}
			onFeatureTabActivate={onFeatureTabActivate}
			className="flex min-h-0 flex-1 flex-col"
		>
			<InspectorLlmEvaluationSidebarSettings
				llmEvaluationFocus={llmEvaluationFocus}
				onLlmEvaluationFocusChange={onLlmEvaluationFocusChange}
				llmEvaluationProviderId={llmEvaluationProviderId}
				onLlmEvaluationProviderIdChange={onLlmEvaluationProviderIdChange}
			/>
		</InspectorFeatureSettingsArea>
	);
}
