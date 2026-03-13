// API Response Types
export interface ServerIcon {
	src: string;
	mimeType?: string | null;
	sizes?: string | null;
}

export interface RegistryRepositoryInfo {
	url?: string | null;
	source?: string | null;
	subfolder?: string | null;
	id?: string | null;
}

export interface RegistryOfficialMeta {
	status?: string | null;
	publishedAt?: string | null;
	updatedAt?: string | null;
	isLatest?: boolean | null;
}

export interface RegistryMetaPayload {
	"io.modelcontextprotocol.registry/official"?: RegistryOfficialMeta | null;
	"io.modelcontextprotocol.registry/publisher-provided"?: Record<
		string,
		unknown
	> | null;
	[namespace: string]: unknown;
}

export interface ServerCapabilitySummary {
	supports_tools: boolean;
	supports_prompts: boolean;
	supports_resources: boolean;
	tools_count: number;
	prompts_count: number;
	resources_count: number;
	resource_templates_count: number;
}

export interface ServerMetaInfo {
	description?: string | null;
	version?: string | null;
	websiteUrl?: string | null;
	repository?: RegistryRepositoryInfo | null;
	_meta?: RegistryMetaPayload | null;
	extras?: Record<string, unknown> | null;
	icons?: ServerIcon[];
}

export interface ServerSummary {
	id: string;
	name: string;
	server_type?: string;
	status: string;
	enabled?: boolean;
	globally_enabled?: boolean;
	enabled_in_suits?: boolean;
	enabled_in_profile?: boolean;
	registry_server_id?: string | null;
	instance_count?: number;
	instances?: InstanceSummary[];
	meta?: ServerMetaInfo;
	icons?: ServerIcon[];
	capability?: ServerCapabilitySummary;
	capabilities?: ServerCapabilitySummary;
	protocol_version?: string | null;
	server_version?: string | null;
	created_at?: string | null;
	updated_at?: string | null;
}

export interface ServerListResponse {
	servers: ServerSummary[];
}

export interface InstanceSummary {
	id: string;
	name: string;
	status: string;
	startTime?: string;
	started_at?: string;
	startedAt?: string;
	lastResponseAt?: unknown;
}

export interface ServerDetail extends ServerSummary {
	command?: string;
	commandPath?: string;
	args?: string[];
	env?: Record<string, string>;
	url?: string | null;
	headers?: Record<string, string> | null;
	instances: InstanceSummary[];
}

export interface RegistryTransportHeader {
	name: string;
	description?: string;
	isRequired?: boolean;
	isSecret?: boolean;
}

export interface RegistryTransport {
	type: string;
	url?: string;
	headers?: RegistryTransportHeader[] | null;
}

export interface RegistryPackageArgument {
	name: string;
	description?: string;
	type?: string;
	isRequired?: boolean;
	default?: string;
	valueHint?: string;
}

export interface RegistryPackage {
	registryType?: string;
	registryBaseUrl?: string;
	identifier?: string;
	version?: string;
	transport?: { type: string };
	environmentVariables?: RegistryTransportHeader[] | null;
	packageArguments?: RegistryPackageArgument[] | null;
	runtimeArguments?: RegistryPackageArgument[] | null;
}

export interface RegistryOfficialMeta {
	serverId: string;
	versionId: string;
	publishedAt: string;
	updatedAt?: string;
	isLatest?: boolean;
}

export interface RegistryServerMeta {
	"io.modelcontextprotocol.registry/official"?: RegistryOfficialMeta;
	"io.modelcontextprotocol.registry/publisher-provided"?: Record<
		string,
		unknown
	>;
	[key: string]: unknown;
}

export interface RegistryServerEntry {
	name: string;
	description?: string;
	version: string;
	status?: string;
	repository?: { url?: string; source?: string; subfolder?: string };
	websiteUrl?: string;
	remotes?: RegistryTransport[] | null;
	packages?: RegistryPackage[] | null;
	_meta?: RegistryServerMeta;
	$schema?: string;
}

export interface RegistryServerListResponse {
	servers: RegistryServerEntryWrapper[];
	metadata: {
		nextCursor?: string;
		count: number;
	};
}

export interface RegistryServerEntryWrapper {
	server: RegistryServerEntry;
	_meta?: RegistryServerMeta;
}

export interface ServersImportData {
	imported_count: number;
	imported_servers: string[];
	skipped_count: number;
	skipped_servers: SkippedServer[];
	failed_count: number;
	failed_servers: string[];
	error_details?: Record<string, string> | null;
}

export interface SkippedServer {
	name: string;
	reason: string;
	existing_query?: string | null;
	incoming_query?: string | null;
}

export interface ServerInstanceDetails {
	connection_attempts: number;
	last_connected_seconds?: number;
	tools_count: number;
	error_message?: string;
	server_type: string;
	process_id?: number;
	cpu_usage?: number;
	memory_usage?: number;
	last_health_check?: string;
}

export interface InspectorToolCallStartData {
	call_id: string;
	server_id: string;
	mode: "proxy" | "native";
	session_id?: string;
	request_id: string;
	progress_token: string;
}

export interface InspectorSessionOpenData {
	session_id: string;
	server_id: string;
	mode: "proxy" | "native";
	expires_at_epoch_ms: number;
}

export interface InspectorSessionCloseData {
	closed: boolean;
}

export interface InspectorToolCallCancelData {
	cancelled: boolean;
}

export type InspectorSseEvent =
	| {
			event: "started";
			call_id: string;
			server_id: string;
			mode: "proxy" | "native";
			session_id?: string;
			started_at_epoch_ms: number;
	  }
	| {
			event: "progress";
			call_id: string;
			progress: number;
			total?: number;
			message?: string;
	  }
	| {
			event: "log";
			call_id: string;
			level?: string;
			logger?: string;
			data: unknown;
	  }
	| {
			event: "result";
			call_id: string;
			server_id: string;
			elapsed_ms: number;
			result: unknown;
	  }
	| {
			event: "error";
			call_id: string;
			server_id: string;
			message: string;
	  }
	| {
			event: "cancelled";
			call_id: string;
			server_id: string;
			reason?: string;
	  };

export interface InstanceDetail {
	id: string;
	name: string;
	server_name: string;
	status: string;
	allowed_operations: string[];
	details: ServerInstanceDetails;
}

export interface InstanceHealth {
	id: string;
	name: string;
	healthy: boolean;
	message: string;
	status: string;
	checked_at: string;
	resource_metrics?: {
		cpu_usage?: number;
		memory_usage?: number;
		process_id?: number;
	};
	connection_stability?: number;
	details?: string;
}

export interface OperationResponse {
	id: string;
	name: string;
	result: string;
	status: string;
	allowed_operations: string[];
}

export interface Tool {
	tool_name: string;
	server_name: string;
	is_enabled: boolean;
	description?: string;
	tool_id?: string;
}

export interface ToolDetail extends Tool {
	configuration: Record<string, unknown>;
}

export interface SystemStatus {
	status: "running" | "degraded" | "stopped" | "error";
	version: string;
	uptime: number;
	active_mcp_servers: number;
	aggregated_tools_count: number;
}

export interface SystemMetrics {
	cpu_usage_percent?: number;
	cpu_usage?: number;
	memory_usage_bytes?: number;
	memory_usage?: number;
	memory_usage_percent?: number;
	active_connections?: number;
	total_requests_mcp?: number;
	error_rate_mcp?: number;
	uptime_seconds?: number;
	timestamp?: string;
	connected_servers_count?: number;
	total_instances_count?: number;
	ready_instances_count?: number;
	error_instances_count?: number;
	initializing_instances_count?: number;
	busy_instances_count?: number;
	shutdown_instances_count?: number;
	system_cpu_usage?: number;
	system_memory_usage?: number;
	system_memory_total?: number;
	system_memory_usage_percent?: number;
	total_tools_count?: number;
	unique_tools_count?: number;
	config_application_status?: string | null;
}

// API Response Status
export type ApiResponse<T> = {
	status: string;
	message?: string;
	data?: T;
};

// Notification Types
export interface ToolsChangedNotification {
	event: "tools_list_changed";
}

// UI Mode Type
export type UIMode = "wizard" | "expert";

// Theme Type
export type Theme = "light" | "dark" | "system";

// Configuration Types
export interface ConfigPreset {
	id: string;
	name: string;
	description?: string;
	created_at: string;
	updated_at: string;
	is_active: boolean;
	config: MCPConfig;
}

export interface MCPConfig {
	servers: MCPServerConfig[];
	tools: MCPToolConfig[];
	global_settings: GlobalSettings;
}

/**
 * MCP服务器配置接口
 *
 * 服务器类型必须严格使用以下标准格式：
 * - "stdio": 标准输入输出服务器
 * - "sse": 服务器发送事件服务器
 * - "streamable_http": 流式HTTP服务器
 */
export interface MCPServerConfig {
	/** 服务器名称 */
	name: string;

	/**
	 * 服务器类型
	 *
	 * 严格格式要求：只接受 "stdio" | "sse" | "streamable_http"
	 * 不接受任何变体格式
	 */
	kind: "stdio" | "sse" | "streamable_http";

	/** 启动命令（stdio类型必填） */
	command?: string;

	/** 命令路径 */
	command_path?: string;

	/** HTTP/SSE endpoint URL (non-stdio servers) */
	url?: string;

	/** 命令参数 */
	args?: string[];

	/** 环境变量 */
	env?: Record<string, string>;

	/** HTTP headers for non-stdio servers */
	headers?: Record<string, string>;

	/** 最大实例数 */
	max_instances?: number;

	/** 重试策略 */
	retry_policy?: RetryPolicy;

	/** 服务器元数据信息 */
	meta?: ServerMetaInfo;
}

export interface MCPToolConfig {
	name: string;
	server_name: string;
	is_enabled: boolean; // Keep this field name for consistency with API
	settings?: Record<string, unknown>;
}

export interface GlobalSettings {
	max_concurrent_connections: number;
	request_timeout_ms: number;
	enable_metrics: boolean;
	log_level: "debug" | "info" | "warn" | "error";
}

export interface RetryPolicy {
	max_attempts: number;
	initial_delay_ms: number;
	max_delay_ms: number;
	backoff_multiplier: number;
}

/**
 * 验证服务器类型格式
 *
 * @param kind 服务器类型字符串
 * @returns 是否为有效的标准格式
 */
export function validateServerType(
	kind: string,
): kind is "stdio" | "sse" | "streamable_http" {
	const validTypes = ["stdio", "sse", "streamable_http"] as const;
	return validTypes.includes(kind as any);
}

/**
 * 获取服务器类型格式错误提示
 *
 * @param invalidKind 无效的服务器类型
 * @returns 详细的错误提示信息
 */
export function getServerTypeErrorMessage(invalidKind: string): string {
	return `无效的服务器类型 '${invalidKind}'。

正确的格式要求：
- 使用 "stdio" (不是 "Stdio" 或其他变体)
- 使用 "sse" (不是 "SSE" 或其他变体)
- 使用 "streamable_http" (不是 "http"、"streamable-http" 或 "streamableHttp")

请检查您的输入并使用正确的标准格式。`;
}

// Config Suits Types
export interface ConfigSuit {
	id: string;
	name: string;
	description?: string;
	suit_type: string;
	multi_select: boolean;
	priority: number;
	is_active: boolean;
	is_default: boolean;
	role?: string;
	allowed_operations: string[];
}

export interface ConfigSuitListResponse {
	suits: ConfigSuit[];
}

export interface CreateConfigSuitRequest {
	name: string;
	description?: string;
	suit_type: string;
	multi_select?: boolean;
	priority?: number;
	is_active?: boolean;
	is_default?: boolean;
	clone_from_id?: string;
}

export interface UpdateConfigSuitRequest {
	name?: string;
	description?: string;
	suit_type?: string;
	multi_select?: boolean;
	priority?: number;
	is_active?: boolean;
	is_default?: boolean;
}

export interface ConfigSuitServer {
	id: string;
	name: string;
	enabled: boolean;
	allowed_operations: string[];
}

export interface ConfigSuitServersResponse {
	suit_id: string;
	suit_name: string;
	servers: ConfigSuitServer[];
}

export interface ConfigSuitTool {
	id: string;
	server_id: string;
	server_name: string;
	tool_name: string;
	unique_name?: string;
	enabled: boolean;
	allowed_operations: string[];
}

export interface ConfigSuitToolsResponse {
	suit_id: string;
	suit_name: string;
	tools: ConfigSuitTool[];
}

export interface ConfigSuitResource {
	id: string;
	server_id: string;
	server_name: string;
	resource_uri: string;
	enabled: boolean;
	allowed_operations: string[];
}

export interface ConfigSuitResourcesResponse {
    suit_id: string;
    suit_name: string;
    resources: ConfigSuitResource[];
}

export interface ConfigSuitResourceTemplate {
    id: string;
    server_id: string;
    server_name: string;
    uri_template: string;
    enabled: boolean;
    allowed_operations: string[];
}

export interface ConfigSuitResourceTemplatesResponse {
    suit_id: string;
    suit_name: string;
    templates: ConfigSuitResourceTemplate[];
}

export interface ConfigSuitPrompt {
	id: string;
	server_id: string;
	server_name: string;
	prompt_name: string;
	enabled: boolean;
	allowed_operations: string[];
}

export interface ConfigSuitPromptsResponse {
	suit_id: string;
	suit_name: string;
	prompts: ConfigSuitPrompt[];
}

export interface BatchOperationRequest {
	ids: string[];
}

export interface BatchOperationResponse {
	success_count: number;
	successful_ids: string[];
	failed_ids: Record<string, string>;
}

// Runtime Types
export interface RuntimeStatusItem {
	runtime_type: string;
	available: boolean;
	path?: string | null;
	version?: string | null;
	message: string;
}

export interface RuntimeStatusResponse {
	uv: RuntimeStatusItem;
	bun: RuntimeStatusItem;
}

export interface RuntimeCacheItem {
	path: string;
	size_bytes: number;
	package_count: number;
	last_modified?: string | null;
}

export interface RuntimeCacheSummaryInfo {
	total_size_bytes: number;
	last_cleanup?: string | null;
}

export interface RuntimeCacheResponse {
	summary: RuntimeCacheSummaryInfo;
	uv: RuntimeCacheItem;
	bun: RuntimeCacheItem;
}

export interface InstallRuntimeRequest {
	runtime_type: string; // "uv" | "bun"
	version?: string;
	timeout?: number;
	max_retries?: number;
	verbose?: boolean;
	interactive?: boolean;
}

export interface InstallResponse {
	success: boolean;
	message: string;
	runtime_type: string;
}

export interface ClearCacheResponse {
	success: boolean;
}

// Capabilities Cache Types
export interface CapabilitiesStorageTables {
	servers: number;
	tools: number;
	resources: number;
	prompts: number;
	resourceTemplates: number;
}

export interface CapabilitiesStorageStats {
	db_path: string;
	cache_size_bytes: number;
	tables: CapabilitiesStorageTables;
	last_cleanup?: string | null;
}

export interface CapabilitiesMetricsStats {
	totalQueries: number;
	cacheHits: number;
	cacheMisses: number;
	hitRatio: number;
	readOperations: number;
	writeOperations: number;
	cacheInvalidations: number;
}

export interface CapabilitiesStatsResponse {
	storage: CapabilitiesStorageStats;
	metrics: CapabilitiesMetricsStats;
	generatedAt: string;
}

export interface CapabilitiesKeyItem {
	key: string;
	serverId: string;
	approxValueSizeBytes: number;
	cachedAt?: string;
}

export interface CapabilitiesKeysResponse {
	keys: CapabilitiesKeyItem[];
	total: number;
}

// --------------------
// OpenAPI Wrapped Responses (RPC-style)
// --------------------

export interface ServerListResp {
	data?: { servers: ServerDetail[] } | null;
	error?: unknown | null;
	success: boolean;
}

export interface ServerDetailsResp {
	data?: ServerDetail | null;
	error?: unknown | null;
	success: boolean;
}

export interface InstanceListResp {
	data?: { name: string; instances: InstanceSummary[] } | null;
	error?: unknown | null;
	success: boolean;
}

export interface InstanceDetailsResp {
	data?: InstanceDetail | null;
	error?: unknown | null;
	success: boolean;
}

export interface InstanceHealthResp {
	data?: InstanceHealth | null;
	error?: unknown | null;
	success: boolean;
}

export interface OperationResponseResp {
	data?: OperationResponse | null;
	error?: unknown | null;
	success: boolean;
}

// Server capabilities (per-server) minimal typings
export interface ServerCapabilityMeta {
	cache_hit: boolean;
	source: string;
	strategy: string;
}

export interface ServerCapabilityList<T = any> {
	items: T[];
	state: string;
	meta: ServerCapabilityMeta;
}

export interface ServerCapabilityResp<T = any> {
	data?: ServerCapabilityList<T> | null;
	error?: unknown | null;
	success: boolean;
}

// --------------------
// Clients (Host applications) Types
// --------------------

export type ClientCategory = "editor" | "terminal" | "browser" | string;

export type ClientConfigType = "json" | "yaml" | "toml" | string;

export interface ClientTemplateStorageMetadata {
	kind: string;
	path_strategy?: string | null;
}

export interface ClientTemplateMetadata {
	format: string;
	container_type: ClientConfigType;
	merge_strategy: string;
	keep_original_config: boolean;
	storage: ClientTemplateStorageMetadata;
	description?: string | null;
	homepage_url?: string | null;
	docs_url?: string | null;
	support_url?: string | null;
	protocol_revision?: string | null;
	managed_source?: string | null;
	// Legacy fields that may still surface from cached payloads
	name?: string | null;
	version?: string | null;
}

export interface ClientInfo {
    identifier: string;
    display_name: string;
    category: ClientCategory;
    enabled: boolean;
    managed: boolean;
    detected: boolean;
    config_path: string;
    config_exists: boolean;
    has_mcp_config: boolean;
    supported_transports: string[];
    description?: string | null;
    homepage_url?: string | null;
    docs_url?: string | null;
    support_url?: string | null;
    config_type?: ClientConfigType | null;
    config_mode?: string | null;
    transport?: string | null;
    client_version?: string | null;
    install_path?: string | null;
    logo_url?: string | null;
    last_detected?: string | null;
    last_modified?: string | null;
    template: ClientTemplateMetadata;
    mcp_servers_count?: number | null;
}

export interface ClientCheckData {
	client: ClientInfo[];
	total: number;
	last_updated: string;
}

export interface ClientCheckResp {
	data?: ClientCheckData | null;
	error?: unknown | null;
	success: boolean;
}

export type ClientManageAction = "enable" | "disable";

export interface ClientManageResp {
	data?: { identifier: string; managed: boolean } | null;
	error?: unknown | null;
	success: boolean;
}

// Client config details
export type ClientConfigMode = "hosted" | "transparent" | "none" | string;
export type ClientConfigSelected = "default" | "profile" | "custom" | string;

export interface ClientImportedServer {
	name: string;
	command: string;
	args: string[];
	env: Record<string, string>;
	server_type: string;
	// Legacy fields kept optional for backward compatibility
	kind?: string;
	status?: string;
	url?: string | null;
}

export interface ClientImportSummary {
	attempted: boolean;
	imported_count: number;
	skipped_count: number;
	failed_count: number;
	errors?: Record<string, string> | null;
}

export interface ClientConfigImportItem {
	name?: string;
	server_name?: string;
	error?: string;
	tools?: { items?: unknown[] };
	resources?: { items?: unknown[] };
	resource_templates?: { items?: unknown[] };
	prompts?: { items?: unknown[] };
}

export interface ClientConfigImportData {
	summary: ClientImportSummary;
	imported_servers?: ClientImportedServer[] | null;
	profile_id?: string | null;
	scheduled?: boolean | null;
	scheduled_reason?: string | null;
	items?: ClientConfigImportItem[];
}

export interface ClientConfigImportResp {
	data?: ClientConfigImportData | null;
	error?: unknown | null;
	success: boolean;
}

export interface ClientConfigImportReq {
	identifier: string;
	preview?: boolean;
	profile_id?: string | null;
}

export interface ClientConfigData {
	config_exists: boolean;
	config_path: string;
	config_type?: ClientConfigType | null;
	content: unknown;
	warnings?: string[];
	has_mcp_config: boolean;
	imported_servers?: ClientImportedServer[] | null;
	import_summary?: ClientImportSummary | null;
	last_modified?: string | null;
	managed: boolean;
	mcp_servers_count: number;
	supported_transports: string[];
	template: ClientTemplateMetadata;
	description?: string | null;
	homepage_url?: string | null;
	docs_url?: string | null;
	support_url?: string | null;
	logo_url?: string | null;
}

export interface ClientConfigResp {
	data?: ClientConfigData | null;
	error?: unknown | null;
	success: boolean;
}

export interface ClientConfigUpdateReq {
	identifier: string;
	mode?: ClientConfigMode;
	preview?: boolean;
	selected_config?: ClientConfigSelected;
}

export interface ClientConfigUpdateData {
	success: boolean;
	preview: Record<string, unknown> | null;
	applied: boolean;
	backup_path?: string | null;
	warnings: string[];
	diff_format?: string | null;
	diff_before?: string | null;
	diff_after?: string | null;
	scheduled?: boolean | null;
	scheduled_reason?: string | null;
}

export interface ClientConfigUpdateResp {
	data?: ClientConfigUpdateData | null;
	error?: unknown | null;
	success: boolean;
}

export interface ClientConfigRestoreReq {
	identifier: string;
	backup: string;
}

// Backups
export interface ClientBackupEntry {
	identifier: string;
	backup: string;
	path: string;
	size: number;
	created_at?: string | null;
}

export interface ClientBackupListData {
	backups: ClientBackupEntry[];
}

export interface ClientBackupListResp {
	data?: ClientBackupListData | null;
	error?: unknown | null;
	success: boolean;
}

export interface ClientBackupActionResp {
	data?: { identifier: string; backup: string; message: string } | null;
	error?: unknown | null;
	success: boolean;
}

// Backup policy
export interface ClientBackupPolicyData {
	identifier: string;
	policy: string; // e.g., 'keep_n'
	limit?: number | null;
}

export interface ClientBackupPolicyResp {
	data?: ClientBackupPolicyData | null;
	error?: unknown | null;
	success: boolean;
}

export interface ClientBackupPolicyPayload {
	policy: string;
	limit?: number | null;
}

export interface ClientBackupPolicySetReq {
	identifier: string;
	policy: ClientBackupPolicyPayload;
}
