import React from "react";

type TabsContextValue = {
	value: string;
	setValue: (v: string) => void;
};

const TabsContext = React.createContext<TabsContextValue | null>(null);

type TabProps = {
	label: React.ReactNode;
	value: string;
	_isTabTrigger?: boolean;
	ctx?: TabsContextValue;
};

export function Tabs({
	defaultValue,
	children,
}: {
	defaultValue?: string;
	children: React.ReactNode;
}) {
	const [value, setValue] = React.useState(defaultValue || "0");
	return (
		<TabsContext.Provider value={{ value, setValue }}>
			{children}
		</TabsContext.Provider>
	);
}

export function TabList({ children }: { children: React.ReactNode }) {
	const ctx = React.useContext(TabsContext)!;
	return (
		<div className="not-prose flex items-center gap-2 border-b border-slate-200 dark:border-slate-700 mb-3">
			{React.Children.map(children, (child, idx) => {
				if (!React.isValidElement(child)) return child;
				return React.cloneElement(child as React.ReactElement<TabProps>, {
					value: String(idx),
					_isTabTrigger: true,
					ctx,
				});
			})}
		</div>
	);
}

export function Tab({ label, value, _isTabTrigger, ctx }: TabProps) {
	if (!_isTabTrigger) return null;
	const active = ctx!.value === value;
	return (
		<button
			type="button"
			className={`px-3 py-2 text-sm -mb-px border-b-2 ${active ? "border-blue-600 text-blue-600 dark:text-blue-400" : "border-transparent text-slate-600 dark:text-slate-300"}`}
			onClick={() => ctx!.setValue(value)}
		>
			{label}
		</button>
	);
}

export function TabPanels({ children }: { children: React.ReactNode }) {
	const ctx = React.useContext(TabsContext)!;
	return (
		<div>
			{React.Children.map(children, (child, idx) => {
				if (!React.isValidElement(child)) return child;
				const v = String(idx);
				return ctx.value === v ? <div>{child}</div> : null;
			})}
		</div>
	);
}
