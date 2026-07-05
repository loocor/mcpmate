import { useCallback, useEffect, useRef, useState } from "react";
import { inspectorApi } from "../api";
import type { InspectorSessionOpenData } from "../types";

export type InspectorNativeTargetRequest = {
	mode: "native";
	server_id?: string;
	scratch_id?: string;
};

function nativeTargetCacheKey(
	target: InspectorNativeTargetRequest | null,
): string | null {
	if (!target) return null;
	if (target.server_id) return `managed:${target.server_id}`;
	if (target.scratch_id) return `scratch:${target.scratch_id}`;
	return null;
}

export function useInspectorNativeSession(
	target: InspectorNativeTargetRequest | null,
) {
	const nativeSessionRef = useRef<InspectorSessionOpenData | null>(null);
	const nativeSessionTargetKeyRef = useRef<string | null>(null);
	const pendingNativeSessionRef = useRef<{
		targetKey: string;
		promise: Promise<InspectorSessionOpenData>;
	} | null>(null);
	const [connected, setConnected] = useState(false);
	const [sessionId, setSessionId] = useState<string | null>(null);

	const closeSession = useCallback(async (session: InspectorSessionOpenData) => {
		try {
			await inspectorApi.sessionClose({ session_id: session.session_id });
		} catch {
			// Best-effort session cleanup.
		}
	}, []);

	const invalidateSession = useCallback(() => {
		const pending = pendingNativeSessionRef.current;
		if (pending) {
			void pending.promise
				.then((session) => closeSession(session))
				.catch(() => undefined);
			pendingNativeSessionRef.current = null;
		}

		const current = nativeSessionRef.current;
		nativeSessionRef.current = null;
		nativeSessionTargetKeyRef.current = null;
		setConnected(false);
		setSessionId(null);
		if (current) {
			void closeSession(current);
		}
	}, [closeSession]);

	const ensureSession = useCallback(async (): Promise<string | undefined> => {
		const cacheKey = nativeTargetCacheKey(target);
		if (!target || !cacheKey) {
			return undefined;
		}

		const current = nativeSessionRef.current;
		if (current && nativeSessionTargetKeyRef.current === cacheKey) {
			return current.session_id;
		}

		const pending = pendingNativeSessionRef.current;
		if (pending?.targetKey === cacheKey) {
			const session = await pending.promise;
			return session.session_id;
		}

		if (current) {
			await closeSession(current);
			nativeSessionRef.current = null;
			nativeSessionTargetKeyRef.current = null;
			setConnected(false);
		}

		const pendingPromise = inspectorApi
			.sessionOpen(target)
			.then((response) => {
				if (!response?.success || !response.data) {
					throw new Error(
						response?.error
							? String(response.error)
							: "Failed to open inspector session",
					);
				}
				return response.data;
			});

		pendingNativeSessionRef.current = {
			targetKey: cacheKey,
			promise: pendingPromise,
		};

		try {
			const session = await pendingPromise;
			if (pendingNativeSessionRef.current?.promise !== pendingPromise) {
				void closeSession(session);
				return undefined;
			}
			pendingNativeSessionRef.current = null;
			nativeSessionRef.current = session;
			nativeSessionTargetKeyRef.current = cacheKey;
			setConnected(true);
			setSessionId(session.session_id);
			return session.session_id;
		} catch (error) {
			if (pendingNativeSessionRef.current?.promise === pendingPromise) {
				pendingNativeSessionRef.current = null;
			}
			setConnected(false);
			setSessionId(null);
			throw error;
		}
	}, [closeSession, target]);

	useEffect(() => {
		invalidateSession();
	}, [target?.server_id, target?.scratch_id, invalidateSession]);

	useEffect(
		() => () => {
			invalidateSession();
		},
		[invalidateSession],
	);

	return { ensureSession, invalidateSession, connected, sessionId };
}
