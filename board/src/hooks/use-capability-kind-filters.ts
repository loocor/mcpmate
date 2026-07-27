import { useCallback, useMemo, useState } from "react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import type { CapabilityKind } from "../components/capability-list";
import {
	capabilityKindLabel,
	matchesCapabilityKindFilter,
} from "../lib/capability-kind-label";

export const CAPABILITY_KINDS: CapabilityKind[] = [
	"tools",
	"resources",
	"templates",
	"prompts",
];

type UseCapabilityKindFiltersOptions = {
	filterKeyPrefix?: string;
};

export function useCapabilityKindFilters(
	t: TFunction,
	options: UseCapabilityKindFiltersOptions = {},
) {
	const { i18n } = useTranslation();
	const filterKeyPrefix =
		options.filterKeyPrefix ?? "servers:detail.filters.kind";
	const [kindFilters, setKindFilters] = useState<CapabilityKind[]>([]);

	const kindMatches = useCallback(
		(kind: CapabilityKind) => matchesCapabilityKindFilter(kindFilters, kind),
		[kindFilters],
	);

	const kindFilterOptions = useMemo(
		() =>
			CAPABILITY_KINDS.map((kind) => ({
				value: kind,
				label: capabilityKindLabel(t, kind),
			})),
		[t, i18n.language],
	);

	const kindFilterLabel = useMemo(() => {
		if (kindFilters.length === 0) {
			return t(`${filterKeyPrefix}.all`, { defaultValue: "All Types" });
		}
		if (kindFilters.length === 1) {
			return capabilityKindLabel(t, kindFilters[0]);
		}
		return t(`${filterKeyPrefix}.selected`, {
			count: kindFilters.length,
			defaultValue: "{{count}} Types",
		});
	}, [filterKeyPrefix, kindFilters, t, i18n.language]);

	const toggleKindFilter = useCallback(
		(kind: CapabilityKind, checked: boolean) => {
			setKindFilters((current) => {
				if (checked) {
					return current.includes(kind) ? current : [...current, kind];
				}
				return current.filter((value) => value !== kind);
			});
		},
		[],
	);

	const clearKindFilters = useCallback(() => {
		setKindFilters([]);
	}, []);

	return {
		kindFilters,
		setKindFilters,
		kindMatches,
		kindFilterOptions,
		kindFilterLabel,
		toggleKindFilter,
		clearKindFilters,
	};
}
