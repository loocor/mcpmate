import type { TFunction } from "i18next";
import {
	INSPECTOR_CONTEXT_KEYS,
	type InspectorContextKey,
	type InspectorEventProtocolView,
} from "../lib/inspector-event-protocol-view";
import {
	INSPECTOR_RECORD_FILTERABLE_KEYS,
	INSPECTOR_RECORD_PROPERTY_FIELD_ORDER,
	InspectorRecordPropertyPanel,
} from "./inspector-record-property-panel";

export function InspectorContextPropertyPanel({
	context,
	t,
	onFilterByServerId,
	onFilterBySessionId,
}: {
	context: InspectorEventProtocolView["context"];
	t: TFunction;
	onFilterByServerId?: (serverId: string) => void;
	onFilterBySessionId?: (sessionId: string) => void;
}) {
	const orderedContextKeys = INSPECTOR_RECORD_PROPERTY_FIELD_ORDER.filter((key): key is InspectorContextKey =>
		(INSPECTOR_CONTEXT_KEYS as readonly string[]).includes(key),
	);

	return (
		<InspectorRecordPropertyPanel
			record={context}
			t={t}
			fieldOrder={orderedContextKeys}
			filterableKeys={INSPECTOR_RECORD_FILTERABLE_KEYS}
			onFilterByServerId={onFilterByServerId}
			onFilterBySessionId={onFilterBySessionId}
		/>
	);
}
