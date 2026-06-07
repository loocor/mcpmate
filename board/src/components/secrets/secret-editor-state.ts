import {
	inferSecretDefaultsFromOrigin,
	type SecretOriginLabelTranslate,
} from "../../lib/secret-origin-hints";
import { suggestSecretAliasFromOrigin } from "../../lib/secret-alias";
import type { SecretKind, SecretOrigin } from "../../lib/types";

export interface SecretEditorState {
	mode: "create" | "edit";
	alias: string;
	kind: SecretKind;
	label: string;
	value: string;
	origin: SecretOrigin | null;
}

export const SECRET_KIND_VALUES: SecretKind[] = [
	"generic",
	"token",
	"api_key",
	"password",
	"oauth_access_token",
	"oauth_refresh_token",
	"url_credential",
	"header_value",
];

export function defaultSecretEditorState(): SecretEditorState {
	return {
		mode: "create",
		alias: "",
		kind: "token",
		label: "",
		value: "",
		origin: null,
	};
}

export function buildCreateEditorStateFromOrigin(
	origin: SecretOrigin,
	existingAliases: Iterable<string>,
	translate?: SecretOriginLabelTranslate,
): SecretEditorState {
	const defaults = inferSecretDefaultsFromOrigin(origin, translate);
	return {
		mode: "create",
		alias: suggestSecretAliasFromOrigin(origin, existingAliases),
		kind: defaults.kind,
		label: defaults.label,
		value: "",
		origin,
	};
}

export const ORIGIN_QUERY_KEYS = [
	"server_id",
	"server_name",
	"server_kind",
	"source",
	"field_group",
	"field_key",
	"field_index",
	"field_path",
] as const;

export function originFromSearchParams(params: URLSearchParams): SecretOrigin | null {
	const origin: SecretOrigin = {};
	for (const key of ORIGIN_QUERY_KEYS) {
		const value = params.get(`origin_${key}`);
		if (!value) continue;
		if (key === "field_index") {
			const parsed = Number.parseInt(value, 10);
			if (Number.isFinite(parsed)) {
				origin.field_index = parsed;
			}
			continue;
		}
		origin[key] = value;
	}
	return Object.keys(origin).length > 0 ? origin : null;
}

export function stripOriginSearchParams(params: URLSearchParams) {
	for (const key of ORIGIN_QUERY_KEYS) {
		params.delete(`origin_${key}`);
	}
}
