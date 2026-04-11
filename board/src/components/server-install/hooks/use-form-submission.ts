import { useCallback } from "react";
import type { ServerInstallDraft } from "../../../hooks/use-server-install-pipeline";
import { parseJsonDrafts } from "../../../lib/install-normalizer";
import { notifyError } from "../../../lib/notify";
import type { ManualServerFormValues } from "../types";

interface UseFormSubmissionProps {
	isEditMode: boolean;
	isMarketMode: boolean;
	onSubmit: (draft: ServerInstallDraft) => Promise<void> | void;
	onClose: () => void;
	reset: () => void;
	viewMode: "form" | "json";
	jsonText: string;
	jsonEditingEnabled: boolean;
	setJsonError: (error: string | null) => void;
	setViewMode: (mode: "form" | "json") => void;
	messages: {
		commandRequiredTitle: string;
		commandRequiredBody: string;
		endpointRequiredTitle: string;
		endpointRequiredBody: string;
		jsonNoServers: string;
		jsonMultipleServers: string;
		jsonParseFailedTitle: string;
		jsonParseFailedFallback: string;
		invalidJsonTitle: string;
		submit: {
			edit: string;
			market: string;
			create: string;
		};
		pending: {
			edit: string;
			market: string;
			create: string;
		};
	};
}

export function useFormSubmission({
	isEditMode,
	isMarketMode,
	onSubmit,
	onClose,
	reset,
	viewMode: _viewMode,
	jsonText,
	jsonEditingEnabled: _jsonEditingEnabled,
	setJsonError,
	setViewMode,
	messages,
}: UseFormSubmissionProps) {
	const buildDraftFromValues = useCallback(
		(values: ManualServerFormValues): ServerInstallDraft => {
			const trim = (value?: string | null) => {
				if (value == null) return undefined;
				const next = value.trim();
				return next.length > 0 ? next : undefined;
			};

			const args = (values.args ?? [])
				.map((item) => trim(item.value))
				.filter((value): value is string => Boolean(value));

			const envEntries = (values.env ?? [])
				.map((entry) => {
					const key = trim(entry.key);
					const value = trim(entry.value);
					return key ? { key, value: value ?? "" } : null;
				})
				.filter((entry): entry is { key: string; value: string } =>
					Boolean(entry),
				);

			const env = envEntries.length
				? envEntries.reduce<Record<string, string>>((acc, entry) => {
						acc[entry.key] = entry.value;
						return acc;
					}, {})
				: undefined;

			const headerEntries = (values.headers ?? [])
				.map((entry) => {
					const key = trim(entry.key);
					const value = trim(entry.value);
					return key ? { key, value: value ?? "" } : null;
				})
				.filter((entry): entry is { key: string; value: string } =>
					Boolean(entry),
				);

			const headers = headerEntries.length
				? headerEntries.reduce<Record<string, string>>((acc, entry) => {
						acc[entry.key] = entry.value;
						return acc;
					}, {})
				: undefined;

			// URL parameters (HTTP kinds only)
			const urlParamEntries = (values.urlParams ?? [])
				.map((entry) => {
					const key = trim(entry.key);
					const value = trim(entry.value);
					return key ? { key, value: value ?? "" } : null;
				})
				.filter((entry): entry is { key: string; value: string } =>
					Boolean(entry),
				);

			const urlParams = urlParamEntries.length
				? urlParamEntries.reduce<Record<string, string>>((acc, entry) => {
						acc[entry.key] = entry.value;
						return acc;
					}, {})
				: undefined;

			const repository = (() => {
				const payload: any = {};
				const url = trim(values.meta_repository_url);
				const source = trim(values.meta_repository_source);
				const subfolder = trim(values.meta_repository_subfolder);
				const id = trim(values.meta_repository_id);
				if (url) payload.url = url;
				if (source) payload.source = source;
				if (subfolder) payload.subfolder = subfolder;
				if (id) payload.id = id;
				return Object.keys(payload).length ? payload : undefined;
			})();

			const meta: any = {};
			const description = trim(values.meta_description);
			const version = trim(values.meta_version);
			const websiteUrl = trim(values.meta_website_url);
			if (description) meta.description = description;
			if (version) meta.version = version;
			if (websiteUrl) meta.websiteUrl = websiteUrl;
			if (repository) meta.repository = repository;

			const envForDraft = values.kind === "stdio" ? env : headers;

			return {
				name: values.name.trim(),
				kind: values.kind,
				command: values.kind === "stdio" ? trim(values.command) : undefined,
				url: values.kind === "stdio" ? undefined : trim(values.url),
				args: values.kind === "stdio" && args.length ? args : undefined,
				env: envForDraft,
				headers: values.kind !== "stdio" ? headers : undefined,
				...(values.kind !== "stdio" && urlParams ? { urlParams } : {}),
				meta: Object.keys(meta).length ? meta : undefined,
			};
		},
		[],
	);

	const submitForm = useCallback(
		async (values: ManualServerFormValues) => {
			const draft = buildDraftFromValues(values);

			if (draft.kind === "stdio" && !draft.command) {
				notifyError(
					messages.commandRequiredTitle,
					messages.commandRequiredBody,
				);
				return;
			}
			if (draft.kind !== "stdio" && !draft.url) {
				notifyError(
					messages.endpointRequiredTitle,
					messages.endpointRequiredBody,
				);
				return;
			}

			await onSubmit(draft);
			onClose();
			reset();
		},
		[buildDraftFromValues, onSubmit, onClose, reset],
	);

	const submitJson = useCallback(async () => {
		try {
			const drafts = parseJsonDrafts(jsonText);
			if (!drafts.length) {
				setJsonError(messages.jsonNoServers);
				return;
			}
			if (drafts.length > 1) {
				setJsonError(messages.jsonMultipleServers);
				return;
			}
			setJsonError(null);
			await onSubmit(drafts[0]);
			onClose();
			reset();
			setViewMode("form");
		} catch (error) {
			const message =
				error instanceof Error
					? error.message
					: messages.jsonParseFailedFallback;
			setJsonError(message);
			notifyError(messages.invalidJsonTitle, message);
		}
	}, [jsonText, onSubmit, onClose, reset, setViewMode, setJsonError]);

	const submitButtonLabel = isEditMode
		? messages.submit.edit
		: isMarketMode
			? messages.submit.market
			: messages.submit.create;
	const pendingButtonLabel = isEditMode
		? messages.pending.edit
		: isMarketMode
			? messages.pending.market
			: messages.pending.create;

	return {
		buildDraftFromValues,
		submitForm,
		submitJson,
		submitButtonLabel,
		pendingButtonLabel,
	};
}
