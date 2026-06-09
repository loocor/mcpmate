import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { ServerInstallDraft } from "../../../hooks/use-server-install-pipeline";
import { SERVER_TYPE_OPTIONS } from "../types";

export function useServerTypeOptions() {
	const { t, i18n } = useTranslation("servers");

	const serverTypeOptions = useMemo(
		() =>
			SERVER_TYPE_OPTIONS.map((option) => ({
				...option,
				label: t(`manual.fields.type.options.${option.value}`, {
					defaultValue: option.label,
				}),
			})),
		[t, i18n.language],
	);

	const transportLabel = useMemo(
		(): Record<ServerInstallDraft["kind"], string> => ({
			stdio: t("manual.fields.type.options.stdio", { defaultValue: "Stdio" }),
			sse: t("manual.fields.type.options.sse", {
				defaultValue: "SSE (Legacy)",
			}),
			streamable_http: t("manual.fields.type.options.streamable_http", {
				defaultValue: "Streamable HTTP",
			}),
		}),
		[t, i18n.language],
	);

	return { serverTypeOptions, transportLabel };
}
