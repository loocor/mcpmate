import { useCallback, useEffect } from "react";
import type { ManualServerFormValues, ManualFormStateJson } from "../types";
import { cloneArgs, cloneKeyValuePairs } from "../types";

interface UseFormSyncProps {
	formStateRef: React.MutableRefObject<ManualFormStateJson>;
	isRestoringRef: React.MutableRefObject<boolean>;
	kind: ManualServerFormValues["kind"];
	watchedName: string | undefined;
	watchedMetaDescription: string | undefined;
	watchedMetaVersion: string | undefined;
	watchedMetaWebsite: string | undefined;
	watchedMetaRepositoryUrl: string | undefined;
	watchedMetaRepositorySource: string | undefined;
	watchedMetaRepositorySubfolder: string | undefined;
	watchedMetaRepositoryId: string | undefined;
	watchedCommand: string | undefined;
	watchedUrl: string | undefined;
	watchedArgs: Array<{ value?: string }> | undefined;
	watchedEnv: Array<{ key?: string; value?: string }> | undefined;
	watchedHeaders: Array<{ key?: string; value?: string }> | undefined;
	getValues: () => ManualServerFormValues;
	reset: (
		values?: ManualServerFormValues,
		options?: {
			keepDirty?: boolean;
			keepTouched?: boolean;
			keepIsSubmitted?: boolean;
			keepErrors?: boolean;
			keepSubmitCount?: boolean;
		},
	) => void;
	buildFormValuesFromState: (
		state: ManualFormStateJson,
	) => ManualServerFormValues;
}

export function useFormSync({
	formStateRef,
	isRestoringRef,
	kind,
	watchedName,
	watchedMetaDescription,
	watchedMetaVersion,
	watchedMetaWebsite,
	watchedMetaRepositoryUrl,
	watchedMetaRepositorySource,
	watchedMetaRepositorySubfolder,
	watchedMetaRepositoryId,
	watchedCommand,
	watchedUrl,
	watchedArgs,
	watchedEnv,
	watchedHeaders,
	getValues,
	reset,
	buildFormValuesFromState,
}: UseFormSyncProps) {
	// Batch update common fields to reduce re-renders
	useEffect(() => {
		if (isRestoringRef.current) return;
		formStateRef.current.name = watchedName || "";
		formStateRef.current.meta.description = watchedMetaDescription || "";
		formStateRef.current.meta.version = watchedMetaVersion || "";
		formStateRef.current.meta.websiteUrl = watchedMetaWebsite || "";
	}, [watchedName, watchedMetaDescription, watchedMetaVersion, watchedMetaWebsite, formStateRef, isRestoringRef]);

	useEffect(() => {
		if (isRestoringRef.current) return;
		const repository = formStateRef.current.meta.repository;
		repository.url = watchedMetaRepositoryUrl || "";
		repository.source = watchedMetaRepositorySource || "";
		repository.subfolder = watchedMetaRepositorySubfolder || "";
		repository.id = watchedMetaRepositoryId || "";
	}, [
		watchedMetaRepositoryUrl,
		watchedMetaRepositorySource,
		watchedMetaRepositorySubfolder,
		watchedMetaRepositoryId,
		formStateRef,
		isRestoringRef,
	]);

	useEffect(() => {
		if (isRestoringRef.current) return;
		formStateRef.current.kind = kind;
	}, [kind, formStateRef, isRestoringRef]);

	// Batch update stdio fields
	useEffect(() => {
		if (isRestoringRef.current || kind !== "stdio") return;
		formStateRef.current.stdio.command = watchedCommand || "";
		formStateRef.current.stdio.args = cloneArgs(watchedArgs || []);
		formStateRef.current.stdio.env = cloneKeyValuePairs(watchedEnv || []);
	}, [kind, watchedCommand, watchedArgs, watchedEnv, formStateRef, isRestoringRef]);

	// Batch update HTTP-based transport fields (sse & streamable_http)
	useEffect(() => {
		if (isRestoringRef.current) return;
		if (kind === "sse") {
			formStateRef.current.sse.url = watchedUrl || "";
			formStateRef.current.sse.headers = cloneKeyValuePairs(watchedHeaders || []);
		} else if (kind === "streamable_http") {
			formStateRef.current.streamable_http.url = watchedUrl || "";
			formStateRef.current.streamable_http.headers = cloneKeyValuePairs(watchedHeaders || []);
		}
	}, [kind, watchedUrl, watchedHeaders, formStateRef, isRestoringRef]);

	const saveTypeSnapshot = useCallback(
		(currentKind: ManualServerFormValues["kind"]) => {
			const values = getValues();
			formStateRef.current.name = values.name ?? "";
			formStateRef.current.meta.description = values.meta_description ?? "";
			formStateRef.current.meta.version = values.meta_version ?? "";
			formStateRef.current.meta.websiteUrl = values.meta_website_url ?? "";
			formStateRef.current.meta.repository = {
				url: values.meta_repository_url ?? "",
				source: values.meta_repository_source ?? "",
				subfolder: values.meta_repository_subfolder ?? "",
				id: values.meta_repository_id ?? "",
			};

			if (currentKind === "stdio") {
				formStateRef.current.stdio = {
					command: values.command ?? "",
					args: cloneArgs(values.args),
					env: cloneKeyValuePairs(values.env),
				};
			} else if (currentKind === "sse") {
				formStateRef.current.sse = {
					url: values.url ?? "",
					headers: cloneKeyValuePairs(values.headers),
					urlParams: cloneKeyValuePairs(values.urlParams),
				};
			} else {
				formStateRef.current.streamable_http = {
					url: values.url ?? "",
					headers: cloneKeyValuePairs(values.headers),
					urlParams: cloneKeyValuePairs(values.urlParams),
				};
			}

			formStateRef.current.kind = currentKind;
		},
		[getValues, formStateRef],
	);

	const restoreTypeSnapshot = useCallback(
		(targetKind: ManualServerFormValues["kind"]) => {
			const state = formStateRef.current;
			state.kind = targetKind;
			isRestoringRef.current = true;
			reset(buildFormValuesFromState(state), {
				keepDirty: true,
				keepTouched: true,
				keepIsSubmitted: true,
				keepErrors: true,
				keepSubmitCount: true,
			});
			isRestoringRef.current = false;
		},
		[buildFormValuesFromState, reset, formStateRef, isRestoringRef],
	);

	return {
		saveTypeSnapshot,
		restoreTypeSnapshot,
	};
}
