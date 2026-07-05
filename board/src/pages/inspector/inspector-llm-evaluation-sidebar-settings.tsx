import { useQuery } from "@tanstack/react-query";
import { useEffect, useId, useMemo } from "react";
import { llmApi } from "../../lib/api";
import {
	INSPECTOR_LLM_EVALUATION_FOCUS_OPTIONS,
	INSPECTOR_LLM_EVALUATION_FOCUS_TOOLTIP,
	INSPECTOR_LLM_EVALUATION_PROVIDER_TOOLTIP,
	INSPECTOR_LLM_EVALUATION_SETTINGS_NOTE,
	type InspectorLlmEvaluationFocus,
} from "./inspector-feature-config";
import {
	InspectorSidebarMultiSegmentControl,
	InspectorSidebarOptionTooltipBody,
	InspectorSidebarSectionHeader,
	InspectorSidebarSelect,
	InspectorSidebarSettingsShell,
	inspectorSidebarSegmentTooltipOptions,
} from "./inspector-sidebar-settings-ui";

type InspectorLlmEvaluationSidebarSettingsProps = {
	llmEvaluationFocus: InspectorLlmEvaluationFocus[];
	onLlmEvaluationFocusChange: (value: InspectorLlmEvaluationFocus[]) => void;
	llmEvaluationProviderId: string;
	onLlmEvaluationProviderIdChange: (value: string) => void;
};

export function InspectorLlmEvaluationSidebarSettings({
	llmEvaluationFocus,
	onLlmEvaluationFocusChange,
	llmEvaluationProviderId,
	onLlmEvaluationProviderIdChange,
}: InspectorLlmEvaluationSidebarSettingsProps) {
	const llmProviderId = useId();
	const { data: providers = [], isLoading } = useQuery({
		queryKey: ["llm-providers"],
		queryFn: () => llmApi.listProviders(),
	});

	const defaultProvider = useMemo(
		() => providers.find((provider) => provider.is_default) ?? providers[0] ?? null,
		[providers],
	);

	const providerOptions = useMemo(
		() =>
			providers.map((provider) => ({
				value: provider.id,
				label: provider.name,
				isDefault: provider.is_default,
			})),
		[providers],
	);

	useEffect(() => {
		if (isLoading || providers.length === 0) {
			return;
		}

		const selectedProviderExists = providers.some(
			(provider) => provider.id === llmEvaluationProviderId,
		);
		if (!llmEvaluationProviderId || !selectedProviderExists) {
			if (defaultProvider) {
				onLlmEvaluationProviderIdChange(defaultProvider.id);
			}
		}
	}, [
		defaultProvider,
		isLoading,
		llmEvaluationProviderId,
		onLlmEvaluationProviderIdChange,
		providers,
	]);

	const providerPlaceholder = isLoading
		? "Loading providers..."
		: defaultProvider
			? defaultProvider.name
			: "No providers configured";

	return (
		<InspectorSidebarSettingsShell notes={INSPECTOR_LLM_EVALUATION_SETTINGS_NOTE}>
			<section className="space-y-1">
				<InspectorSidebarSectionHeader
					title="Focus dimensions"
					tooltip={
						<InspectorSidebarOptionTooltipBody
							summary={INSPECTOR_LLM_EVALUATION_FOCUS_TOOLTIP}
							options={inspectorSidebarSegmentTooltipOptions(
								INSPECTOR_LLM_EVALUATION_FOCUS_OPTIONS,
							)}
						/>
					}
				/>
				<InspectorSidebarMultiSegmentControl
					options={INSPECTOR_LLM_EVALUATION_FOCUS_OPTIONS.map((option) => ({
						value: option.value,
						label: option.segmentLabel,
					}))}
					selected={llmEvaluationFocus}
					onChange={onLlmEvaluationFocusChange}
				/>
			</section>

			<section className="space-y-1">
				<InspectorSidebarSectionHeader
					title="Provider"
					tooltip={<p>{INSPECTOR_LLM_EVALUATION_PROVIDER_TOOLTIP}</p>}
				/>
				<InspectorSidebarSelect
					id={llmProviderId}
					value={llmEvaluationProviderId}
					onValueChange={onLlmEvaluationProviderIdChange}
					options={providerOptions}
					placeholder={providerPlaceholder}
					disabled={isLoading || providerOptions.length === 0}
				/>
			</section>
		</InspectorSidebarSettingsShell>
	);
}
