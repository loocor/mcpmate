import type { SecretKind, SecretOrigin } from "./types";

export const OAUTH_SECRET_KINDS: SecretKind[] = [
	"oauth_client_secret",
	"oauth_access_token",
	"oauth_refresh_token",
];

/** Kinds users may pick when creating a secret from the Board UI. */
export const USER_CREATABLE_SECRET_KINDS: SecretKind[] = [
	"generic",
	"token",
	"api_key",
	"password",
	"url_credential",
	"header_value",
];

export interface SecretOriginDefaults {
	kind: SecretKind;
	label: string;
}

export type SecretOriginLabelTranslate = (
	key: string,
	defaultValue: string,
) => string;

function normalizedFieldKey(origin: SecretOrigin): string {
	return origin.field_key?.trim().toLowerCase() ?? "";
}

function keyBasedKind(key: string): SecretKind | null {
	if (!key) return null;
	if (/api[_-]?key|apikey/.test(key)) return "api_key";
	if (/pass(word|wd)?|credential/.test(key)) return "password";
	if (/\burl\b|uri|endpoint|link|address/.test(key)) return "url_credential";
	if (/token|bearer|jwt|session/.test(key)) return "token";
	if (/auth(orization)?|x-api-key/.test(key)) return "header_value";
	return null;
}

export function inferSecretKindFromOrigin(origin: SecretOrigin): SecretKind {
	const fieldGroup = origin.field_group?.trim().toLowerCase();
	const fieldKey = normalizedFieldKey(origin);
	const fromKey = keyBasedKind(fieldKey);

	if (fieldGroup === "streamable_http") {
		if (fieldKey === "url" || origin.field_path === "url") {
			return "url_credential";
		}
		return fromKey ?? "url_credential";
	}

	if (fieldGroup === "stdio") {
		if (fieldKey === "command" || origin.field_path === "command") {
			return fromKey ?? "generic";
		}
		return fromKey ?? "generic";
	}

	if (fieldGroup === "headers") {
		if (fromKey === "password") return "password";
		if (fromKey === "api_key") return "api_key";
		return "header_value";
	}

	if (fieldGroup === "url_params" || fieldGroup === "env") {
		return fromKey ?? "token";
	}

	if (fieldGroup === "args") {
		return fromKey ?? "generic";
	}

	return fromKey ?? "token";
}

function fieldGroupLabel(
	origin: SecretOrigin,
	translate: SecretOriginLabelTranslate,
): string {
	const fieldGroup = origin.field_group?.trim().toLowerCase();
	switch (fieldGroup) {
		case "url_params":
			return translate("originLabel.urlParameter", "URL parameter");
		case "env":
			return translate(
				"originLabel.environmentVariable",
				"Environment variable",
			);
		case "headers":
			return translate("originLabel.httpHeader", "HTTP header");
		case "args":
			return translate("originLabel.argument", "Argument");
		case "stdio":
			return normalizedFieldKey(origin) === "command" ||
				origin.field_path === "command"
				? translate("originLabel.command", "Command")
				: translate("originLabel.stdioField", "Stdio field");
		case "streamable_http":
			return translate("originLabel.serverUrl", "Server URL");
		default:
			return translate("originLabel.field", "Field");
	}
}

function fieldKeyLabel(origin: SecretOrigin): string {
	const fieldKey = origin.field_key?.trim();
	if (fieldKey) return fieldKey;
	return `k${(origin.field_index ?? 0) + 1}`;
}

export function inferSecretLabelFromOrigin(
	origin: SecretOrigin,
	translate?: SecretOriginLabelTranslate,
): string {
	const t = translate ?? ((_, defaultValue) => defaultValue);
	const serverName =
		origin.server_name?.trim() ||
		origin.server_id?.trim() ||
		t("originLabel.serverFallback", "Server");
	return `${serverName} · ${fieldGroupLabel(origin, t)} · ${fieldKeyLabel(origin)}`;
}

export function inferSecretDefaultsFromOrigin(
	origin: SecretOrigin,
	translate?: SecretOriginLabelTranslate,
): SecretOriginDefaults {
	return {
		kind: inferSecretKindFromOrigin(origin),
		label: inferSecretLabelFromOrigin(origin, translate),
	};
}

export function isUserCreatableSecretKind(kind: SecretKind): boolean {
	return USER_CREATABLE_SECRET_KINDS.includes(kind);
}

export function kindOptionsForEditor(
	allOptions: Array<{ value: SecretKind; label: string }>,
	editor: { mode: "create" | "edit"; kind: SecretKind } | null,
): Array<{ value: SecretKind; label: string }> {
	if (!editor) return allOptions;
	if (editor.mode === "edit" && OAUTH_SECRET_KINDS.includes(editor.kind)) {
		return allOptions;
	}
	return allOptions.filter((option) =>
		isUserCreatableSecretKind(option.value),
	);
}
