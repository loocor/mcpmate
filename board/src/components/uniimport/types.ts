import { z } from "zod";
import type { ServerInstallDraft } from "../../hooks/use-server-install-pipeline";
import type { SegmentOption } from "../ui/segment";

// Constants
export const DEFAULT_INGEST_MESSAGE =
	"Drop JSON/TOML/Text or MCP bundles (WIP) to begin";

// Server type options for Segment component
export const SERVER_TYPE_OPTIONS: SegmentOption[] = [
	{ value: "stdio", label: "Stdio" },
	{ value: "sse", label: "SSE" },
	{ value: "streamable_http", label: "Streamable HTTP" },
];

// Breathing animation styles
export const breathingAnimation = `
@keyframes breathing {
  0%, 100% {
    transform: scale(1);
    opacity: 1;
  }
  50% {
    transform: scale(1.1);
    opacity: 0.8;
  }
}
`;

// Zod schemas
export const argSchema = z.object({
	value: z.string().optional(),
});

export const envSchema = z.object({
	key: z.string().optional(),
	value: z.string().optional(),
});

export const headerSchema = z.object({
	key: z.string().optional(),
	value: z.string().optional(),
});

export const urlParamSchema = z.object({
	key: z.string().optional(),
	value: z.string().optional(),
});

export const manualServerSchema = z
	.object({
		name: z.string().min(1, "manual.errors.nameRequired"),
		kind: z.enum(["stdio", "sse", "streamable_http"], {
			required_error: "manual.errors.kindRequired",
		}),
		command: z.string().optional(),
		url: z
			.string()
			.url("manual.errors.urlInvalid")
			.optional()
			.or(z.literal("")),
		args: z.array(argSchema).optional(),
		env: z.array(envSchema).optional(),
		headers: z.array(headerSchema).optional(),
		urlParams: z.array(urlParamSchema).optional(),
		meta_description: z.string().optional(),
		meta_version: z.string().optional(),
		meta_website_url: z
			.string()
			.url("manual.errors.urlInvalid")
			.optional()
			.or(z.literal("")),
		meta_repository_url: z
			.string()
			.url("manual.errors.urlInvalid")
			.optional()
			.or(z.literal("")),
		meta_repository_source: z.string().optional(),
		meta_repository_subfolder: z.string().optional(),
		meta_repository_id: z.string().optional(),
	})
	.superRefine((value, ctx) => {
		const kind = value.kind;
		const command = value.command?.trim();
		const url = value.url?.trim();
		if (kind === "stdio" && !command) {
			ctx.addIssue({
				code: z.ZodIssueCode.custom,
				message: "manual.errors.commandRequired",
				path: ["command"],
			});
		}
		if (kind !== "stdio" && !url) {
			ctx.addIssue({
				code: z.ZodIssueCode.custom,
				message: "manual.errors.urlRequired",
				path: ["url"],
			});
		}
	});

// Type definitions
export type ManualServerFormValues = z.infer<typeof manualServerSchema>;

export interface KeyValuePair {
	key: string;
	value: string;
}

export interface StdioState {
	command: string;
	args: Array<{ value: string }>;
	env: KeyValuePair[];
}

export interface HttpState {
	url: string;
	headers: KeyValuePair[];
	urlParams?: KeyValuePair[];
}

export interface IconState {
	src: string;
	mimeType?: string;
	sizes?: string;
}

export interface ManualFormStateJson {
	name: string;
	kind: ManualServerFormValues["kind"];
	meta: {
		description: string;
		version: string;
		websiteUrl: string;
		repository: {
			url: string;
			source: string;
			subfolder: string;
			id: string;
		};
		icons: IconState[];
	};
	stdio: StdioState;
	sse: HttpState;
	streamable_http: HttpState;
}

export interface ServerInstallManualFormHandle {
	ingest: (payload: {
		text?: string;
		buffer?: ArrayBuffer;
		fileName?: string;
	}) => Promise<void>;
	loadDraft: (draft: ServerInstallDraft) => Promise<void> | void;
	getCurrentDraft: () => ServerInstallDraft | null;
}

export interface ServerInstallManualFormProps {
	isOpen: boolean;
	onClose: () => void;
	onSubmit: (draft: ServerInstallDraft) => Promise<void> | void;
	onSubmitMultiple?: (drafts: ServerInstallDraft[]) => Promise<void> | void;
	/**
	 * Controls whether the form behaves as a creation flow, edit experience, or market import.
	 * Edit mode disables ingest-only UX like drag & drop and forces JSON view to be read-only.
	 * Market mode shows server information and transport options for registry imports.
	 */
	mode?: "create" | "edit" | "market";
	/** Optional initial draft used to hydrate the form when editing an existing server. */
	initialDraft?: ServerInstallDraft | null;
	/** Allow users to modify the JSON representation. Defaults to true in create mode. */
	allowJsonEditing?: boolean;
	/** Market mode specific props */
	onPreview?: () => void;
	onImport?: () => void;
	/** Allow programmatic ingestion even when ingest UI is disabled (e.g., market mode) */
	allowProgrammaticIngest?: boolean;
}

// Utility functions
export const cloneArgs = (
	items?: ManualServerFormValues["args"] | Array<{ value?: string }>,
): Array<{ value: string }> =>
	Array.isArray(items)
		? items.map((item) => ({ value: item?.value ?? "" }))
		: [];

export const cloneKeyValuePairs = (
	items?:
		| ManualServerFormValues["env"]
		| ManualServerFormValues["headers"]
		| Array<{ key?: string; value?: string }>,
): KeyValuePair[] =>
	Array.isArray(items)
		? items.map((item) => ({
				key: item?.key ?? "",
				value: item?.value ?? "",
			}))
		: [];
