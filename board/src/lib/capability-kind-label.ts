import type { TFunction } from "i18next";
import type { CapabilityKind } from "../components/capability-list";
import { toTitleCase } from "./utils";

export function matchesCapabilityKindFilter(
	kindFilters: CapabilityKind[],
	kind: CapabilityKind,
): boolean {
	return kindFilters.length === 0 || kindFilters.includes(kind);
}

export function capabilityKindLabel(t: TFunction, kind: CapabilityKind): string {
	if (kind === "templates") {
		return t("servers:detail.capabilityList.labels.templates", {
			defaultValue: "Resource Templates",
		});
	}
	return t(`servers:detail.capabilityList.labels.${kind}`, {
		defaultValue: toTitleCase(kind),
	});
}
