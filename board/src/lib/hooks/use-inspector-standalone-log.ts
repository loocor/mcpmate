import { useCallback, useEffect, useRef, useState } from "react";
import {
	createInspectorLogEntry,
	filterInspectorLogEventsForServer,
	filterInspectorStandaloneActivityLogEvents,
	isInspectorStandaloneActivityLogEntry,
	normalizeInspectorLogEvents,
	trimInspectorLogEvents,
	type CreateInspectorLogEntryInput,
	type InspectorLogEventEntry,
} from "../inspector-event-log";
import {
	clearInspectorStandaloneEvents,
	loadInspectorStandaloneEvents,
	saveInspectorStandaloneEvents,
} from "../inspector-standalone-storage";

function loadEventsForServer(serverId: string): InspectorLogEventEntry[] {
	if (!serverId) {
		return [];
	}
	return filterInspectorLogEventsForServer(
		filterInspectorStandaloneActivityLogEvents(
			normalizeInspectorLogEvents(loadInspectorStandaloneEvents(serverId)),
		),
		serverId,
	);
}

export function useInspectorStandaloneLog(
	serverId: string,
	onAppended?: () => void,
): {
	events: InspectorLogEventEntry[];
	appendEvent: (input: CreateInspectorLogEntryInput) => void;
	clearEvents: () => void;
} {
	const [events, setEvents] = useState<InspectorLogEventEntry[]>(() =>
		loadEventsForServer(serverId),
	);
	const eventsRef = useRef(events);
	eventsRef.current = events;
	const activeServerIdRef = useRef(serverId);
	const suppressPersistRef = useRef(false);

	useEffect(() => {
		const leavingServerId = activeServerIdRef.current;
		if (leavingServerId && leavingServerId !== serverId) {
			saveInspectorStandaloneEvents(
				leavingServerId,
				filterInspectorStandaloneActivityLogEvents(
					filterInspectorLogEventsForServer(eventsRef.current, leavingServerId),
				),
			);
		}
		activeServerIdRef.current = serverId;
		suppressPersistRef.current = true;
		setEvents(loadEventsForServer(serverId));
	}, [serverId]);

	useEffect(() => {
		if (!serverId || activeServerIdRef.current !== serverId) {
			return;
		}
		if (suppressPersistRef.current) {
			suppressPersistRef.current = false;
			return;
		}
		saveInspectorStandaloneEvents(
			serverId,
			filterInspectorStandaloneActivityLogEvents(
				filterInspectorLogEventsForServer(events, serverId),
			),
		);
	}, [events, serverId]);

	useEffect(() => {
		return () => {
			const id = activeServerIdRef.current;
			if (id) {
				saveInspectorStandaloneEvents(
					id,
					filterInspectorStandaloneActivityLogEvents(
						filterInspectorLogEventsForServer(eventsRef.current, id),
					),
				);
			}
		};
	}, []);

	const appendEvent = useCallback(
		(input: CreateInspectorLogEntryInput) => {
			const entry = createInspectorLogEntry(input);
			if (!isInspectorStandaloneActivityLogEntry(entry)) {
				return;
			}
			setEvents((prev) =>
				trimInspectorLogEvents([...prev, entry]),
			);
			onAppended?.();
		},
		[onAppended],
	);

	const clearEvents = useCallback(() => {
		setEvents([]);
		if (serverId) {
			clearInspectorStandaloneEvents(serverId);
		}
	}, [serverId]);

	return { events, appendEvent, clearEvents };
}
