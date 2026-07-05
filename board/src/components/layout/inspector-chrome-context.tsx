import { createContext, useContext, type ReactNode } from "react";

export type InspectorChromeContextValue = {
	activityPanelExpanded: boolean;
	toggleActivityPanel: () => void;
};

export const INSPECTOR_ACTIVITY_TRIGGER_SELECTOR = "[data-inspector-activity-trigger]";

const InspectorChromeContext = createContext<InspectorChromeContextValue | null>(null);

export function InspectorChromeProvider({
	value,
	children,
}: {
	value: InspectorChromeContextValue;
	children: ReactNode;
}) {
	return (
		<InspectorChromeContext.Provider value={value}>{children}</InspectorChromeContext.Provider>
	);
}

export function useInspectorChrome(): InspectorChromeContextValue | null {
	return useContext(InspectorChromeContext);
}
