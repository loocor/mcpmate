import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { InspectorCapabilityKind } from "../lib/inspector-capability";
import {
	mapInspectorMcpResponseSegmentToLabel,
	resolveAvailablePayloadSegments,
	type InspectorPayloadSegmentMode,
} from "../lib/inspector-mcp-response-view";
import type { SegmentOption } from "./ui/segment";

export function useInspectorPayloadSegmentOptions(
	value: unknown,
	kind: InspectorCapabilityKind = "tool",
): SegmentOption[] {
	const { t, i18n } = useTranslation("inspector");
	return useMemo(() => {
		const available = resolveAvailablePayloadSegments(value, kind);
		return available.map((mode) => ({
			value: mode,
			label: t(`payload.display.${mode}`, {
				defaultValue: mapInspectorMcpResponseSegmentToLabel(mode),
			}),
		}));
	}, [t, i18n.language, value, kind]);
}

export type { InspectorPayloadSegmentMode };
