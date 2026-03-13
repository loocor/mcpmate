export const serversTranslations = {
    en: {
            title: "Connect and monitor MCP servers",
		toolbar: {
			search: {
				placeholder: "Search servers...",
				fields: {
					name: "Name",
					description: "Description",
				},
			},
			sort: {
				options: {
					name: "Name",
					enabled: "Enable Status",
				},
			},
		},
		actions: {
			debug: {
				title: "Inspect",
				show: "Inspect",
				hide: "Hide Inspect",
				open: "Open inspect view",
			},
			refresh: {
				title: "Refresh",
			},
			add: {
				title: "Add Server",
			},
		},
		emptyState: {
			title: "No servers found",
			description: "Add your first MCP server to get started",
			action: "Add First Server",
		},
		notifications: {
			importUnsupported: {
				title: "Unsupported content",
				message:
					"Drop text, JSON snippets, URLs, or MCP bundles to use Uni-Import.",
			},
			importEmpty: {
				title: "Nothing to import",
				message:
					"We could not detect any usable configuration from the dropped content.",
			},
			toggle: {
				enabledTitle: "Server enabled",
				disabledTitle: "Server disabled",
				message: "Server {{serverId}}",
				enabledDetail: "Server {{serverId}} has been enabled",
				disabledDetail: "Server {{serverId}} has been disabled",
				enableAction: "enable",
				disableAction: "disable",
				error: "Unable to {{action}} server: {{message}}",
				failedTitle: "Failed to toggle server",
			},
			update: {
				title: "Server updated",
				message: "Server {{serverId}}",
				errorTitle: "Update failed",
				errorMessage: "Unable to update {{serverId}}: {{message}}",
			},
			delete: {
				title: "Server deleted",
				message: "Server {{serverId}}",
				errorFallback: "Error deleting server",
			},
			genericError: {
				title: "Operation failed",
				unknown: "Unknown error",
			},
		},
		statsCards: {
			total: {
				title: "Total Servers",
				description: "registered",
			},
			enabled: {
				title: "Enabled",
				description: "feature toggled",
			},
			connected: {
				title: "Connected",
				description: "active connections",
			},
			instances: {
				title: "Instances",
				description: "total across servers",
			},
		},
		errors: {
			loadFailed: "Failed to load servers",
		},
		debug: {
			cardTitle: "Inspect Details",
			close: "Close",
			info: {
				baseUrl: "API Base URL",
				currentTime: "Current Time",
				error: "Error",
				data: "Servers Data",
			},
		},
		entity: {
			description: {
				serverLabel: "Server: {{name}}",
				profilesLabel: "Profiles: {{profiles}}",
				profilesNone: "-",
			},
			connectionTags: {
				stdio: "STDIO",
				http: "HTTP",
				sse: "SSE",
			},
			iconAlt: {
				named: "{{name}} icon",
				fallback: "Server icon",
			},
			stats: {
				tools: "Tools",
				prompts: "Prompts",
				resources: "Resources",
				templates: "Templates",
			},
			bottomTags: {
				profiles: "Profiles: {{profiles}}",
			},
		},
			capabilityList: {
				searchPlaceholder: "Search {{label}}...",
				emptyFallback: "No data.",
				detailsToggle: "Details",
				inputSchemaTitle: "Input Schema",
				outputSchemaTitle: "Output Schema",
				table: {
					argument: "Argument",
					required: "Required",
					requiredYes: "Yes",
					requiredNo: "No",
					description: "Description",
					property: "Property",
					type: "Type",
					details: "Details",
					enum: "enum:",
					items: "items:",
					itemsEnum: "items.enum:",
				},
			},
		detail: {
			errors: {
				noServerId: "No server ID provided",
			},
			viewModes: {
				browse: "Browse",
				debug: "Inspect",
			},
			overview: {
				labels: {
					service: "Service",
					runtime: "Runtime",
					type: "Type",
					protocol: "Protocol",
					version: "Version",
					capabilities: "Capabilities",
					description: "Description",
					defaultHeaders: "Default Headers",
					category: "Category",
					scenario: "Scenario",
					command: "Command",
					repository: "Repository",
				},
				status: {
					enabled: "Enabled",
					disabled: "Disabled",
				},
			},
			deleteDialog: {
				title: "Delete Server",
				description: "This action cannot be undone.",
				cancel: "Cancel",
				confirm: "Delete",
				pending: "Deleting...",
			},
			actions: {
				refresh: "Refresh",
				edit: "Edit",
				disable: "Disable",
				enable: "Enable",
				delete: "Delete",
			},
			instances: {
				title: "Instances ({{count}})",
				empty: "No instances.",
			},
			tabs: {
				overview: "Overview",
				tools: "Tools ({{count}})",
				prompts: "Prompts ({{count}})",
				resources: "Resources ({{count}})",
				templates: "Resource Templates ({{count}})",
			},
			capabilityList: {
				labels: {
					tools: "Tools",
					resources: "Resources",
					prompts: "Prompts",
					templates: "Resource Templates",
				},
				title: "{{label}} ({{count}})",
				empty: "No {{label}} from this server",
			},
			debug: {
				proxyUnavailable:
					"Proxy mode unavailable: server not enabled in any active profile.",
			},
			inspector: {
				channel: {
					proxy: "Proxy",
					native: "Native",
					fallback: "Fallback to native until proxy is available",
					hintTitle: "Proxy unavailable",
					hintDescription:
						"Enable this server in an active profile to exercise proxy mode.",
					openProfiles: "Open Profiles",
				},
				labels: {
					tools: "Tools",
					resources: "Resources",
					prompts: "Prompts",
					templates: "Resource Templates",
				},
				tabs: {
					results: "Results ({{count}})",
					logs: "Logs ({{count}})",
				},
				filterPlaceholder: "Filter {{label}}...",
				actions: {
					refresh: "Refresh",
					list: "List",
					inspect: "Inspect",
				},
				logs: {
					search: "Search logs...",
					clear: "Clear Logs",
					empty: "No inspector events yet.",
					status: {
						mode: {
							proxy: "PROXY",
							native: "NATIVE",
						},
						event: {
							request: "REQUEST",
							success: "SUCCESS",
							error: "ERROR",
							progress: "PROGRESS",
							log: "LOG",
							cancelled: "CANCELLED",
						},
					},
				},
				results: {
					lastFetched: "Last listed at {{time}}",
					emptyFetched: "No {{label}} returned.",
					emptyPrompt: "Run {{label}} list to fetch live data.",
				},
			},
		},
		instanceDetail: {
			errors: {
				missingParams: "Server ID or instance ID not provided",
				notFound: "Instance not found or error loading instance details.",
			},
			sections: {
				details: {
					title: "Instance Details",
					fields: {
						instanceId: "Instance ID:",
						server: "Server:",
						status: "Status:",
						connectionAttempts: "Connection Attempts:",
						connectedFor: "Connected For:",
						tools: "Tools:",
						processId: "Process ID:",
						health: "Health:",
					},
				},
				controls: {
					title: "Instance Controls",
					description: "Manage the instance connection state",
					actions: {
						cancelInitialization: "Cancel Initialization",
						disconnect: "Disconnect Instance",
						forceDisconnect: "Force Disconnect",
						reconnect: "Reconnect Instance",
						resetReconnect: "Reset & Reconnect",
					},
					hints: {
						reconnectDelay: "Reconnection may take a few moments to complete",
					},
				},
				metrics: {
					title: "Instance Metrics",
					description: "Performance metrics and statistics for this instance",
					fields: {
						cpuUsage: "CPU Usage:",
						memoryUsage: "Memory Usage:",
						connectionStability: "Connection Stability:",
					},
				},
			},
			notices: {
				healthIssue: "Health issue detected:",
				statusNote: "Status note:",
				error: "Error:",
			},
			healthMessages: {
				idlePlaceholder: "Instance is idle (placeholder, not connected)",
			},
		},
		manual: {
			ingest: {
				default: "Drop JSON/TOML/Text or MCP bundles (WIP) to begin",
				parsingDropped: "Parsing dropped text",
				parsingPasted: "Parsing pasted content",
				success: "Server configuration loaded successfully",
				noneDetectedError: "No servers detected in the input",
				noneDetectedTitle: "No servers detected",
				noneDetectedDescription:
					"We could not find any server definitions in the input.",
				parseFailedFallback: "Failed to parse input",
				parseFailedTitle: "Parsing failed",
				processingBundle: "Processing bundle: {{name}}",
				parsingFile: "Parsing text from {{name}}",
				editing: "Editing server",
				shortcut: "Ctrl/Cmd + V",
				tipPrefix: "Tip: press",
				tipSuffix: "to paste instantly.",
			},
			viewMode: {
				form: "Form",
				json: "JSON",
			},
			tabs: {
				core: "Core configuration",
				meta: "Meta information",
				metaWip: "WIP",
			},
			header: {
				title: {
					edit: "Editing server",
					import: "Import Server",
					create: "Server Uni-Import",
				},
				description: {
					edit: "Review and update the existing server settings. JSON preview remains read-only in this mode.",
					import: "Configure and import this server from the registry.",
					create:
						"You can directly drag and drop the configuration information, or enter it manually.",
				},
			},
			buttons: {
				reset: "Reset form",
				resetAria: "Reset form",
				cancel: "Cancel",
				preview: "Preview",
				previewing: "Previewing...",
				save: "Save changes",
				import: "Import server",
				saving: "Saving...",
				importing: "Importing...",
				processing: "Processing...",
			},
			fields: {
				name: {
					label: "Name",
					placeholder: "e.g., local-mcp",
					readOnlyTitle: "Editing server names is disabled",
				},
				type: {
					label: "Type",
					options: {
						stdio: "Stdio",
						sse: "SSE",
						streamable_http: "Streamable HTTP",
					},
				},
				command: {
					label: "Command",
					placeholder: "e.g., uvx my-mcp",
				},
				url: {
					label: "Server URL",
					placeholder: "https://example.com/mcp",
				},
				args: {
					label: "Arguments",
					ghost: "Add a new argument",
					placeholder: "Argument {{count}}",
				},
				env: {
					label: "Environment Variables",
					ghostKey: "Add a new key",
					keyPlaceholder: "KEY",
				},
				headers: {
					label: "HTTP Headers",
					ghostKey: "Add a new header",
					keyPlaceholder: "Header",
				},
				urlParams: {
					label: "URL Parameters",
					ghostKey: "Parameter name",
					ghostValue: "Value",
					keyPlaceholder: "Parameter",
				},
				common: {
					ghostValue: "Add a new value",
					valuePlaceholder: "Value",
				},
				meta: {
					iconAlt: "Server icon",
					version: {
						label: "Version",
						placeholder: "e.g., 1.0.0",
					},
					website: {
						label: "Website",
						placeholder: "https://example.com",
					},
					repo: {
						url: {
							label: "Repository URL",
							placeholder: "https://github.com/org/repo",
						},
						source: {
							label: "Repository Source",
							placeholder: "e.g., github",
						},
						subfolder: {
							label: "Repository Subfolder",
							placeholder: "Optional subfolder",
						},
						id: {
							label: "Repository Entry ID",
							placeholder: "Optional identifier",
						},
					},
					description: {
						label: "Description",
						placeholder: "Short description",
					},
				},
				json: {
					label: "Server JSON",
				},
			},
			errors: {
				nameRequired: "Name is required",
				kindRequired: "Select a server type",
				urlInvalid: "Provide a valid URL",
				commandRequired: "Command is required for stdio servers",
				urlRequired: "URL is required for non-stdio servers",
				commandRequiredTitle: "Command required",
				commandRequiredBody: "Provide a command for stdio servers.",
				endpointRequiredTitle: "Endpoint required",
				endpointRequiredBody: "Provide a URL for non-stdio servers.",
				jsonNoServers: "No servers found in JSON payload",
				jsonMultipleServers:
					"Manual entry accepts exactly one server in JSON mode",
				jsonParseFailedTitle: "Invalid JSON",
				jsonParseFailedFallback: "Failed to parse JSON",
				invalidJsonTitle: "Invalid JSON",
			},
		},
		wizard: {
			steps: {
				form: { label: "Configuration", hint: "Setup" },
				preview: { label: "Preview", hint: "Review" },
				result: { label: "Import & Profile", hint: "Complete" },
			},
			header: {
				addTitle: "Add MCP Server",
				addDescription: "Configure and install a new MCP server",
				editTitle: "Edit Server",
				editDescription: "Update server configuration",
			},
			preview: {
				retry: "Retry preview",
				generating: "Generating capability preview…",
				capabilities: {
					tool: "tool",
					tools: "tools",
					resource: "resource",
					resources: "resources",
					template: "template",
					templates: "templates",
					prompt: "prompt",
					prompts: "prompts",
				},
			},
			buttons: {
				back: "Back",
				cancel: "Cancel",
				preview: "Preview",
				previewing: "Previewing...",
				next: "Next",
				import: "Import",
				importing: "Importing...",
				validating: "Validating...",
				done: "Done",
			},
			result: {
				validation: {
					ready: "Ready to import",
					skipped: "Will be skipped",
					failed: "Failed validation",
				},
				readyTitle: "Ready to Import",
				readyDescription:
					"Click the Import button to proceed with installation",
				validating: "Validating import...",
				validationFailedTitle: "Import Validation Failed",
				validationFailedDescription:
					"Resolve the blocking issues below and run validation again.",
				validatedTitle: "Import Validated",
				validatedDescription:
					"Pre-validation succeeded. The server can be safely imported.",
				validatedWithWarningsTitle: "Import Validated With Warnings",
				validatedWithWarningsDescription:
					"Pre-validation succeeded, but some servers will be skipped.",
				alreadyInstalledTitle: "Already Installed",
				alreadyInstalledDescription:
					"Every selected server already exists. You can use it immediately—no import required.",
				skipSummary: {
					baseSingle: "Skipped {{count}} server",
					baseMultiple: "Skipped {{count}} servers",
					withDetail: "{{base}}: {{detail}}",
					suffixAlreadyInstalled: "Already installed—no new import required.",
				},
				readyStatusTitle: "Import Ready",
				readyStatusDescription:
					"The server configuration is ready to be imported. Review the information below and click Import when ready.",
				importingStatus: "Importing servers…",
				successTitle: "Import Successful",
				successAllSkipped:
					"All selected servers were already installed. No changes were applied.",
				successInstalled:
					"The server has been successfully installed and is ready to use.",
				successAutoEnabled: 'Enabled automatically in "{{profile}}".',
				failureTitle: "Import Failed",
				failureGeneric: "An error occurred during import",
				stats: {
					imported: "Imported",
					skipped: "Skipped",
				},
				installedServersTitle: "Installed Servers",
				success: {
					close:
						"Close this drawer to continue browsing or queue another server for import.",
					servers:
						"Open the Servers dashboard to review and manage the new server.",
					profiles:
						"Visit Profiles to add this server to the appropriate activation sets.",
					profilesWithName:
						'Open Profiles to verify "{{profile}}" reflects the new server.',
				},
				failure: {
					adjustServers:
						"Return to the Servers dashboard to adjust or remove the configuration before retrying.",
					reviewPreview:
						"Review the preview output above for errors and apply the necessary fixes before confirming again.",
					rerunPreview:
						"Keep this drawer open, update the configuration, and rerun Preview before another import attempt.",
				},
				nextSteps: {
					title: "Next steps",
				},
				skipSteps: {
					useExisting: "Close this drawer and start using the existing server.",
					chooseAnother:
						"Go back to the previous step to choose a different server if needed.",
				},
				failedSummary:
					"Import validation failed for {{servers}}. Resolve the issues before importing.",
				failedSummaryFallback_one: "the selected server",
				failedSummaryFallback_other: "the selected servers",
				validationErrorGeneric: "Failed to validate import",
				readySteps: {
					reviewConfig:
						"Review the server configuration and capabilities from the previous step.",
					autoAdd:
						"The server will be automatically added to the Default profile based on your settings.",
					manualAssign:
						"The server will remain unassigned. You can add it to profiles later from the Profiles page.",
					importAction:
						"Click the Import button below to install the server to your system.",
				},
			},
		},
		confirmDelete: {
			title: "Delete Server",
			description:
				'Are you sure you want to delete the server "{{serverId}}"? This action cannot be undone.',
			confirm: "Delete",
			cancel: "Cancel",
		},
	},
    "zh-CN": {
            title: "接入与监控 MCP 服务器运行状态",
		toolbar: {
			search: {
				placeholder: "搜索服务器...",
				fields: {
					name: "名称",
					description: "描述",
				},
			},
			sort: {
				options: {
					name: "名称",
					enabled: "启用状态",
				},
			},
		},
		actions: {
			debug: {
				title: "检视",
				show: "检视",
				hide: "隐藏检视",
				open: "打开检视视图",
			},
			refresh: {
				title: "刷新",
			},
			add: {
				title: "添加服务器",
			},
		},
		emptyState: {
			title: "没有找到服务器",
			description: "添加你的首个 MCP 服务器以开始使用",
			action: "添加首个服务器",
		},
		notifications: {
			importUnsupported: {
				title: "不支持的内容",
				message: "请拖放文本、JSON 片段、URL 或 MCP 安装包以使用 Uni-Import。",
			},
			importEmpty: {
				title: "没有可导入的内容",
				message: "无法从拖放的内容中检测到可用的配置。",
			},
			toggle: {
				enabledTitle: "服务器已启用",
				disabledTitle: "服务器已禁用",
				message: "服务器 {{serverId}}",
				enabledDetail: "服务器 {{serverId}} 已启用",
				disabledDetail: "服务器 {{serverId}} 已禁用",
				enableAction: "启用",
				disableAction: "禁用",
				error: "无法{{action}}服务器：{{message}}",
				failedTitle: "切换服务器失败",
			},
			update: {
				title: "服务器已更新",
				message: "服务器 {{serverId}}",
				errorTitle: "更新失败",
				errorMessage: "无法更新 {{serverId}}：{{message}}",
			},
			delete: {
				title: "服务器已删除",
				message: "服务器 {{serverId}}",
				errorFallback: "删除服务器时出错",
			},
			genericError: {
				title: "操作失败",
				unknown: "未知错误",
			},
		},
		statsCards: {
			total: {
				title: "服务器总数",
				description: "已登记",
			},
			enabled: {
				title: "已启用",
				description: "功能开关",
			},
			connected: {
				title: "已连接",
				description: "活动连接",
			},
			instances: {
				title: "实例",
				description: "服务器实例总数",
			},
		},
		errors: {
			loadFailed: "加载服务器失败",
		},
		debug: {
			cardTitle: "检视详情",
			close: "关闭",
			info: {
				baseUrl: "API 基础地址",
				currentTime: "当前时间",
				error: "错误",
				data: "服务器数据",
			},
		},
		entity: {
			description: {
				serverLabel: "服务器：{{name}}",
				profilesLabel: "关联配置：{{profiles}}",
				profilesNone: "-",
			},
			connectionTags: {
				stdio: "STDIO",
				http: "HTTP",
				sse: "SSE",
			},
			iconAlt: {
				named: "{{name}} 图标",
				fallback: "服务器图标",
			},
			stats: {
				tools: "工具",
				prompts: "提示",
				resources: "资源",
				templates: "模板",
			},
			bottomTags: {
				profiles: "关联配置：{{profiles}}",
			},
		},
			capabilityList: {
				searchPlaceholder: "搜索{{label}}...",
				emptyFallback: "暂无数据",
				detailsToggle: "详情",
				inputSchemaTitle: "输入模式",
				outputSchemaTitle: "输出模式",
				table: {
					argument: "参数",
					required: "必填",
					requiredYes: "是",
					requiredNo: "否",
					description: "说明",
					property: "字段",
					type: "类型",
					details: "细节",
					enum: "枚举:",
					items: "子项:",
					itemsEnum: "子项枚举:",
				},
			},
		detail: {
			errors: {
				noServerId: "未提供服务器 ID。",
			},
			viewModes: {
				browse: "浏览",
				debug: "检视",
			},
			overview: {
				labels: {
					service: "服务",
					runtime: "运行时",
					type: "类型",
					protocol: "协议",
					version: "版本",
					capabilities: "能力",
					description: "描述",
					defaultHeaders: "默认 Header",
					category: "分类",
					scenario: "场景",
					command: "启动指令",
					repository: "仓库",
				},
				status: {
					enabled: "已启用",
					disabled: "已禁用",
				},
			},
			deleteDialog: {
				title: "删除服务器",
				description: "此操作无法撤销。",
				cancel: "取消",
				confirm: "删除",
				pending: "正在删除...",
			},
			actions: {
				refresh: "刷新",
				edit: "编辑",
				disable: "禁用",
				enable: "启用",
				delete: "删除",
			},
			instances: {
				title: "实例 ({{count}})",
				empty: "暂无实例。",
			},
			tabs: {
				overview: "概览",
				tools: "工具 ({{count}})",
				prompts: "提示 ({{count}})",
				resources: "资源 ({{count}})",
				templates: "模板 ({{count}})",
			},
			capabilityList: {
				labels: {
					tools: "工具",
					resources: "资源",
					prompts: "提示",
					templates: "模板",
				},
				title: "{{label}} ({{count}})",
				empty: "该服务器无 {{label}}",
			},
			debug: {
				proxyUnavailable: "代理模式不可用：该服务器未在任何激活的配置中启用。",
			},
			inspector: {
				channel: {
					proxy: "代理",
					native: "本地",
					fallback: "代理不可用时自动使用本地模式",
					hintTitle: "代理不可用",
					hintDescription: "请在某个启用的配置中启用该服务器以使用代理模式。",
					openProfiles: "打开配置",
				},
				labels: {
					tools: "工具",
					resources: "资源",
					prompts: "提示",
					templates: "模板",
				},
				tabs: {
					results: "结果 ({{count}})",
					logs: "日志 ({{count}})",
				},
				filterPlaceholder: "筛选 {{label}}...",
				actions: {
					refresh: "刷新",
					list: "列出",
					inspect: "检视",
				},
				logs: {
					search: "搜索日志...",
					clear: "清空日志",
					empty: "暂无检测事件。",
					status: {
						mode: {
							proxy: "代理",
							native: "本地",
						},
						event: {
							request: "请求",
							success: "成功",
							error: "错误",
							progress: "进度",
							log: "日志",
							cancelled: "已取消",
						},
					},
				},
				results: {
					lastFetched: "上次列出时间 {{time}}",
					emptyFetched: "未返回任何 {{label}}。",
					emptyPrompt: "运行 {{label}} 列表以获取最新数据。",
				},
			},
		},
		instanceDetail: {
			errors: {
				missingParams: "未提供服务器 ID 或实例 ID。",
				notFound: "未找到实例或加载实例详情时出错。",
			},
			sections: {
				details: {
					title: "实例详情",
					fields: {
						instanceId: "实例 ID",
						server: "服务器",
						status: "状态",
						connectionAttempts: "连接尝试次数",
						connectedFor: "已连接时长",
						tools: "工具",
						processId: "进程 ID",
						health: "健康状态",
					},
				},
				controls: {
					title: "实例控制",
					description: "管理实例的连接状态",
					actions: {
						cancelInitialization: "取消初始化",
						disconnect: "断开实例",
						forceDisconnect: "强制断开",
						reconnect: "重新连接实例",
						resetReconnect: "重置并重新连接",
					},
					hints: {
						reconnectDelay: "重新连接可能需要一些时间完成",
					},
				},
				metrics: {
					title: "实例指标",
					description: "查看该实例的性能指标与统计数据",
					fields: {
						cpuUsage: "CPU 使用率",
						memoryUsage: "内存使用",
						connectionStability: "连接稳定性",
					},
				},
			},
			notices: {
				healthIssue: "检测到健康状况问题",
				statusNote: "状态提示",
				error: "错误",
			},
			healthMessages: {
				idlePlaceholder: "实例处于空闲状态（占位，未连接）",
			},
		},
		manual: {
			ingest: {
				default: "拖拽 JSON/TOML/文本或 MCP 安装包（即将支持）即可开始",
				parsingDropped: "正在解析拖入的文本",
				parsingPasted: "正在解析粘贴的内容",
				success: "服务器配置已成功载入",
				noneDetectedError: "输入中未检测到服务器",
				noneDetectedTitle: "未检测到服务器",
				noneDetectedDescription: "无法在输入内容中找到任何服务器定义。",
				parseFailedFallback: "解析输入失败",
				parseFailedTitle: "解析失败",
				processingBundle: "正在处理安装包：{{name}}",
				parsingFile: "正在解析 {{name}} 中的文本",
				editing: "编辑服务器",
				shortcut: "Ctrl/Cmd + V",
				tipPrefix: "提示：按下",
				tipSuffix: "即可快速粘贴。",
			},
			viewMode: {
				form: "表单",
				json: "JSON",
			},
			tabs: {
				core: "核心配置",
				meta: "元信息",
				metaWip: "预览",
			},
			header: {
				title: {
					edit: "编辑服务器",
					import: "导入服务器",
					create: "服务器统一导入",
				},
				description: {
					edit: "检查并更新服务器设置。此模式下 JSON 预览仅可读。",
					import: "配置并从仓库导入此服务器。",
					create: "可以直接拖拽配置，也可以手动录入。",
				},
			},
			buttons: {
				reset: "重置表单",
				resetAria: "重置表单",
				cancel: "取消",
				preview: "预览",
				previewing: "正在预览...",
				save: "保存",
				import: "导入服务器",
				saving: "正在保存...",
				importing: "正在导入...",
				processing: "正在处理...",
			},
			fields: {
				name: {
					label: "名称",
					placeholder: "例如：local-mcp",
					readOnlyTitle: "编辑模式下不可修改名称",
				},
				type: {
					label: "类型",
					options: {
						stdio: "Stdio",
						sse: "SSE",
						streamable_http: "Streamable HTTP",
					},
				},
				command: {
					label: "启动命令",
					placeholder: "例如：uvx my-mcp",
				},
				url: {
					label: "服务器地址",
					placeholder: "https://example.com/mcp",
				},
				args: {
					label: "命令参数",
					ghost: "添加参数",
					placeholder: "参数 {{count}}",
				},
				env: {
					label: "环境变量",
					ghostKey: "添加键名",
					keyPlaceholder: "键名",
				},
				headers: {
					label: "HTTP 头",
					ghostKey: "添加 Header",
					keyPlaceholder: "Header",
				},
				urlParams: {
					label: "URL 参数",
					ghostKey: "参数名",
					ghostValue: "参数值",
					keyPlaceholder: "参数",
				},
				common: {
					ghostValue: "添加值",
					valuePlaceholder: "值",
				},
				meta: {
					iconAlt: "服务器图标",
					version: {
						label: "版本",
						placeholder: "例如：1.0.0",
					},
					website: {
						label: "网站",
						placeholder: "https://example.com",
					},
					repo: {
						url: {
							label: "仓库地址",
							placeholder: "https://github.com/org/repo",
						},
						source: {
							label: "仓库来源",
							placeholder: "例如：github",
						},
						subfolder: {
							label: "仓库子目录",
							placeholder: "可选子目录",
						},
						id: {
							label: "仓库条目 ID",
							placeholder: "可选标识",
						},
					},
					description: {
						label: "描述",
						placeholder: "简短说明",
					},
				},
				json: {
					label: "服务器 JSON",
				},
			},
			errors: {
				nameRequired: "名称为必填项",
				kindRequired: "请选择服务器类型",
				urlInvalid: "请输入合法的 URL",
				commandRequired: "Stdio 服务器需要提供启动命令",
				urlRequired: "非 Stdio 服务器需要提供 URL",
				commandRequiredTitle: "缺少命令",
				commandRequiredBody: "请为 Stdio 服务器提供启动命令。",
				endpointRequiredTitle: "缺少端点",
				endpointRequiredBody: "请为非 Stdio 服务器提供 URL。",
				jsonNoServers: "JSON 中未找到服务器定义",
				jsonMultipleServers: "手动录入模式仅支持单个服务器",
				jsonParseFailedTitle: "JSON 无法解析",
				jsonParseFailedFallback: "解析 JSON 失败",
				invalidJsonTitle: "JSON 无效",
			},
		},
		wizard: {
			steps: {
				form: { label: "配置", hint: "设置" },
				preview: { label: "预览", hint: "复核" },
				result: { label: "导入", hint: "完成" },
			},
			header: {
				addTitle: "新增服务器",
				addDescription: "配置并安装新的 MCP 服务器",
				editTitle: "编辑服务器",
				editDescription: "更新 MCP 服务器配置",
			},
			preview: {
				retry: "重新预览",
				generating: "正在生成能力预览…",
				capabilities: {
					tool: "工具",
					tools: "工具",
					resource: "资源",
					resources: "资源",
					template: "模板",
					templates: "模板",
					prompt: "提示",
					prompts: "提示",
				},
			},
			buttons: {
				back: "返回",
				cancel: "取消",
				preview: "预览",
				previewing: "正在预览...",
				next: "下一步",
				import: "导入",
				importing: "正在导入...",
				validating: "正在校验...",
				done: "完成",
			},
			result: {
				validation: {
					ready: "可被导入",
					skipped: "将被跳过",
					failed: "校验失败",
				},
				readyTitle: "准备导入",
				readyDescription: "点击“导入”按钮继续安装流程",
				validating: "正在校验导入...",
				validationFailedTitle: "导入校验失败",
				validationFailedDescription: "请解决下方列出的阻塞项后重新运行校验。",
				validatedTitle: "导入校验通过",
				validatedDescription: "预校验成功，可以安全执行导入。",
				validatedWithWarningsTitle: "导入校验通过（含提示）",
				validatedWithWarningsDescription: "预校验成功，但部分服务器会被跳过。",
				alreadyInstalledTitle: "无需导入",
				alreadyInstalledDescription:
					"所选服务器已经存在，可直接使用，无需再次导入。",
				skipSummary: {
					baseSingle: "已跳过 {{count}} 个服务器",
					baseMultiple: "已跳过 {{count}} 个服务器",
					withDetail: "{{base}}：{{detail}}",
					suffixAlreadyInstalled: "已存在可直接使用，无需重新导入。",
				},
				readyStatusTitle: "可执行导入",
				readyStatusDescription: "配置已就绪，请确认信息后点击“导入”。",
				importingStatus: "正在导入服务器…",
				successTitle: "导入成功",
				successAllSkipped: "所选服务器已安装，本次未做任何更改。",
				successInstalled: "服务器已成功安装，可立即使用。",
				successAutoEnabled: '已自动启用至 "{{profile}}"。',
				failureTitle: "导入失败",
				failureGeneric: "导入过程中发生错误",
				stats: {
					imported: "已导入",
					skipped: "已跳过",
				},
				installedServersTitle: "已安装服务器",
				success: {
					close: "关闭抽屉，继续浏览或排队下一个服务器。",
					servers: "打开服务器面板检查并管理新服务器。",
					profiles: "前往配置页，将该服务器加入适用的激活集合。",
					profilesWithName: '打开配置页，确认 "{{profile}}" 已显示该服务器。',
				},
				failure: {
					adjustServers: "返回服务器面板调整或移除配置后再试。",
					reviewPreview: "根据上方预览结果查找错误并修复后重新确认。",
					rerunPreview: "保持当前抽屉，更新配置并重新预览后再导入。",
				},
				nextSteps: {
					title: "后续操作",
				},
				skipSteps: {
					useExisting: "关闭抽屉，直接使用已有服务器。",
					chooseAnother: "返回上一步，重新选择其他服务器（如有需要）。",
				},
				failedSummary: "导入校验失败：{{servers}}。请解决问题后再试。",
				failedSummaryFallback_one: "所选服务器",
				failedSummaryFallback_other: "所选服务器",
				validationErrorGeneric: "导入校验失败",
				readySteps: {
					reviewConfig: "请再次检查上一阶段生成的服务器配置与能力。",
					autoAdd: "将按设置自动加入 Default 配置集。",
					manualAssign: "当前不会自动分配，可稍后在配置页手动添加。",
					importAction: "点击下方“导入”按钮安装服务器。",
				},
			},
		},
		confirmDelete: {
			title: "删除服务器",
			description: '确定要删除服务器 "{{serverId}}" 吗？此操作无法撤销。',
			confirm: "删除",
			cancel: "取消",
		},
	},
    "ja-JP": {
            title: "MCP サーバーの接続と監視",
		toolbar: {
			search: {
				placeholder: "サーバーを検索...",
				fields: {
					name: "名前",
					description: "説明",
				},
			},
			sort: {
				options: {
					name: "名前",
					enabled: "有効状態",
				},
			},
		},
		actions: {
			debug: {
				title: "検査",
				show: "検査",
				hide: "検査を隠す",
				open: "検査ビューを開く",
			},
			refresh: {
				title: "更新",
			},
			add: {
				title: "サーバーを追加",
			},
		},
		emptyState: {
			title: "サーバーが見つかりません",
			description: "最初の MCP サーバーを追加して利用を開始してください",
			action: "最初のサーバーを追加",
		},
		notifications: {
			importUnsupported: {
				title: "サポートされていない内容",
				message:
					"Uni-Import を使うにはテキスト、JSON スニペット、URL、または MCP バンドルをドロップしてください。",
			},
			importEmpty: {
				title: "インポートできる内容がありません",
				message: "ドロップされた内容から利用可能な設定を検出できませんでした。",
			},
			toggle: {
				enabledTitle: "サーバーを有効化しました",
				disabledTitle: "サーバーを無効化しました",
				message: "サーバー {{serverId}}",
				enabledDetail: "サーバー {{serverId}} を有効化しました",
				disabledDetail: "サーバー {{serverId}} を無効化しました",
				enableAction: "有効化",
				disableAction: "無効化",
				error: "サーバーを{{action}}できません: {{message}}",
				failedTitle: "サーバーの切り替えに失敗しました",
			},
			update: {
				title: "サーバーを更新しました",
				message: "サーバー {{serverId}}",
				errorTitle: "更新に失敗しました",
				errorMessage: "{{serverId}} を更新できません: {{message}}",
			},
			delete: {
				title: "サーバーを削除しました",
				message: "サーバー {{serverId}}",
				errorFallback: "サーバーの削除中にエラーが発生しました",
			},
			genericError: {
				title: "操作に失敗しました",
				unknown: "不明なエラー",
			},
		},
		statsCards: {
			total: {
				title: "サーバー総数",
				description: "登録済み",
			},
			enabled: {
				title: "有効",
				description: "機能トグル",
			},
			connected: {
				title: "接続中",
				description: "アクティブ接続",
			},
			instances: {
				title: "インスタンス",
				description: "全サーバーの合計",
			},
		},
		errors: {
			loadFailed: "サーバーの読み込みに失敗しました",
		},
		debug: {
			cardTitle: "検査情報",
			close: "閉じる",
			info: {
				baseUrl: "API ベース URL",
				currentTime: "現在時刻",
				error: "エラー",
				data: "サーバーデータ",
			},
		},
		entity: {
			description: {
				serverLabel: "サーバー: {{name}}",
				profilesLabel: "関連プロファイル: {{profiles}}",
				profilesNone: "-",
			},
			connectionTags: {
				stdio: "STDIO",
				http: "HTTP",
				sse: "SSE",
			},
			iconAlt: {
				named: "{{name}} のアイコン",
				fallback: "サーバーのアイコン",
			},
			stats: {
				tools: "ツール",
				prompts: "プロンプト",
				resources: "リソース",
				templates: "テンプレート",
			},
			bottomTags: {
				profiles: "関連プロファイル: {{profiles}}",
			},
		},
		capabilityList: {
			searchPlaceholder: "{{label}} を検索...",
			emptyFallback: "データがありません",
			detailsToggle: "詳細",
			inputSchemaTitle: "入力スキーマ",
			outputSchemaTitle: "出力スキーマ",
			table: {
				argument: "引数",
				required: "必須",
				requiredYes: "はい",
				requiredNo: "いいえ",
				description: "説明",
				property: "プロパティ",
				type: "型",
				details: "詳細",
				enum: "列挙:",
				items: "items:",
				itemsEnum: "items.enum:",
			},
		},
		detail: {
			errors: {
				noServerId: "サーバー ID が指定されていません。",
			},
			viewModes: {
				browse: "閲覧",
				debug: "検査",
			},
			overview: {
				labels: {
					service: "サービス",
					runtime: "ランタイム",
					type: "タイプ",
					protocol: "プロトコル",
					version: "バージョン",
					capabilities: "機能",
					description: "説明",
					defaultHeaders: "既定ヘッダー",
					category: "カテゴリ",
					scenario: "シナリオ",
					command: "コマンド",
					repository: "リポジトリ",
				},
				status: {
					enabled: "有効",
					disabled: "無効",
				},
			},
			deleteDialog: {
				title: "サーバーを削除",
				description: "この操作は元に戻せません。",
				cancel: "キャンセル",
				confirm: "削除",
				pending: "削除中...",
			},
			actions: {
				refresh: "更新",
				edit: "編集",
				disable: "無効化",
				enable: "有効化",
				delete: "削除",
			},
			instances: {
				title: "インスタンス ({{count}})",
				empty: "インスタンスがありません。",
			},
			tabs: {
				overview: "概要",
				tools: "ツール ({{count}})",
				prompts: "プロンプト ({{count}})",
				resources: "リソース ({{count}})",
				templates: "テンプレート ({{count}})",
			},
			capabilityList: {
				labels: {
					tools: "ツール",
					resources: "リソース",
					prompts: "プロンプト",
					templates: "テンプレート",
				},
				title: "{{label}} ({{count}})",
				empty: "このサーバーには {{label}} がありません",
			},
			debug: {
				proxyUnavailable:
					"プロキシモードは利用できません：このサーバーは有効なプロファイルに含まれていません。",
			},
			inspector: {
				channel: {
					proxy: "プロキシ",
					native: "ネイティブ",
					fallback:
						"プロキシが利用可能になるまでネイティブにフォールバックします",
					hintTitle: "プロキシが利用不可",
					hintDescription:
						"プロキシモードを使うには、有効なプロファイルでこのサーバーを有効化してください。",
					openProfiles: "プロファイルを開く",
				},
				labels: {
					tools: "ツール",
					resources: "リソース",
					prompts: "プロンプト",
					templates: "テンプレート",
				},
				tabs: {
					results: "結果 ({{count}})",
					logs: "ログ ({{count}})",
				},
				filterPlaceholder: "{{label}} をフィルタ...",
				actions: {
					refresh: "更新",
					list: "取得",
					inspect: "検査",
				},
				logs: {
					search: "ログを検索...",
					clear: "ログをクリア",
					empty: "インスペクターのイベントはありません。",
					status: {
						mode: {
							proxy: "プロキシ",
							native: "ネイティブ",
						},
						event: {
							request: "リクエスト",
							success: "成功",
							error: "エラー",
							progress: "進行",
							log: "ログ",
							cancelled: "キャンセル済み",
						},
					},
				},
				results: {
					lastFetched: "最終取得 {{time}}",
					emptyFetched: "{{label}} は返されませんでした。",
					emptyPrompt:
						"{{label}} リストを実行して最新データを取得してください。",
				},
			},
		},
		instanceDetail: {
			errors: {
				missingParams:
					"サーバー ID またはインスタンス ID が指定されていません。",
				notFound:
					"インスタンスが見つからないか、詳細の読み込み中にエラーが発生しました。",
			},
			sections: {
				details: {
					title: "インスタンス詳細",
					fields: {
						instanceId: "インスタンス ID：",
						server: "サーバー：",
						status: "状態：",
						connectionAttempts: "接続試行回数：",
						connectedFor: "接続時間：",
						tools: "ツール：",
						processId: "プロセス ID：",
						health: "ヘルス：",
					},
				},
				controls: {
					title: "インスタンス制御",
					description: "インスタンスの接続状態を管理します",
					actions: {
						cancelInitialization: "初期化をキャンセル",
						disconnect: "インスタンスを切断",
						forceDisconnect: "強制切断",
						reconnect: "インスタンスを再接続",
						resetReconnect: "リセットして再接続",
					},
					hints: {
						reconnectDelay: "再接続の完了には数秒かかる場合があります",
					},
				},
				metrics: {
					title: "インスタンスメトリクス",
					description: "このインスタンスのパフォーマンス指標と統計",
					fields: {
						cpuUsage: "CPU 使用率：",
						memoryUsage: "メモリ使用量：",
						connectionStability: "接続安定性：",
					},
				},
			},
			notices: {
				healthIssue: "ヘルス問題を検出しました：",
				statusNote: "ステータスメモ：",
				error: "エラー：",
			},
			healthMessages: {
				idlePlaceholder:
					"インスタンスはアイドル状態です（プレースホルダー、未接続）",
			},
		},
		manual: {
			ingest: {
				default:
					"JSON/TOML/テキストまたは MCP バンドル（プレビュー）をドロップして開始",
				parsingDropped: "ドロップしたテキストを解析しています",
				parsingPasted: "貼り付けた内容を解析しています",
				success: "サーバー構成を読み込みました",
				noneDetectedError: "入力からサーバーが検出されませんでした",
				noneDetectedTitle: "サーバーが見つかりません",
				noneDetectedDescription:
					"入力内容にサーバー定義が含まれていませんでした。",
				parseFailedFallback: "入力の解析に失敗しました",
				parseFailedTitle: "解析に失敗しました",
				processingBundle: "バンドルを処理中：{{name}}",
				parsingFile: "{{name}} のテキストを解析しています",
				editing: "サーバーを編集",
				shortcut: "Ctrl/Cmd + V",
				tipPrefix: "ヒント：",
				tipSuffix: "を押すとすぐに貼り付けできます。",
			},
			viewMode: {
				form: "フォーム",
				json: "JSON",
			},
			tabs: {
				core: "基本設定",
				meta: "メタ情報",
				metaWip: "プレビュー",
			},
			header: {
				title: {
					edit: "サーバーを編集",
					import: "サーバーをインポート",
					create: "サーバー統合インポート",
				},
				description: {
					edit: "既存のサーバー設定を確認して更新します。このモードでは JSON プレビューは読み取り専用です。",
					import: "レジストリから取得したサーバーを設定してインポートします。",
					create: "構成をドラッグ＆ドロップするか、手動で入力してください。",
				},
			},
			buttons: {
				reset: "フォームをリセット",
				resetAria: "フォームをリセット",
				cancel: "キャンセル",
				preview: "プレビュー",
				previewing: "プレビュー中...",
				save: "変更を保存",
				import: "サーバーをインポート",
				saving: "保存中...",
				importing: "インポート中...",
				processing: "処理中...",
			},
			fields: {
				name: {
					label: "名称",
					placeholder: "例: local-mcp",
					readOnlyTitle: "編集モードでは名称を変更できません",
				},
				type: {
					label: "種別",
					options: {
						stdio: "Stdio",
						sse: "SSE",
						streamable_http: "ストリーミング HTTP",
					},
				},
				command: {
					label: "コマンド",
					placeholder: "例: uvx my-mcp",
				},
				url: {
					label: "サーバー URL",
					placeholder: "https://example.com/mcp",
				},
				args: {
					label: "引数",
					ghost: "引数を追加",
					placeholder: "引数 {{count}}",
				},
				env: {
					label: "環境変数",
					ghostKey: "キーを追加",
					keyPlaceholder: "キー",
				},
				headers: {
					label: "HTTP ヘッダー",
					ghostKey: "ヘッダーを追加",
					keyPlaceholder: "ヘッダー",
				},
				urlParams: {
					label: "URL パラメータ",
					ghostKey: "パラメータ名",
					ghostValue: "値",
					keyPlaceholder: "パラメータ",
				},
				common: {
					ghostValue: "値を追加",
					valuePlaceholder: "値",
				},
				meta: {
					iconAlt: "サーバーアイコン",
					version: {
						label: "バージョン",
						placeholder: "例: 1.0.0",
					},
					website: {
						label: "Web サイト",
						placeholder: "https://example.com",
					},
					repo: {
						url: {
							label: "リポジトリ URL",
							placeholder: "https://github.com/org/repo",
						},
						source: {
							label: "リポジトリ種類",
							placeholder: "例: github",
						},
						subfolder: {
							label: "リポジトリ サブフォルダ",
							placeholder: "任意のサブフォルダ",
						},
						id: {
							label: "リポジトリ ID",
							placeholder: "任意の識別子",
						},
					},
					description: {
						label: "説明",
						placeholder: "簡単な説明",
					},
				},
				json: {
					label: "サーバー JSON",
				},
			},
			errors: {
				nameRequired: "名称は必須です",
				kindRequired: "サーバー種別を選択してください",
				urlInvalid: "有効な URL を入力してください",
				commandRequired: "Stdio サーバーにはコマンドが必要です",
				urlRequired: "非 Stdio サーバーには URL が必要です",
				commandRequiredTitle: "コマンドが必要です",
				commandRequiredBody:
					"Stdio サーバーに使用するコマンドを入力してください。",
				endpointRequiredTitle: "エンドポイントが必要です",
				endpointRequiredBody: "非 Stdio サーバーには URL を入力してください。",
				jsonNoServers: "JSON からサーバーが見つかりませんでした",
				jsonMultipleServers: "JSON モードではサーバーを 1 件のみ扱えます",
				jsonParseFailedTitle: "JSON を解析できません",
				jsonParseFailedFallback: "JSON の解析に失敗しました",
				invalidJsonTitle: "JSON が無効です",
			},
		},
		wizard: {
			steps: {
				form: { label: "構成", hint: "セットアップ" },
				preview: { label: "プレビュー", hint: "確認" },
				result: { label: "インポートと割り当て", hint: "完了" },
			},
			header: {
				addTitle: "MCP サーバーを追加",
				addDescription: "新しい MCP サーバーを設定してインストールします",
				editTitle: "サーバーを編集",
				editDescription: "サーバー設定を更新します",
			},
			preview: {
				retry: "プレビューを再実行",
				generating: "機能プレビューを生成中…",
				capabilities: {
					tool: "ツール",
					tools: "ツール",
					resource: "リソース",
					resources: "リソース",
					template: "テンプレート",
					templates: "テンプレート",
					prompt: "プロンプト",
					prompts: "プロンプト",
				},
			},
			buttons: {
				back: "戻る",
				cancel: "キャンセル",
				preview: "プレビュー",
				previewing: "プレビュー中...",
				next: "次へ",
				import: "インポート",
				importing: "インポート中...",
				validating: "検証中...",
				done: "完了",
			},
			result: {
				validation: {
					ready: "インポート対象",
					skipped: "スキップ予定",
					failed: "検証失敗",
				},
				readyTitle: "インポートの準備完了",
				readyDescription:
					"インポートボタンを押してインストールを続行してください",
				validating: "インポートを検証中...",
				validationFailedTitle: "インポート検証に失敗しました",
				validationFailedDescription:
					"以下のブロッカーを解消してから、再度検証を実行してください。",
				validatedTitle: "インポート検証に成功",
				validatedDescription:
					"事前検証を通過しました。安全にインポートできます。",
				validatedWithWarningsTitle: "警告付きで検証に成功",
				validatedWithWarningsDescription:
					"事前検証は成功しましたが、一部のサーバーはスキップされます。",
				alreadyInstalledTitle: "インポート不要",
				alreadyInstalledDescription:
					"選択したサーバーはすでに存在します。インポートせずにそのまま利用できます。",
				skipSummary: {
					baseSingle: "{{count}} 件のサーバーをスキップしました",
					baseMultiple: "{{count}} 件のサーバーをスキップしました",
					withDetail: "{{base}}：{{detail}}",
					suffixAlreadyInstalled:
						"既に利用可能なため、再インポートは不要です。",
				},
				readyStatusTitle: "インポート可能",
				readyStatusDescription:
					"設定は完了しています。内容を確認し、準備が整ったらインポートを実行してください。",
				importingStatus: "サーバーをインポートしています…",
				successTitle: "インポート成功",
				successAllSkipped:
					"選択したサーバーは既にインストール済みのため、変更はありませんでした。",
				successInstalled:
					"サーバーは正常にインストールされ、すぐに利用できます。",
				successAutoEnabled: '"{{profile}}" に自動的に割り当てました。',
				failureTitle: "インポート失敗",
				failureGeneric: "インポート中にエラーが発生しました",
				stats: {
					imported: "インポート済み",
					skipped: "スキップ",
				},
				installedServersTitle: "インストール済みサーバー",
				success: {
					close:
						"このドロワーを閉じてブラウズを続けるか、別のサーバーを追加してください。",
					servers:
						"サーバーダッシュボードを開き、新しいサーバーを確認・管理します。",
					profiles:
						"プロファイルを開いて適切なアクティベーションセットに追加します。",
					profilesWithName:
						'プロファイルを開き、"{{profile}}" に新しいサーバーが反映されているか確認します。',
				},
				failure: {
					adjustServers:
						"サーバーダッシュボードに戻り、設定を調整または削除してから再試行してください。",
					reviewPreview:
						"上部のプレビュー結果を確認し、問題を解消してから再度実行してください。",
					rerunPreview:
						"このドロワーを開いたまま設定を更新し、プレビューを再実行してからもう一度試してください。",
				},
				nextSteps: {
					title: "次のステップ",
				},
				skipSteps: {
					useExisting:
						"ドロワーを閉じ、既存のサーバーをそのまま利用してください。",
					chooseAnother:
						"必要に応じて前のステップへ戻り、別のサーバーを選択してください。",
				},
				failedSummary:
					"インポート検証に失敗しました（対象: {{servers}}）。問題を解消してから再試行してください。",
				failedSummaryFallback_one: "選択したサーバー",
				failedSummaryFallback_other: "選択したサーバー",
				validationErrorGeneric: "インポート検証に失敗しました",
				readySteps: {
					reviewConfig:
						"前のステップで生成された構成と機能を再確認してください。",
					autoAdd: "設定に従い自動的に Default プロファイルへ追加されます。",
					manualAssign:
						"現在は割り当てられません。後からプロファイル画面で追加できます。",
					importAction:
						"下の「インポート」ボタンを押してサーバーをインストールします。",
				},
			},
		},
		confirmDelete: {
			title: "サーバーを削除",
			description:
				'サーバー "{{serverId}}" を削除してもよろしいですか？この操作は取り消せません。',
			confirm: "削除",
			cancel: "キャンセル",
		},
	},
};
