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
	const [sessionData, setSessionData] = useState<InspectorSessionOpenData | null>(
		null,
	);

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
		setSessionData(null);
		if (current) {
			void closeSession(current);
		}
	}, [closeSession]);

	const ensureSessionData = useCallback(async (): Promise<
		InspectorSessionOpenData | undefined
	> => {
		const cacheKey = nativeTargetCacheKey(target);
		if (!target || !cacheKey) {
			return undefined;
		}

		const current = nativeSessionRef.current;
		if (current && nativeSessionTargetKeyRef.current === cacheKey) {
			return current;
		}

		const pending = pendingNativeSessionRef.current;
		if (pending?.targetKey === cacheKey) {
			return pending.promise;
		}

		if (current) {
			await closeSession(current);
			nativeSessionRef.current = null;
			nativeSessionTargetKeyRef.current = null;
			setConnected(false);
			setSessionData(null);
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
			setSessionData(session);
			return session;
		} catch (error) {
			if (pendingNativeSessionRef.current?.promise === pendingPromise) {
				pendingNativeSessionRef.current = null;
			}
			setConnected(false);
			setSessionId(null);
			setSessionData(null);
			throw error;
		}
	}, [closeSession, target]);

	const ensureSession = useCallback(async (): Promise<string | undefined> => {
		const session = await ensureSessionData();
		return session?.session_id;
	}, [ensureSessionData]);

	useEffect(() => {
		const cacheKey = nativeTargetCacheKey(target);
		const currentKey = nativeSessionTargetKeyRef.current;
		const pendingKey = pendingNativeSessionRef.current?.targetKey ?? null;

		if (!cacheKey) {
			if (currentKey || pendingKey) {
				invalidateSession();
			}
			return;
		}

		if (
			(currentKey && currentKey !== cacheKey) ||
			(pendingKey && pendingKey !== cacheKey)
		) {
			invalidateSession();
		}
	}, [target?.server_id, target?.scratch_id, invalidateSession]);

	useEffect(
		() => () => {
			invalidateSession();
		},
		[invalidateSession],
	);

	return {
		ensureSession,
		ensureSessionData,
		invalidateSession,
		connected,
		sessionId,
		sessionData,
	};
}
