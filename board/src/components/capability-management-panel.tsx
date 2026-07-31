import type { ReactNode } from "react";
import { BulkSelectionHeader } from "./bulk-selection";
import {
	CapabilityPreviewList,
	type CapabilityPreviewFlatItem,
} from "./capability-preview-list";
import {
	CapabilityToolbar,
	type CapabilityToolbarMultiFilter,
	type CapabilityToolbarSingleFilter,
} from "./capability-toolbar";
import type { CapabilityKind } from "./capability-list";
import type { CapabilityRecord } from "../types/capabilities";
import { matchesCapabilityKindFilter } from "../lib/capability-kind-label";
import { cn } from "../lib/utils";

type CapabilityManagementPanelHeaderVariant = "default" | "toolbar-inline";

type CapabilityManagementPanelProps = {
	title?: string;
	description?: string;
	headerVariant?: CapabilityManagementPanelHeaderVariant;
	isBulkMode: boolean;
	onToggleBulkMode: () => void;
	bulkActions: ReactNode;
	searchValue: string;
	onSearchChange: (value: string) => void;
	searchPlaceholder: string;
	kindFilter: CapabilityToolbarMultiFilter;
	serverFilter?: CapabilityToolbarMultiFilter;
	statusFilter: CapabilityToolbarSingleFilter;
	hasSource?: boolean;
	isLoading?: boolean;
	tools: CapabilityRecord[];
	resources: CapabilityRecord[];
	prompts: CapabilityRecord[];
	templates: CapabilityRecord[];
	kindFilters: CapabilityKind[];
	selectHintText?: string;
	emptyText?: string;
	emptySearchText?: string;
	renderFlatList: (items: CapabilityPreviewFlatItem[]) => ReactNode;
	className?: string;
};

export function CapabilityManagementPanel({
	title,
	description,
	headerVariant = "default",
	isBulkMode,
	onToggleBulkMode,
	bulkActions,
	searchValue,
	onSearchChange,
	searchPlaceholder,
	kindFilter,
	serverFilter,
	statusFilter,
	hasSource = true,
	isLoading = false,
	tools,
	resources,
	prompts,
	templates,
	kindFilters,
	selectHintText,
	emptyText,
	emptySearchText,
	renderFlatList,
	className,
}: CapabilityManagementPanelProps) {
	const kindMatches = (kind: CapabilityKind) =>
		matchesCapabilityKindFilter(kindFilters, kind);
	const showToolsSection =
		kindMatches("tools") && (isLoading || tools.length > 0);
	const showResourcesSection =
		kindMatches("resources") && (isLoading || resources.length > 0);
	const showPromptsSection =
		kindMatches("prompts") && (isLoading || prompts.length > 0);
	const showTemplatesSection =
		kindMatches("templates") && (isLoading || templates.length > 0);

	const capabilityToolbar = (
		<CapabilityToolbar
			className="w-full"
			compact={headerVariant === "toolbar-inline"}
			searchValue={searchValue}
			onSearchChange={onSearchChange}
			searchPlaceholder={searchPlaceholder}
			serverFilter={serverFilter}
			kindFilter={kindFilter}
			statusFilter={statusFilter}
		/>
	);

	return (
		<div className={cn("flex min-h-0 flex-col", className)}>
			<div className="shrink-0 p-3">
				{headerVariant === "toolbar-inline" ? (
					<BulkSelectionHeader
						className="mb-0"
						leading={capabilityToolbar}
						isBulkMode={isBulkMode}
						onToggleBulkMode={onToggleBulkMode}
						actions={bulkActions}
					/>
				) : (
					<>
						<BulkSelectionHeader
							className="mb-3"
							title={title}
							description={description}
							isBulkMode={isBulkMode}
							onToggleBulkMode={onToggleBulkMode}
							actions={bulkActions}
						/>
						{capabilityToolbar}
					</>
				)}
			</div>
			<CapabilityPreviewList
				className="mx-3 mb-3 mt-0"
				contentClassName="flex min-h-0 flex-1 flex-col p-0"
				framed={false}
				showHeader={false}
				hasSource={hasSource}
				isLoading={isLoading}
				searchValue={searchValue}
				tools={showToolsSection ? tools : []}
				resources={showResourcesSection ? resources : []}
				prompts={showPromptsSection ? prompts : []}
				templates={showTemplatesSection ? templates : []}
				selectHintText={selectHintText}
				emptyText={emptyText}
				emptySearchText={emptySearchText}
				renderFlatList={renderFlatList}
			/>
		</div>
	);
}
