import type { TFunction } from "i18next";
import type { SecretUsage } from "./types";

export function secretUsageLabel(usage: SecretUsage, t: TFunction<"secrets">): string {
	const location = usage.location;
	if (typeof location === "string") {
		const keyMap: Record<string, string> = {
			stdio_command: "stdioCommand",
			streamable_http_url: "httpUrl",
			oauth_token: "oauthToken",
		};
		return t(`usage.location.${keyMap[location] ?? location}`, {
			defaultValue: location,
		});
	}
	if ("stdio_env" in location && typeof location.stdio_env === "object") {
		return t("usage.location.stdioEnv", {
			defaultValue: "stdio env {{name}}",
			name: (location.stdio_env as { name?: string }).name ?? "",
		});
	}
	if ("stdio_argument" in location && typeof location.stdio_argument === "object") {
		return t("usage.location.stdioArgument", {
			defaultValue: "stdio arg {{index}}",
			index: (location.stdio_argument as { index?: number }).index ?? "",
		});
	}
	if (
		"streamable_http_header" in location &&
		typeof location.streamable_http_header === "object"
	) {
		return t("usage.location.httpHeader", {
			defaultValue: "http header {{name}}",
			name: (location.streamable_http_header as { name?: string }).name ?? "",
		});
	}
	if ("stdio_command" in location) {
		return t("usage.location.stdioCommand", { defaultValue: "stdio command" });
	}
	if ("streamable_http_url" in location) {
		return t("usage.location.httpUrl", { defaultValue: "http url" });
	}
	if ("oauth_token" in location) {
		return t("usage.location.oauthToken", { defaultValue: "oauth token" });
	}
	return JSON.stringify(location);
}
