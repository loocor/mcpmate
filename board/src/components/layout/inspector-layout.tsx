import { useAppStore } from "../../lib/store";
import { cn } from "../../lib/utils";
import type { InspectorFooterWorkspace } from "../../pages/inspector/inspector-feature-config";
import { Header } from "./header";
import { InspectorSidebar } from "./inspector-sidebar";

type InspectorWindowLayoutProps = {
	sidebar: React.ReactNode;
	children: React.ReactNode;
	footerWorkspace: InspectorFooterWorkspace | null;
	onFooterWorkspaceChange: (workspace: InspectorFooterWorkspace) => void;
	workspaceModeLabel: string;
	headerActions?: React.ReactNode;
};

export function InspectorWindowLayout({
	sidebar,
	children,
	footerWorkspace,
	onFooterWorkspaceChange,
	workspaceModeLabel,
	headerActions,
}: InspectorWindowLayoutProps) {
	const sidebarOpen = useAppStore((state) => state.sidebarOpen);

	return (
		<div className="flex h-screen flex-col overflow-hidden">
			<InspectorSidebar
				footerWorkspace={footerWorkspace}
				onFooterWorkspaceChange={onFooterWorkspaceChange}
			>
				{sidebar}
			</InspectorSidebar>
			<Header titleOverride={workspaceModeLabel} actionsOverride={headerActions} />
			<main
				className={cn(
					"min-h-0 min-w-0 flex-1 pt-16 transition-all duration-300 ease-in-out",
					sidebarOpen ? "ml-64" : "ml-16",
				)}
			>
				<div className="box-border flex h-full w-full min-w-0 flex-col overflow-hidden">
					{children}
				</div>
			</main>
		</div>
	);
}
