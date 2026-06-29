import type { TFunction } from "i18next";
import * as z from "zod";

export type ClientConfigFileChoice = "with_config_file" | "without_config_file";
export const SUPPORTED_TRANSPORT_VALUES = ["streamable_http", "sse", "stdio"] as const;
export type SupportedTransportValue = (typeof SUPPORTED_TRANSPORT_VALUES)[number];
export const CONFIG_PARSE_FORMAT_VALUES = ["json", "json5", "toml", "yaml"] as const;
export type ConfigParseFormatValue = (typeof CONFIG_PARSE_FORMAT_VALUES)[number];
export const CONFIG_PARSE_CONTAINER_TYPE_VALUES = ["standard", "array"] as const;
export const CLIENT_IDENTIFIER_PATTERN = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

export function createClientFormSchema(t: TFunction) {
	return z
		.object({
			identifier: z.string().min(1, {
				message: t("detail.form.validation.identifierRequired", {
					defaultValue: "Client ID is required.",
				}),
			}).regex(CLIENT_IDENTIFIER_PATTERN, {
				message: t("detail.form.validation.identifierFormat", {
					defaultValue:
						"Client ID can only use lowercase English letters, numbers, and hyphens.",
				}),
			}),
			displayName: z.string().min(1, {
				message: t("detail.form.validation.displayNameRequired", {
					defaultValue: "Client name is required.",
				}),
			}),
			configFileChoice: z.enum(["with_config_file", "without_config_file"]),
			supportedTransports: z.array(z.enum(SUPPORTED_TRANSPORT_VALUES)),
			configPath: z.string().optional(),
			configFileParseFormat: z.enum(CONFIG_PARSE_FORMAT_VALUES),
			configFileParseContainerType: z.enum(CONFIG_PARSE_CONTAINER_TYPE_VALUES),
			configFileParseContainerKeysText: z.string().optional(),
			clientVersion: z.string().optional(),
			description: z.string().optional(),
			homepageUrl: z.string().optional(),
			docsUrl: z.string().optional(),
			supportUrl: z.string().optional(),
			logoUrl: z.string().optional(),
		})
		.superRefine((values, ctx) => {
			if (values.configFileChoice === "with_config_file" && values.supportedTransports.length === 0) {
				ctx.addIssue({
					code: z.ZodIssueCode.custom,
					path: ["supportedTransports"],
					message: t("detail.form.transportSupport.validation.required", {
						defaultValue: "Select at least one supported transport before saving.",
					}),
				});
			}
		});
}

export type ClientRecordFormValues = z.infer<ReturnType<typeof createClientFormSchema>>;
