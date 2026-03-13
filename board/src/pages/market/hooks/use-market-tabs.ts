import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { TFunction } from "i18next";
import { type MarketPortalMeta, useAppStore } from "../../../lib/store";
import { mergePortalOverrides } from "../portal-registry";
import type { TabItem, UseMarketTabsReturn } from "../types";

export function useMarketTabs(
	t?: TFunction,
): UseMarketTabsReturn {
	// Get default market setting
	const defaultMarket = useAppStore(
		(state) => state.dashboardSettings.defaultMarket,
	);
	const marketPortals = useAppStore(
		(state) => state.dashboardSettings.marketPortals,
	);

	const mergedPortals = useMemo(
		() => mergePortalOverrides(marketPortals),
		[marketPortals],
	);

	const availablePortals = useMemo(
		() =>
			Object.values(mergedPortals).sort((a, b) =>
				a.label.localeCompare(b.label),
			),
		[mergedPortals],
	);

	const createPortalTab = useCallback(
		(meta: MarketPortalMeta, closable: boolean): TabItem => ({
			id: meta.id,
			portalId: meta.id,
			label: meta.label,
			type: "third-party",
			url: meta.proxyPath,
			icon: meta.proxyFavicon ?? meta.favicon,
			closable,
		}),
		[],
	);

	const officialTab = useMemo<TabItem>(
		() => ({
			id: "official",
			label: t
				? t("market:officialRegistry", {
						defaultValue: "Official MCP Registry",
					})
				: "Official MCP Registry",
			type: "official",
			closable: defaultMarket !== "official",
		}),
		[defaultMarket, t],
	);

	const defaultPortalMeta = useMemo(() => {
		if (defaultMarket === "official") return null;
		return mergedPortals[defaultMarket] ?? availablePortals[0] ?? null;
	}, [availablePortals, defaultMarket, mergedPortals]);

	const [tabs, setTabs] = useState<TabItem[]>(() => {
		if (defaultMarket === "official" || !defaultPortalMeta) {
			return [officialTab];
		}
		return [createPortalTab(defaultPortalMeta, false)];
	});

	const [activeTab, setActiveTab] = useState<string>(
		defaultMarket === "official"
			? "official"
			: (defaultPortalMeta?.id ?? "official"),
	);

	const prevDefaultMarketRef = useRef(defaultMarket);

	// Sync tab metadata when portal definitions change
	useEffect(() => {
		setTabs((prev) =>
			prev.map((tab) => {
				if (tab.type === "official") {
					return {
						...tab,
						closable: defaultMarket !== "official",
					};
				}
				if (tab.portalId) {
					const meta = mergedPortals[tab.portalId];
					if (!meta) return tab;
					return {
						...tab,
						label: meta.label,
						url: meta.proxyPath,
						icon: meta.proxyFavicon ?? meta.favicon ?? tab.icon,
						closable: defaultMarket !== meta.id,
					};
				}
				return tab;
			}),
		);
	}, [defaultMarket, mergedPortals]);

	useEffect(() => {
		if (defaultMarket === "official") {
			if (!tabs.some((tab) => tab.type === "official")) {
				setTabs((prev) => [officialTab, ...prev]);
			}
			return;
		}
		const desired = mergedPortals[defaultMarket];
		if (!desired) return;
		setTabs((prev) => {
			const exists = prev.some(
				(tab) => tab.portalId === desired.id && tab.type === "third-party",
			);
			if (exists) return prev;
			return [createPortalTab(desired, false), ...prev];
		});
	}, [createPortalTab, defaultMarket, mergedPortals, officialTab, tabs]);

	useEffect(() => {
		const prev = prevDefaultMarketRef.current;
		if (defaultMarket === prev) {
			return;
		}
		if (defaultMarket === "official") {
			setActiveTab("official");
		} else {
			setActiveTab(defaultMarket);
		}
		prevDefaultMarketRef.current = defaultMarket;
	}, [defaultMarket]);

	const addOfficialTab = useCallback(() => {
		setTabs((prev) => {
			if (prev.some((tab) => tab.id === "official")) return prev;
			return [
				...prev,
				{ ...officialTab, closable: defaultMarket !== "official" },
			];
		});
		setActiveTab("official");
	}, [defaultMarket, officialTab]);

	const addPortalTab = useCallback(
		(portalId: string) => {
			const meta = mergedPortals[portalId];
			if (!meta) return;
			setTabs((prev) => {
				if (prev.some((tab) => tab.portalId === portalId)) {
					return prev;
				}
				return [...prev, createPortalTab(meta, defaultMarket !== portalId)];
			});
			setActiveTab(portalId);
		},
		[createPortalTab, defaultMarket, mergedPortals],
	);

	const closeTab = useCallback(
		(tabId: string) => {
			if (tabId === defaultMarket) return;
			setTabs((prev) => {
				const newTabs = prev.filter((tab) => tab.id !== tabId);
				if (activeTab === tabId) {
					setActiveTab(
						defaultMarket === "official" ? "official" : defaultMarket,
					);
				}
				return newTabs;
			});
		},
		[activeTab, defaultMarket],
	);

	return {
		tabs,
		activeTab,
		setActiveTab,
		availablePortals,
		addPortalTab,
		addOfficialTab,
		closeTab,
		portalMap: mergedPortals,
	};
}
