export const clientsTranslations = {
    en: {
        title: "Discover and manage client connections and visibility",
		toolbar: {
			search: {
				placeholder: "Search clients...",
				fields: {
					displayName: "Display Name",
					identifier: "Identifier",
					description: "Description",
				},
			},
			filters: {
				title: "Filter",
				options: {
					all: "All",
					allowed: "Allowed",
					pending: "Pending",
					denied: "Denied",
				},
			},
			sort: {
				options: {
					displayName: "Name",
					approvalStatus: "Governance Status",
					managed: "Management Status",
				},
			},
			actions: {
				refresh: {
					title: "Refresh",
					notificationTitle: "Refresh triggered",
					notificationMessage: "Latest client state will sync to the list",
				},
				add: {
					title: "Add Client",
					notificationTitle: "Feature in Development",
					notificationMessage:
						"This feature is being implemented, please stay tuned",
				},
			},
		},
		statsCards: {
			total: {
				title: "Total Clients",
				description: "discovered",
			},
			detected: {
				title: "Detected",
				description: "installed",
			},
			managed: {
				title: "Managed",
				description: "management enabled",
			},
			configured: {
				title: "Configured",
				description: "has MCP config",
			},
		},
		notifications: {
			managementUpdated: {
				title: "Updated",
				message: "Client management state updated",
			},
			operationFailed: {
				title: "Operation failed",
			},
		},
		entity: {
			fallbackName: "Client",
			stats: {
				config: "Config",
				servers: "Servers",
				managed: "Managed",
				detected: "Detected",
			},
			config: {
				notConfigured: "Not configured",
			},
			bottomTags: {
				servers: "Servers: {{count}}",
			},
			status: {
				detected: "Detected",
				notDetected: "Not Detected",
			},
			badge: {
				detected: "Detected",
				notDetected: "Not Detected",
			},
		},
		states: {
			on: "On",
			off: "Off",
			yes: "Yes",
			no: "No",
			present: "Present",
			missing: "Missing",
		},
		emptyState: {
			title: "No clients found",
			description:
				"Make sure MCPMate backend is running and detection is enabled",
		},
		detail: {
			noIdentifier: "No client identifier provided.",
			badges: {
				managed: "Managed",
				unmanaged: "Unmanaged",
				detected: "Detected",
				notDetected: "Not Detected",
			},
			tabs: {
				overview: "Overview",
				configuration: "Configuration",
				backups: "Backups",
				logs: "Logs",
			},
			directExposure: {
				title: "Capability Level",
				badge: "Direct Exposure",
				description:
					"Adjust which capabilities from this server should be exposed directly to the current client.",
				serverDescriptionFallback:
					"Choose which capabilities from this server should be exposed directly to the client.",
				toolsTitle: "Tools",
				toolsSummary: "{{enabled}} / {{total}} capabilities exposed directly",
				tabs: {
					tools: "Tools",
					prompts: "Prompts",
					resources: "Resources",
					templates: "Templates",
					toolsWithCounts: "Tools ({{enabled}}/{{total}})",
					promptsWithCounts: "Prompts ({{enabled}}/{{total}})",
					resourcesWithCounts: "Resources ({{enabled}}/{{total}})",
					templatesWithCounts: "Templates ({{enabled}}/{{total}})",
				},
				searchPlaceholders: {
					tools: "Search tools...",
					prompts: "Search prompts...",
					resources: "Search resources...",
					templates: "Search templates...",
				},
				empty: {
					tools: "No tools found for this server.",
					prompts: "No prompts found for this server.",
					resources: "No resources found for this server.",
					templates: "No templates found for this server.",
				},
				statusPlaceholder: "Status",
				filters: {
					status: {
						all: "All",
						enabled: "Enabled",
						disabled: "Disabled",
					},
				},
				buttons: {
					selectAll: "Select all",
					clearSelection: "Clear",
					enable: "Enable",
					disable: "Disable",
				},
				notifications: {
					savedTitle: "Direct capabilities updated",
					savedMessage: "The direct capability exposure settings have been updated.",
					saveFailedTitle: "Unable to update direct capabilities",
				},
			},
				overview: {
					labels: {
					configPath: "Config Path",
					lastModified: "Last Modified",
					supportedTransports: "Supported Transports",
					homepage: "Homepage",
					docs: "Docs",
					support: "Support",
				},
				buttons: {
					edit: "Edit",
					refresh: "Refresh",
					enable: "Enable",
					disable: "Disable",
					approve: "Approve",
					reject: "Reject",
					allow: "Allow",
					deny: "Deny",
				},
				noDetails: "No details available",
					currentServers: {
						title: "Current Servers",
						import: "Import from Config",
						configuredLabel: "configured",
						empty: "No servers extracted from current config.",
					},
				},
				form: {
					titleCreate: "Add Client Record",
					titleEdit: "Edit Client Record",
					descriptionCreate: "Create a client record with its management shape and metadata.",
					descriptionEdit: "Update this client record and its management settings.",
					tabs: { basic: "Basic", meta: "Meta" },
					connectionShape: {
						label: "Client Shape",
						description:
							"Choose whether this client has a writable local config file or is a non-writable remote/unknown client.",
						options: {
							localWithConfig: "Local + Config",
							localWithoutConfig: "Local / Unknown Config",
							remoteHttp: "Remote HTTP",
						},
					},
					transportSupport: {
						label: "Transport Support",
						placeholder: "Select supported transports",
						empty: "No transports found.",
						description:
							"Choose which runtime transports this client supports. This array is the only source used to constrain hosted/unify transport selection.",
						options: {
							stdio: "STDIO",
							streamableHttpLegacy: "Streamable HTTP",
							sseLegacy: "SSE (Legacy)",
						},
					},
					fields: {
						displayName: { label: "Client Name", placeholder: "Cursor Desktop" },
						identifier: {
							label: "Client ID",
							placeholder: "cursor-desktop",
							description:
								"Spaces and casing are normalized automatically when creating a new record.",
						},
						clientVersion: { label: "Client Version", placeholder: "optional" },
						configPath: {
							label: "Config File Path",
							placeholder: "~/.cursor/mcp.json",
							description:
								"A writable local config path enables MCPMate to manage this client through file-based configuration operations.",
							unavailableHint:
								"This client does not currently have a writable local config path, so file-based configuration management is unavailable.",
							browse: "Choose…",
							browseAria: "Browse for configuration file on disk",
							dialogTitle: "Select configuration file",
							pickFailedTitle: "Unable to read selected file",
							webPickInfoTitle: "Browser file access",
							webPickInfoDescription:
								"Your browser cannot read the absolute path automatically. Please paste it manually if needed.",
						},
						configFileParse: {
							format: { label: "Format" },
							containerType: { label: "Container Type" },
							containerKeys: { label: "Container Keys (JSON Path)" },
							validation: {
								formatMatch: "Format matches",
								formatMismatch: "Format mismatch",
								containerFound: "Container found",
								containerMissing: "Container missing",
								serversDetected: "{{count}} server(s) detected",
								noServers: "No servers detected",
								checking: "Inspecting..."
							},
							preview: "Preview",
							previewTitle: "Config Preview",
							autoDetect: "Auto-Detect Rules",
							applyDetected: "Apply Detected Rules"
						},
						formatRulesJsonText: {
							label: "Format Rules (JSON)",
							placeholder: "Paste JSON format rules here",
							description: "Advanced: Fine-grained transport format rules as JSON. Leave empty to reset to defaults.",
						},
						logoUrl: { label: "Logo URL", placeholder: "https://example.com/logo.png" },
						homepageUrl: { label: "Homepage URL", placeholder: "https://example.com" },
						docsUrl: { label: "Docs URL", placeholder: "https://docs.example.com" },
						supportUrl: { label: "Support URL", placeholder: "https://support.example.com" },
						description: {
							label: "Description",
							placeholder: "A short summary of this client.",
							description:
								"These meta fields are stored for display and guidance; the old template files remain only as compatibility seeds.",
						},
					},
					buttons: { cancel: "Cancel", create: "Create Record", save: "Save Changes", delete: "Delete" },
					confirmDelete: {
						title: "Delete Client Record",
						description: "Are you sure you want to delete this client record? This action cannot be undone.",
						confirm: "Delete",
						cancel: "Cancel",
					},
					notifications: {
						createSuccess: { title: "Client record created", message: "The client record has been created." },
						editSuccess: { title: "Client record updated", message: "The client record has been updated." },
						deleteSuccess: { title: "Client record deleted", message: "The client record has been deleted." },
						createBackupPolicyFailed: {
							message:
								"Client record was created, but applying initial backup policy failed. You can retry in Backup settings.",
						},
						formatRulesJsonParseError: "Failed to parse format rules JSON. Please check the syntax and try again.",
						saveFailed: { title: "Unable to save client record" },
						deleteFailed: {
							title: "Unable to delete client record",
							messageMissingIdentifier: "Client identifier is missing.",
						},
					},
				},
				configuration: {
				title: "Configuration Mode",
				description:
					"If you don't understand what this means, please don't make any changes and keep the current settings.",
					warnings: {
						mixedRouting: "Mixed routing: splitting stateful workflows may cause issues.",
					},
				writeTargetRequiredReason:
					"Applying governance to the client configuration requires a verified writable local MCP config file.",
				applyRequiresApprovedReason:
					"Applying client configuration requires an approved governance state and a verified local config target.",
				managementSettingsPendingReason:
					"Save management settings after this client leaves pending approval.",
				apply: "Apply",
				reapply: "Re-apply",
				sections: {
					mode: {
						title: "1. Management Mode",
						descriptions: {
						unify:
							"Unify starts with builtin MCP tools only and works with capabilities from globally enabled servers during the current session.",
							hosted:
								"Hosted keeps a durable managed configuration for this client and remembers the selected working state.",
							transparent:
								"MCPMate writes the selected profile servers directly into this client's MCP configuration and does not preserve capability-level controls.",
						},
						managedDisabledReason:
							"Hosted and Unify require at least one supported transport.",
						transparentDisabledReason:
							"Transparent requires a writable local config path.",
						options: {
						unify: "Unify",
							hosted: "Hosted Mode",
							transparent: "Transparent",
						},
					},
					source: {
						title: "2. Configuration",
						titleTransparent: "2. Configuration",
						unifyRouteModes: {
							broker_only: "Broker Only",
							server_live: "Server Level",
							capability_level: "Capability Level",
						},
						descriptions: {
						unify:
							"Unify does not use dashboard profile selection. Use the builtin UCAN tools during the session to browse and call capabilities from globally enabled servers.",
							unify_broker_only:
								"All capabilities are kept behind the UCAN broker catalog. Builtin MCP tools will browse and call capabilities from globally enabled servers.",
							unify_server_live:
								"Directly expose all capabilities from selected eligible servers to this client. Exposed capabilities are excluded from the UCAN catalog.",
							unify_capability_level:
								"Directly expose only selected capabilities (tools, prompts, resources, templates) from eligible servers to this client. (Advanced)",
							default: "Review the profiles that are currently active for this client runtime.",
							profile: "Browse the shared scene library and choose the exact working set for this client.",
						custom: "Create client-specific adjustments on top of the current unify-mode working state.",
							transparentDefault:
								"Write the servers from all currently activated profiles directly into this client's MCP configuration.",
							transparentProfile:
								"Write the servers from the selected shared profiles directly into this client's MCP configuration.",
							transparentCustom:
								"Write the servers from the client-specific custom profile directly into this client's MCP configuration.",
						},
					options: {
						default: "Active",
						profile: "Profiles",
						custom: "Customized",
					},
						statusLabel: {
							default: "",
							profile: "",
							custom: "",
						},
					},
					profiles: {
					title: "3. Profiles",
						descriptions: {
						unify:
							"Unify does not maintain a profile working set here. Use profiles for Hosted Mode or Transparent Mode workflows instead.",
							default:
								"Review the profiles that are already active for this client runtime. This view is read-only to keep the active scene set consistent.",
							profile:
								"Choose the reusable shared profiles that define this client's working set.",
							custom:
								"Create and maintain client-specific overrides for the current working state.",
							transparentDefault:
								"Transparent mode will write the enabled servers from all currently activated profiles directly into this client's MCP configuration.",
							transparentProfile:
								"Select which shared profiles contribute enabled servers to this client's MCP configuration in transparent mode.",
							transparentCustom:
								"Transparent mode uses only the enabled servers from this client-specific custom profile when writing the MCP configuration.",
						},
						empty: {
							active: "No active profiles found",
							shared: "No shared profiles found",
						},
						ghost: {
							titleCustom: "Customize current state",
							titleDefault: "Open profiles library",
							subtitleCustom: "Create and manage client-specific overrides for the current workspace",
							subtitleCustomTransparent:
								"Configure which servers should be written into this client directly.",
							subtitleDefault: "Browse reusable shared scenes and edit them from the profiles page",
						},
					},
					unify: {
						title: "2. Configuration",
						description:
							"Unify starts with builtin MCP tools only. It uses session-local builtin tooling to browse capabilities from globally enabled servers and resets when the session ends.",
						items: {
							builtinOnly: "Builtin tools only",
							sessionScoped: "Session-local builtin flow",
							noFurtherSetup: "No further setup in the dashboard",
						},
					},
					exposure: {
						title: "3. Direct Exposure",
						descriptions: {
							broker_only:
								"All enabled MCP servers, including servers marked for direct exposure, remain reachable through the builtin UCAN tools in Broker Only mode.",
							server_live:
								"Select the eligible servers whose tools, prompts, resources, and resource templates should be exposed directly to the client.",
							capability_level:
								"Select eligible servers and configure direct exposure at capability level across tools, prompts, resources, and templates. (Advanced)",
						},
						labels: {
							ucanRoutingDescription:
								"In Broker Only mode, all enabled MCP servers — including servers marked for direct exposure — are still accessed through the UCAN catalog and call tools.",
						},
						empty: {
							no_eligible:
								"No eligible servers found. Enable Unify Direct Exposure on a server first.",
						},
					},
				},
			labels: {
				noDescription: "No description",
				openProfileDetail: "Open profile details",
				servers: "Servers",
				tools: "Tools",
				resources: "Resources",
				prompts: "Prompts",
				resourceTemplates: "Resource templates",
				toolsExposed: "tools exposed",
				capabilitiesExposed: "capabilities exposed",
			},
			nonWritableReason: "This record is currently non-writable.",
			transportOptions: {
				auto: "Auto",
				stdio: "STDIO",
				streamableHttp: "Streamable HTTP",
				streamableHttpLegacy: "Streamable HTTP",
				sseLegacy: "SSE (Legacy)",
			},
			form: {
				fields: {
					configPath: {
						placeholder: "~/.cursor/mcp.json",
					},
					logoUrl: {
						placeholder: "https://example.com/logo.png",
					},
					homepageUrl: {
						placeholder: "https://example.com",
					},
					docsUrl: {
						placeholder: "https://docs.example.com",
					},
					supportUrl: {
						placeholder: "https://support.example.com",
					},
				},
			},
		},
			backups: {
				title: "Backups",
				description: "Restore or delete configuration snapshots.",
				buttons: {
					refresh: "Refresh",
					selectAll: "Select all",
					clear: "Clear",
					deleteSelected: "Delete selected ({{count}})",
					restore: "Restore",
					delete: "Delete",
				},
				empty: "No backups.",
				emptyDisabledByPolicy: "Backups are currently disabled by system policy.",
				bulk: {
					title: "Delete Selected Backups",
					description:
						"Are you sure you want to delete {{count}} backup(s)? This action cannot be undone.",
				},
			},
			logs: {
				title: "Logs",
				description: "Runtime warnings and backend notes for this client.",
				searchPlaceholder: "Search logs...",
				refresh: "Refresh Logs",
				expand: "Expand Logs",
				collapse: "Collapse Logs",
				loading: "Loading logs...",
				headers: {
					timestamp: "Timestamp",
					action: "Action",
					category: "Category",
					status: "Status",
					target: "Target",
				},
				empty: "No log entries recorded for this client yet.",
			},
			confirm: {
				deleteTitle: "Delete Backup",
				restoreTitle: "Restore Backup",
				deleteDescription:
					"Are you sure you want to delete this backup? This action cannot be undone.",
				restoreDescription:
					"Restore the local client configuration file from the selected backup? MCPMate management mode and capability settings stay unchanged.",
				deleteLabel: "Delete",
				restoreLabel: "Restore",
				cancelLabel: "Cancel",
			},
			policy: {
				title: "Backup Policy",
				fields: {
					policy: "Policy",
					policyDescription:
						'Backup retention strategy. For now, only "keep_n" is supported, which keeps at most N recent backups and prunes older ones.',
					options: {
						keepN: "keep_n",
					},
					limit: "Limit",
					limitDescription:
						"Maximum number of backups to keep for this client. Set to 0 for no limit.",
				},
				buttons: {
					save: "Save Policy",
				},
			},
			importPreview: {
				title: "Import Preview",
				description: "Summary of servers detected from current client config.",
				fields: {
					attempted: "Attempted",
					imported: "Imported",
					skipped: "Skipped",
					failed: "Failed",
				},
				noPreview: "No preview data.",
				sections: {
					servers: "Servers to import",
					errors: "Errors",
					raw: "Raw preview JSON",
					stats:
						"tools: {{tools}} • resources: {{resources}} • templates: {{templates}} • prompts: {{prompts}}",
				},
				buttons: {
					close: "Close",
					apply: "Apply Import",
					preview: "Preview",
				},
				states: {
					noImportNeeded: "No import needed",
				},
			},
				notifications: {
					reviewSuccess: {
						title: "Success",
						messageApproved: "Record approved successfully.",
						messageRejected: "Record rejected successfully.",
					},
					reviewFailed: {
						title: "Review failed",
					},
					previewReady: {
						title: "Preview ready",
						message: "Review the diff before applying.",
					noChanges: "No changes detected in this configuration.",
				},
				applied: {
					title: "Applied",
					message: "Configuration applied",
				},
				managementSaved: {
					title: "Saved",
					message:
						"Management settings were saved in MCPMate. Local client configuration was not updated.",
				},
				applyFailed: {
					title: "Apply failed",
				},
				imported: {
					title: "Imported",
					message: "{{count}} server(s) imported successfully",
				},
				nothingToImport: {
					title: "Nothing to import",
					message: "All entries were skipped or no importable servers found.",
				},
				importFailed: {
					title: "Import failed",
				},
				refreshed: {
					title: "Refreshed",
					message: "Detection refreshed",
				},
				refreshFailed: {
					title: "Refresh failed",
				},
				managedUpdated: {
					title: "Updated",
					message: "Managed state changed",
				},
				managedFailed: {
					title: "Update failed",
				},
					governanceUpdated: {
						title: "Updated",
						message: "Client governance state changed",
					},
				governanceFailed: {
					title: "Update failed",
				},
				governanceAllowed: {
					title: "Updated",
					message: "Client governance is now allowed.",
				},
				governanceDenied: {
					title: "Updated",
					message: "Client governance is now denied.",
				},
				previewFailed: {
					title: "Preview failed",
				},
				restored: {
					title: "Restored",
					message: "Configuration restored from backup",
				},
				restoreFailed: {
					title: "Restore failed",
				},
				deleted: {
					title: "Deleted",
					message: "Backup deleted",
				},
				deleteFailed: {
					title: "Delete failed",
				},
				bulkDeleted: {
					title: "Deleted",
					message: "Selected backups have been deleted",
				},
				bulkDeleteFailed: {
					title: "Bulk delete failed",
				},
				saved: {
					title: "Saved",
					message: "Backup policy updated",
				},
				saveFailed: {
					title: "Save failed",
				},
			},
		},
	},
    "zh-CN": {
        title: "发现并管理客户端连接与可见性",
		toolbar: {
			search: {
				placeholder: "搜索客户端...",
				fields: {
					displayName: "显示名称",
					identifier: "标识符",
					description: "描述",
				},
			},
			filters: {
				title: "筛选",
				options: {
					all: "全部",
					allowed: "已允许",
					pending: "待审批",
					denied: "已拒绝",
				},
			},
			sort: {
				options: {
					displayName: "名称",
					approvalStatus: "审批状态",
					managed: "管理状态",
				},
			},
			actions: {
				refresh: {
					title: "刷新",
					notificationTitle: "已触发刷新",
					notificationMessage: "将同步最新客户端状态",
				},
				add: {
					title: "新增客户端",
					notificationTitle: "功能开发中",
					notificationMessage: "该功能正在实现，敬请期待",
				},
			},
		},
		statsCards: {
			total: {
				title: "客户端总数",
				description: "已发现",
			},
			detected: {
				title: "已检测",
				description: "已安装",
			},
			managed: {
				title: "管理中",
				description: "管理已启用",
			},
			configured: {
				title: "已配置",
				description: "存在 MCP 配置",
			},
		},
		notifications: {
			managementUpdated: {
				title: "已更新",
				message: "客户端管理状态已更新",
			},
			operationFailed: {
				title: "操作失败",
			},
		},
		entity: {
			fallbackName: "客户端",
			stats: {
				config: "配置",
				servers: "服务器",
				managed: "管理",
				detected: "检测",
			},
			config: {
				notConfigured: "未配置",
			},
			bottomTags: {
				servers: "服务器：{{count}}",
			},
			status: {
				detected: "已检测",
				notDetected: "未检测",
			},
			badge: {
				detected: "已检测",
				notDetected: "未检测",
			},
		},
		states: {
			on: "开启",
			off: "关闭",
			yes: "是",
			no: "否",
			present: "存在",
			missing: "缺失",
		},
		emptyState: {
			title: "未找到任何客户端",
			description: "请确认 MCPMate 后端已运行并开启检测",
		},
		detail: {
			noIdentifier: "未提供客户端标识符。",
			badges: {
				managed: "管理中",
				unmanaged: "未管理",
				detected: "已检测",
				notDetected: "未检测",
			},
			tabs: {
				overview: "概览",
				configuration: "配置",
				backups: "备份",
				logs: "日志",
			},
			directExposure: {
				title: "能力级直达",
				badge: "直达暴露",
				description: "调整本服务器有哪些能力应直接暴露给当前客户端。",
				serverDescriptionFallback: "选择本服务器中应直接暴露给客户端的能力。",
				toolsTitle: "工具",
				toolsSummary: "已直达 {{enabled}} / {{total}} 个能力",
				tabs: {
					tools: "工具",
					prompts: "提示",
					resources: "资源",
					templates: "模板",
					toolsWithCounts: "工具（{{enabled}}/{{total}}）",
					promptsWithCounts: "提示（{{enabled}}/{{total}}）",
					resourcesWithCounts: "资源（{{enabled}}/{{total}}）",
					templatesWithCounts: "模板（{{enabled}}/{{total}}）",
				},
				searchPlaceholders: {
					tools: "搜索工具…",
					prompts: "搜索提示…",
					resources: "搜索资源…",
					templates: "搜索模板…",
				},
				empty: {
					tools: "未找到该服务器的工具。",
					prompts: "未找到该服务器的提示。",
					resources: "未找到该服务器的资源。",
					templates: "未找到该服务器的模板。",
				},
				statusPlaceholder: "状态",
				filters: {
					status: {
						all: "全部",
						enabled: "已启用",
						disabled: "已停用",
					},
				},
				buttons: {
					selectAll: "全选",
					clearSelection: "清空",
					enable: "启用",
					disable: "停用",
				},
				notifications: {
					savedTitle: "直达能力已更新",
					savedMessage: "直达能力暴露配置已更新。",
					saveFailedTitle: "无法更新直达能力",
				},
			},
			overview: {
				labels: {
					configPath: "配置路径",
					lastModified: "最近修改",
					supportedTransports: "传输协议",
					homepage: "主页",
					docs: "文档",
					support: "支持",
				},
				buttons: {
					edit: "编辑",
					refresh: "刷新",
					enable: "启用",
					disable: "停用",
					approve: "批准",
					reject: "拒绝",
					allow: "允许",
					deny: "拒绝治理",
				},
				noDetails: "暂无详细信息",
					currentServers: {
						title: "当前服务器",
						import: "从配置导入",
						configuredLabel: "已配置",
						empty: "未从当前配置解析到服务器。",
					},
				},
				form: {
					titleCreate: "新增客户端记录",
					titleEdit: "编辑客户端记录",
					descriptionCreate: "创建一个带有管理形态与元数据的客户端记录。",
					descriptionEdit: "更新该客户端记录及其管理设置。",
					tabs: { basic: "基础", meta: "元数据" },
					connectionShape: {
						label: "客户端形态",
						description: "选择该客户端是否具备可写本地配置文件，或属于不可写的远程/未知客户端。",
						options: {
							localWithConfig: "本地 + 配置文件",
							localWithoutConfig: "本地 / 未知配置",
							remoteHttp: "远程 HTTP",
						},
					},
					transportSupport: {
						label: "传输支持",
						placeholder: "选择支持的传输方式",
						empty: "未找到传输方式。",
						description: "选择该客户端支持的运行时传输方式。该数组会作为 hosted / unify 传输选择的唯一约束来源。",
						options: {
							stdio: "STDIO",
							streamableHttpLegacy: "Streamable HTTP",
							sseLegacy: "SSE（旧版兼容）",
						},
					},
					fields: {
						displayName: { label: "客户端名称", placeholder: "Cursor Desktop" },
						identifier: { label: "客户端 ID", placeholder: "cursor-desktop", description: "创建新记录时，空格和大小写会自动规范化。" },
						clientVersion: { label: "客户端版本", placeholder: "可选" },
						configPath: {
							label: "配置文件路径",
							placeholder: "~/.cursor/mcp.json",
							description: "可写的本地配置路径会让 MCPMate 能通过文件配置操作来管理该客户端。",
							unavailableHint: "该客户端当前没有可写的本地配置路径，因此暂时无法进行基于文件的配置管理。",
							browse: "选择…",
							browseAria: "从磁盘选择配置文件",
							dialogTitle: "选择配置文件",
							pickFailedTitle: "无法读取所选文件",
							webPickInfoTitle: "浏览器文件访问",
							webPickInfoDescription: "浏览器无法自动读取绝对路径，如有需要请手动粘贴。",
						},
						configFileParse: {
							format: { label: "文件格式" },
							containerType: { label: "容器类型" },
							containerKeys: { label: "容器键名 (JSON Path)" },
							validation: {
								formatMatch: "格式匹配",
								formatMismatch: "格式不匹配",
								containerFound: "找到容器",
								containerMissing: "未找到容器",
								serversDetected: "检测到 {{count}} 个服务器",
								noServers: "未检测到服务器",
								checking: "检查中..."
							},
							preview: "预览",
							previewTitle: "配置预览",
							autoDetect: "自动检测规则",
							applyDetected: "应用检测到的规则"
						},
						formatRulesJsonText: {
							label: "格式规则（JSON）",
							placeholder: "在此粘贴 JSON 格式规则",
							description: "高级：为传输格式定义细粒度规则（JSON 格式）。留空将重置为默认值。",
						},
						logoUrl: { label: "Logo 地址", placeholder: "https://example.com/logo.png" },
						homepageUrl: { label: "主页地址", placeholder: "https://example.com" },
						docsUrl: { label: "文档地址", placeholder: "https://docs.example.com" },
						supportUrl: { label: "支持地址", placeholder: "https://support.example.com" },
						description: {
							label: "描述",
							placeholder: "简要描述这个客户端。",
							description: "这些元数据字段仅用于展示与提示；旧模板文件现在只保留兼容性 seed 作用。",
						},
					},
					buttons: { cancel: "取消", create: "创建记录", save: "保存更改", delete: "删除" },
					confirmDelete: {
						title: "删除客户端记录",
						description: "确定要删除这条客户端记录吗？此操作不可撤销。",
						confirm: "删除",
						cancel: "取消",
					},
					notifications: {
						createSuccess: { title: "客户端记录已创建", message: "客户端记录已创建。" },
						editSuccess: { title: "客户端记录已更新", message: "客户端记录已更新。" },
						deleteSuccess: { title: "客户端记录已删除", message: "客户端记录已删除。" },
						createBackupPolicyFailed: {
							message: "客户端记录已创建，但初始化备份策略失败。你可以稍后在备份设置中重试。",
						},
						formatRulesJsonParseError: "格式规则 JSON 解析失败。请检查语法后重试。",
						saveFailed: { title: "无法保存客户端记录" },
						deleteFailed: {
							title: "无法删除客户端记录",
							messageMissingIdentifier: "缺少客户端标识符。",
						},
					},
				},
				configuration: {
				title: "配置模式",
				description: "若不清楚含义，请勿修改并保持现有设置。",
					warnings: {
						mixedRouting: "混合路由：若把有状态工作流拆成代理与直连调用，可能出现问题。",
					},
				writeTargetRequiredReason: "要把治理配置真正应用到客户端配置文件，必须先确认一个已验证且可写的本地 MCP 配置文件。",
				applyRequiresApprovedReason: "要把客户端配置真正应用落盘，必须先处于已允许治理状态，并且拥有一个已验证的本地配置目标。",
				managementSettingsPendingReason: "请在该客户端结束待审批状态后再保存管理设置。",
				apply: "应用",
				reapply: "重新应用",
				sections: {
					mode: {
						title: "1. 管理模式",
						descriptions: {
						unify:
							"统一模式初始仅提供内建 MCP 工具，并在当前会话中面向全局启用服务器的能力工作。",
							hosted:
								"托管模式会为该客户端保留持久化托管配置，并记住当前选择的工作状态。",
							transparent:
								"MCPMate 会将所选配置集中的服务器直接写入该客户端的 MCP 配置，且不会保留能力层级的控制。",
						},
						managedDisabledReason: "托管模式与统一模式至少需要一种后端已声明支持的传输方式。",
						transparentDisabledReason: "透明模式需要可写的本地配置文件路径。",
						options: {
						unify: "统一模式",
							hosted: "托管模式",
							transparent: "透明模式",
						},
					},
					source: {
						title: "2. 配置详情",
						titleTransparent: "2. 配置详情",
						unifyRouteModes: {
						broker_only: "全部代理",
						server_live: "服务器直达",
						capability_level: "能力级直达",
						},
						descriptions: {
						unify: "统一模式不使用仪表板中的配置集选择。请在当前会话内通过内建 UCAN 工具浏览并调用来自全局启用服务器的能力。",
							unify_broker_only:
								"所有能力均通过 UCAN 代理目录访问。内建 MCP 工具会浏览并调用来自全局启用服务器的能力。",
							unify_server_live:
								"将所选符合条件的服务器的全部能力直接暴露给该客户端。已暴露的能力不会出现在 UCAN 目录中。",
							unify_capability_level:
								"仅从符合条件的服务器中选择部分能力（工具、提示、资源、模板）直接暴露给该客户端。（进阶）",
							default: "查看当前已对该客户端运行态生效的配置集。",
							profile: "浏览共享场景库，并为该客户端选择精确的工作集。",
						custom: "在当前统一模式工作状态之上创建客户端专属调整。",
						transparentDefault:
							"将当前所有已激活配置集中的服务器直接写入该客户端的 MCP 配置。",
						transparentProfile:
							"将所选共享配置集中的服务器直接写入该客户端的 MCP 配置。",
						transparentCustom:
							"将该客户端专属自定义配置集中的服务器直接写入该客户端的 MCP 配置。",
						},
					options: {
						default: "当前生效",
						profile: "配置集库",
						custom: "自定义工作区",
					},
					statusLabel: {
						default: "",
						profile: "",
						custom: "",
					},
				},
					profiles: {
					title: "3. 配置集",
						descriptions: {
						unify: "统一模式不会在这里维护配置集工作集。配置集仅用于托管模式或透明模式工作流。",
							default:
								"查看当前已对该客户端运行态生效的配置集。为保持场景一致性，此视图为只读。",
							profile: "选择定义该客户端工作集的可复用共享配置集。",
							custom: "为当前工作状态创建并维护客户端专属覆盖项。",
							transparentDefault:
								"透明模式会将当前所有已激活配置集中已启用的服务器直接写入该客户端的 MCP 配置。",
							transparentProfile:
								"选择哪些共享配置集为透明模式下该客户端的 MCP 配置提供已启用服务器。",
							transparentCustom:
								"透明模式在写入 MCP 配置时，仅使用该客户端专属自定义配置集中已启用的服务器。",
						},
						empty: {
							active: "未找到已激活的配置集",
							shared: "未找到共享配置集",
						},
				ghost: {
					titleCustom: "自定义当前状态",
					titleDefault: "打开配置集库",
					subtitleCustom: "为当前工作集创建并管理客户端专属覆盖项",
					subtitleCustomTransparent: "配置哪些服务器会被直接写入当前客户端。",
					subtitleDefault: "浏览可复用共享场景，并在配置集页面中维护它们",
				},
			},
					unify: {
						title: "2. 配置详情",
						description:
							"统一模式初始仅提供内建 MCP 工具。它会在当前 MCP 会话中通过会话内建工具浏览全局启用服务器的 capabilities，并在会话结束后自动重置。",
						items: {
							builtinOnly: "仅内建工具",
							sessionScoped: "会话内建流程",
							noFurtherSetup: "仪表板中无需进一步设置",
						},
					},
					exposure: {
						title: "3. 直达暴露",
						descriptions: {
							broker_only:
								"在仅代理路由模式下，包括标记为直达暴露在内的所有已启用 MCP 服务器，仍通过内建 UCAN 工具访问。",
							server_live:
								"选择应将工具、提示、资源与资源模板直接暴露给该客户端的符合条件服务器。",
							capability_level:
								"选择符合条件的服务器，并在工具、提示、资源、模板四类能力上进行能力级直达配置。（进阶）",
						},
						labels: {
							ucanRoutingDescription:
								"在仅代理路由模式下，包括标记为直达暴露的服务器在内，所有已启用的 MCP 服务器仍通过 UCAN 目录与工具调用访问。",
						},
						empty: {
							no_eligible: "暂无可选符合条件服务器。请先在服务器详情中启用统一模式直达暴露。",
						},
					},
				},
			labels: {
				noDescription: "暂无描述",
				openProfileDetail: "打开配置集详情",
				servers: "服务器",
				tools: "工具",
				resources: "资源",
				prompts: "提示",
				resourceTemplates: "资源模板",
				toolsExposed: "个工具已直达",
				capabilitiesExposed: "项能力已直达",
			},
			nonWritableReason: "该记录当前不可写。",
			transportOptions: {
				auto: "自动",
				stdio: "STDIO",
				streamableHttp: "Streamable HTTP",
				streamableHttpLegacy: "Streamable HTTP",
				sseLegacy: "SSE（旧版兼容）",
			},
			form: {
				fields: {
					configPath: {
						placeholder: "~/.cursor/mcp.json",
					},
					logoUrl: {
						placeholder: "https://example.com/logo.png",
					},
					homepageUrl: {
						placeholder: "https://example.com",
					},
					docsUrl: {
						placeholder: "https://docs.example.com",
					},
					supportUrl: {
						placeholder: "https://support.example.com",
					},
				},
			},
		},
			backups: {
				title: "备份",
				description: "恢复或删除配置快照。",
				buttons: {
					refresh: "刷新",
					selectAll: "全选",
					clear: "清空",
					deleteSelected: "删除（{{count}}）",
					restore: "恢复",
					delete: "删除",
				},
				empty: "暂无备份。",
				emptyDisabledByPolicy: "系统策略当前已禁用备份。",
				bulk: {
					title: "删除备份",
					description: "确定要删除 {{count}} 个备份吗？该操作不可撤销。",
				},
			},
			logs: {
				title: "日志",
				description: "展示该客户端关联的运行日志与审计事件。",
				searchPlaceholder: "搜索日志...",
				refresh: "刷新日志",
				expand: "展开日志",
				collapse: "收起日志",
				loading: "正在加载日志...",
				headers: {
					timestamp: "时间",
					action: "动作",
					category: "类别",
					status: "状态",
					target: "目标",
				},
				empty: "暂未记录任何日志。",
			},
			confirm: {
				deleteTitle: "删除备份",
				restoreTitle: "恢复备份",
				deleteDescription: "确定要删除该备份吗？此操作不可撤销。",
				restoreDescription:
					"要从选定备份恢复本地客户端配置文件吗？MCPMate 的管理模式与能力设置不会改变。",
				deleteLabel: "删除",
				restoreLabel: "恢复",
				cancelLabel: "取消",
			},
			policy: {
				title: "备份策略",
				fields: {
					policy: "策略",
					policyDescription:
						"备份保留策略。目前仅支持“keep_n”，会保留最近 N 个备份并清理更早的备份。",
					options: {
						keepN: "keep_n",
					},
					limit: "上限",
					limitDescription: "该客户端保留的备份数量上限，设置为 0 表示不限。",
				},
				buttons: {
					save: "保存策略",
				},
			},
			importPreview: {
				title: "导入预览",
				description: "概览当前客户端配置中检测到的服务器。",
				fields: {
					attempted: "已尝试",
					imported: "已导入",
					skipped: "已跳过",
					failed: "失败",
				},
				noPreview: "暂无预览数据。",
				sections: {
					servers: "待导入服务器",
					errors: "错误信息",
					raw: "原始预览 JSON",
					stats:
						"工具：{{tools}} • 资源：{{resources}} • 模板：{{templates}} • 提示：{{prompts}}",
				},
				buttons: {
					close: "关闭",
					apply: "应用导入",
					preview: "生成预览",
				},
				states: {
					noImportNeeded: "无需导入",
				},
			},
				notifications: {
					reviewSuccess: {
						title: "成功",
						messageApproved: "记录已成功批准。",
						messageRejected: "记录已成功拒绝。",
					},
					reviewFailed: {
						title: "审批失败",
					},
					previewReady: {
						title: "预览已就绪",
						message: "请在应用前先查看差异。",
					noChanges: "当前配置未产生任何变化。",
				},
				applied: {
					title: "已应用",
					message: "配置已应用",
				},
				managementSaved: {
					title: "已保存",
					message: "管理设置已保存到 MCPMate，本地客户端配置未更新。",
				},
				applyFailed: {
					title: "应用失败",
				},
				imported: {
					title: "导入完成",
					message: "成功导入 {{count}} 个服务器",
				},
				nothingToImport: {
					title: "无需导入",
					message: "所有条目已跳过或没有可导入的服务器。",
				},
				importFailed: {
					title: "导入失败",
				},
				refreshed: {
					title: "已刷新",
					message: "检测状态已刷新",
				},
				refreshFailed: {
					title: "刷新失败",
				},
				managedUpdated: {
					title: "已更新",
					message: "托管状态已变更",
				},
				managedFailed: {
					title: "更新失败",
				},
					governanceUpdated: {
						title: "已更新",
						message: "客户端治理状态已变更",
					},
				governanceFailed: {
					title: "更新失败",
				},
				governanceAllowed: {
					title: "已更新",
					message: "该客户端现已允许。",
				},
				governanceDenied: {
					title: "已更新",
					message: "该客户端现已禁行。",
				},
				previewFailed: {
					title: "预览失败",
				},
				restored: {
					title: "恢复完成",
					message: "已从备份恢复配置",
				},
				restoreFailed: {
					title: "恢复失败",
				},
				deleted: {
					title: "删除完成",
					message: "备份已删除",
				},
				deleteFailed: {
					title: "删除失败",
				},
				bulkDeleted: {
					title: "删除完成",
					message: "已删除所选备份",
				},
				bulkDeleteFailed: {
					title: "批量删除失败",
				},
				saved: {
					title: "已保存",
					message: "备份策略已更新",
				},
				saveFailed: {
					title: "保存失败",
				},
			},
		},
	},
    "ja-JP": {
        title: "クライアント接続と可視性の管理",
		toolbar: {
			search: {
				placeholder: "クライアントを検索...",
				fields: {
					displayName: "表示名",
					identifier: "識別子",
					description: "説明",
				},
			},
			filters: {
				title: "フィルター",
				options: {
					all: "すべて",
					allowed: "許可済み",
					pending: "承認待ち",
					denied: "拒否",
				},
			},
			sort: {
				options: {
					displayName: "名前",
					approvalStatus: "承認状態",
					managed: "管理状況",
				},
			},
			actions: {
				refresh: {
					title: "更新",
					notificationTitle: "更新を開始しました",
					notificationMessage: "最新のクライアント状態を同期します",
				},
				add: {
					title: "クライアントを追加",
					notificationTitle: "開発中の機能",
					notificationMessage: "機能を開発中です。しばらくお待ちください",
				},
			},
		},
		statsCards: {
			total: {
				title: "クライアント総数",
				description: "検出済み",
			},
			detected: {
				title: "検出",
				description: "インストール済み",
			},
			managed: {
				title: "管理中",
				description: "管理が有効",
			},
			configured: {
				title: "設定済み",
				description: "MCP 設定あり",
			},
		},
		notifications: {
			managementUpdated: {
				title: "更新しました",
				message: "クライアントの管理状態を更新しました",
			},
			operationFailed: {
				title: "操作に失敗しました",
			},
		},
		entity: {
			fallbackName: "クライアント",
			stats: {
				config: "設定",
				servers: "サーバー",
				managed: "管理",
				detected: "検出",
			},
			config: {
				notConfigured: "未設定",
			},
			bottomTags: {
				servers: "サーバー: {{count}}",
			},
			status: {
				detected: "検出済み",
				notDetected: "未検出",
			},
			badge: {
				detected: "検出済み",
				notDetected: "未検出",
			},
		},
		states: {
			on: "オン",
			off: "オフ",
			yes: "はい",
			no: "いいえ",
			present: "あり",
			missing: "なし",
		},
		emptyState: {
			title: "クライアントが見つかりません",
			description: "MCPMate バックエンドが動作し検出が有効か確認してください",
		},
		detail: {
			noIdentifier: "クライアント識別子が指定されていません。",
			badges: {
				managed: "管理中",
				unmanaged: "未管理",
				detected: "検出済み",
				notDetected: "未検出",
			},
			tabs: {
				overview: "概要",
				configuration: "設定",
				backups: "バックアップ",
				logs: "ログ",
			},
			directExposure: {
				title: "能力レベル直達",
				badge: "直接公開",
				description:
					"このサーバーのどのケイパビリティを現在のクライアントへ直接公開するかを調整します。",
				serverDescriptionFallback:
					"このサーバーからクライアントへ直接公開するケイパビリティを選択します。",
				toolsTitle: "ツール",
				toolsSummary: "直接公開 {{enabled}} / {{total}} ケイパビリティ",
				tabs: {
					tools: "ツール",
					prompts: "プロンプト",
					resources: "リソース",
					templates: "テンプレート",
					toolsWithCounts: "ツール（{{enabled}}/{{total}}）",
					promptsWithCounts: "プロンプト（{{enabled}}/{{total}}）",
					resourcesWithCounts: "リソース（{{enabled}}/{{total}}）",
					templatesWithCounts: "テンプレート（{{enabled}}/{{total}}）",
				},
				searchPlaceholders: {
					tools: "ツールを検索…",
					prompts: "プロンプトを検索…",
					resources: "リソースを検索…",
					templates: "テンプレートを検索…",
				},
				empty: {
					tools: "このサーバーにツールが見つかりません。",
					prompts: "このサーバーにプロンプトが見つかりません。",
					resources: "このサーバーにリソースが見つかりません。",
					templates: "このサーバーにテンプレートが見つかりません。",
				},
				statusPlaceholder: "状態",
				filters: {
					status: {
						all: "すべて",
						enabled: "有効",
						disabled: "無効",
					},
				},
				buttons: {
					selectAll: "すべて選択",
					clearSelection: "クリア",
					enable: "有効化",
					disable: "無効化",
				},
				notifications: {
					savedTitle: "直接公開ケイパビリティを更新しました",
					savedMessage: "直接公開するケイパビリティ設定を更新しました。",
					saveFailedTitle: "直接公開ケイパビリティを更新できません",
				},
			},
			overview: {
				labels: {
					configPath: "設定パス",
					lastModified: "最終更新",
					supportedTransports: "対応トランスポート",
					homepage: "ホームページ",
					docs: "ドキュメント",
					support: "サポート",
				},
				buttons: {
					edit: "編集",
					refresh: "更新",
					enable: "有効化",
					disable: "無効化",
					approve: "承認",
					reject: "拒否",
					allow: "許可",
					deny: "拒否",
				},
				noDetails: "詳細情報がありません",
					currentServers: {
						title: "現在のサーバー",
						import: "設定からインポート",
						configuredLabel: "設定済み",
						empty: "現在の設定からサーバーを取得できませんでした。",
					},
				},
				form: {
					titleCreate: "クライアントレコードを追加",
					titleEdit: "クライアントレコードを編集",
					descriptionCreate: "管理形態とメタデータを含むクライアントレコードを作成します。",
					descriptionEdit: "このクライアントレコードと管理設定を更新します。",
					tabs: { basic: "基本", meta: "メタデータ" },
					connectionShape: {
						label: "クライアント形態",
						description: "このクライアントが書き込み可能なローカル設定ファイルを持つか、書き込み不可のリモート/未知クライアントかを選択します。",
						options: {
							localWithConfig: "ローカル + 設定ファイル",
							localWithoutConfig: "ローカル / 不明な設定",
							remoteHttp: "リモート HTTP",
						},
					},
					transportSupport: {
						label: "対応トランスポート",
						placeholder: "対応するトランスポートを選択",
						empty: "トランスポートが見つかりません。",
						description: "このクライアントが対応するランタイムトランスポートを選択します。この配列は hosted / unify のトランスポート選択を制約する唯一の情報源です。",
						options: {
							stdio: "STDIO",
							streamableHttpLegacy: "ストリーミング HTTP",
							sseLegacy: "SSE（レガシー互換）",
						},
					},
					fields: {
						displayName: { label: "クライアント名", placeholder: "Cursor Desktop" },
						identifier: { label: "クライアント ID", placeholder: "cursor-desktop", description: "新規レコード作成時は、スペースと大文字小文字が自動で正規化されます。" },
						clientVersion: { label: "クライアントバージョン", placeholder: "任意" },
						configPath: {
							label: "設定ファイルパス",
							placeholder: "~/.cursor/mcp.json",
							description: "書き込み可能なローカル設定パスがある場合、MCPMate はファイルベースの設定操作でこのクライアントを管理できます。",
							unavailableHint: "このクライアントには現在書き込み可能なローカル設定パスがないため、ファイルベースの設定管理は利用できません。",
							browse: "選択…",
							browseAria: "ディスク上の設定ファイルを選択",
							dialogTitle: "設定ファイルを選択",
							pickFailedTitle: "選択したファイルを読み取れませんでした",
							webPickInfoTitle: "ブラウザのファイルアクセス",
							webPickInfoDescription: "ブラウザは絶対パスを自動取得できません。必要なら手動で貼り付けてください。",
						},
						configFileParse: {
							format: { label: "フォーマット" },
							containerType: { label: "コンテナタイプ" },
							containerKeys: { label: "コンテナキー (JSON Path)" },
							validation: {
								formatMatch: "フォーマット一致",
								formatMismatch: "フォーマット不一致",
								containerFound: "コンテナ検出",
								containerMissing: "コンテナ未検出",
								serversDetected: "{{count}}個のサーバーを検出",
								noServers: "サーバー未検出",
								checking: "検査中..."
							},
							preview: "プレビュー",
							previewTitle: "設定プレビュー",
							autoDetect: "ルール自動検出",
							applyDetected: "検出ルールを適用"
						},
						formatRulesJsonText: {
							label: "フォーマットルール（JSON）",
							placeholder: "JSON フォーマットルールをここに貼り付けてください",
							description: "高度な設定：トランスポート形式の細粒度ルールを JSON で定義します。空にするとデフォルト値にリセットされます。",
						},
						logoUrl: { label: "ロゴ URL", placeholder: "https://example.com/logo.png" },
						homepageUrl: { label: "ホームページ URL", placeholder: "https://example.com" },
						docsUrl: { label: "ドキュメント URL", placeholder: "https://docs.example.com" },
						supportUrl: { label: "サポート URL", placeholder: "https://support.example.com" },
						description: {
							label: "説明",
							placeholder: "このクライアントの概要を入力してください。",
							description: "これらのメタデータは表示とガイダンスのために保存されます。旧テンプレートファイルは互換性 seed としてのみ残ります。",
						},
					},
					buttons: { cancel: "キャンセル", create: "レコードを作成", save: "変更を保存", delete: "削除" },
					confirmDelete: {
						title: "クライアントレコードを削除",
						description: "このクライアントレコードを削除してもよろしいですか？この操作は元に戻せません。",
						confirm: "削除",
						cancel: "キャンセル",
					},
					notifications: {
						createSuccess: { title: "クライアントレコードを作成しました", message: "クライアントレコードを作成しました。" },
						editSuccess: { title: "クライアントレコードを更新しました", message: "クライアントレコードを更新しました。" },
						deleteSuccess: { title: "クライアントレコードを削除しました", message: "クライアントレコードを削除しました。" },
						createBackupPolicyFailed: {
							message:
								"クライアントレコードは作成されましたが、初期バックアップポリシーの適用に失敗しました。後でバックアップ設定から再試行できます。",
						},
						formatRulesJsonParseError: "フォーマットルール JSON の解析に失敗しました。構文を確認して再度お試しください。",
						saveFailed: { title: "クライアントレコードを保存できませんでした" },
						deleteFailed: {
							title: "クライアントレコードを削除できません",
							messageMissingIdentifier: "クライアント識別子がありません。",
						},
					},
				},
				configuration: {
				title: "設定モード",
				description: "意味が不明な場合は変更せず現状の設定を維持してください。",
					warnings: {
						mixedRouting: "混合ルーティング: ステートフルな手順を分割すると問題が起きる可能性があります。",
					},
				writeTargetRequiredReason:
					"クライアント設定ファイルへガバナンスを適用するには、検証済みで書き込み可能なローカル MCP 設定ファイルが必要です。",
				applyRequiresApprovedReason:
					"クライアント設定を適用するには、許可済みのガバナンス状態と検証済みのローカル設定対象が必要です。",
				managementSettingsPendingReason:
					"このクライアントが承認待ち状態を抜けてから管理設定を保存してください。",
				apply: "適用",
				reapply: "再適用",
				sections: {
					mode: {
						title: "1. 管理モード",
						descriptions: {
						unify:
							"統一モードは内蔵 MCP ツールのみで開始し、現在のセッションではグローバルに有効なサーバーのケイパビリティを扱います。",
							hosted:
								"ホスト型モードはこのクライアントの持続的な管理設定を保持し、現在のワーク状態を記憶します。",
							transparent:
								"MCPMate は選択したプロファイルのサーバーをこのクライアントの MCP 設定へ直接書き込み、ケイパビリティ単位の制御は保持しません。",
						},
						managedDisabledReason:
							"ホスト型モードと統一モードを使うには、少なくとも 1 つのサポート対象トランスポートが必要です。",
						transparentDisabledReason:
							"トランスペアレントモードには、書き込み可能なローカル設定ファイルのパスが必要です。",
						options: {
						unify: "統一モード",
							hosted: "ホスト型モード",
							transparent: "トランスペアレントモード",
						},
					},
					source: {
						title: "2. 設定",
						titleTransparent: "2. 設定",
						unifyRouteModes: {
							broker_only: "ブローカーのみ",
							server_live: "サーバーレベル",
							capability_level: "能力レベル直達",
						},
						descriptions: {
						unify:
							"統一モードではダッシュボード上のプロファイル選択を使いません。現在のセッションでは、内蔵 UCAN ツールでグローバルに有効なサーバーのケイパビリティを参照・呼び出します。",
							unify_broker_only:
								"すべてのケイパビリティは UCAN ブローカーカタログ越しに提供されます。内蔵 MCP ツールがグローバルに有効なサーバーのケイパビリティを参照・呼び出します。",
							unify_server_live:
								"選択した対象サーバーから、このクライアントへケイパビリティを直接公開します。公開されたケイパビリティは UCAN カタログには載りません。",
							unify_capability_level:
								"対象サーバーから選択したケイパビリティ（ツール、プロンプト、リソース、テンプレート）のみをこのクライアントへ直接公開します。（上級）",
							default: "このクライアントの実行時に現在有効なプロファイルを確認します。",
							profile: "共有シーンライブラリを参照し、このクライアントの正確なワークセットを選択します。",
						custom: "現在の統一モードのワーク状態の上にクライアント専用の調整を作成します。",
						transparentDefault:
							"現在有効なすべてのプロファイルのサーバーをこのクライアントの MCP 設定へ直接書き込みます。",
						transparentProfile:
							"選択した共有プロファイルのサーバーをこのクライアントの MCP 設定へ直接書き込みます。",
						transparentCustom:
							"このクライアント専用のカスタムプロファイルのサーバーをこのクライアントの MCP 設定へ直接書き込みます。",
						},
					options: {
						default: "有効なプロファイル",
						profile: "プロファイルライブラリ",
						custom: "カスタムワークスペース",
					},
					statusLabel: {
						default: "",
						profile: "",
						custom: "",
					},
				},
					profiles: {
					title: "3. プロファイル",
						descriptions: {
						unify:
							"統一モードではここでプロファイルのワークセットを維持しません。プロファイルはホスト型モードまたはトランスペアレントモードのワークフローで使用します。",
							default:
								"このクライアントの実行時にすでに有効なプロファイルを確認します。シーンの一貫性を保つため、このビューは読み取り専用です。",
							profile: "このクライアントのワークセットを定義する再利用可能な共有プロファイルを選択します。",
							custom: "現在のワーク状態に対するクライアント専用オーバーライドを作成・維持します。",
							transparentDefault:
								"トランスペアレントモードでは、現在有効なすべてのプロファイルの有効化されたサーバーをこのクライアントの MCP 設定へ直接書き込みます。",
							transparentProfile:
								"トランスペアレントモードでこのクライアントの MCP 設定に有効化されたサーバーを提供する共有プロファイルを選択します。",
							transparentCustom:
								"トランスペアレントモードで MCP 設定を書き込む際は、このクライアント専用カスタムプロファイルの有効化されたサーバーのみを使用します。",
						},
						empty: {
							active: "有効なプロファイルが見つかりません",
							shared: "共有プロファイルが見つかりません",
						},
						ghost: {
							titleCustom: "現在の状態をカスタマイズ",
							titleDefault: "プロファイルライブラリを開く",
							subtitleCustom: "現在のワークスペースに対するクライアント専用オーバーライドを作成・管理",
							subtitleCustomTransparent:
								"このクライアントへ直接書き込むサーバーを設定します。",
							subtitleDefault: "再利用可能な共有シーンを参照し、プロファイルページで編集します",
						},
					},
					unify: {
						title: "2. 設定",
						description:
							"統一モードは内蔵 MCP ツールのみで開始します。現在の MCP セッションでは、セッション内の内蔵フローでグローバルに有効なサーバーのケイパビリティを参照し、終了時に自動的にリセットされます。",
						items: {
							builtinOnly: "内蔵ツールのみ",
							sessionScoped: "セッション内の内蔵フロー",
							noFurtherSetup: "ダッシュボードで追加設定は不要",
						},
					},
					exposure: {
						title: "3. 直接公開",
						descriptions: {
							broker_only:
								"ブローカーのみモードでは、直接公開対象としてマークされたサーバーを含む、有効な MCP サーバーはすべて内蔵 UCAN ツール経由で利用できます。",
							server_live:
								"ツール、プロンプト、リソース、リソーステンプレートをこのクライアントへ直接公開する対象サーバーを選択します。",
							capability_level:
								"対象サーバーを選び、ツール、プロンプト、リソース、テンプレートを能力単位で直接公開する設定を行います。（上級）",
						},
						labels: {
							ucanRoutingDescription:
								"ブローカーのみモードでは、直接公開としてマークされたサーバーを含め、有効な MCP サーバーはすべて UCAN カタログとツール呼び出し経由でアクセスされます。",
						},
						empty: {
							no_eligible:
								"対象となるサーバーがありません。先にサーバー詳細で Unify の直接公開を有効にしてください。",
						},
					},
				},
			labels: {
				noDescription: "説明なし",
				openProfileDetail: "プロファイルの詳細を開く",
				servers: "サーバー",
				tools: "ツール",
				resources: "リソース",
				prompts: "プロンプト",
				resourceTemplates: "リソーステンプレート",
				toolsExposed: "個のツールを直接公開",
				capabilitiesExposed: "個のケイパビリティを直接公開",
			},
			nonWritableReason: "このレコードは現在書き込みできません。",
			transportOptions: {
				auto: "自動",
				stdio: "STDIO",
				streamableHttp: "ストリーミング HTTP",
				streamableHttpLegacy: "ストリーミング HTTP",
				sseLegacy: "SSE（レガシー互換）",
			},
			form: {
				fields: {
					configPath: {
						placeholder: "~/.cursor/mcp.json",
					},
					logoUrl: {
						placeholder: "https://example.com/logo.png",
					},
					homepageUrl: {
						placeholder: "https://example.com",
					},
					docsUrl: {
						placeholder: "https://docs.example.com",
					},
					supportUrl: {
						placeholder: "https://support.example.com",
					},
				},
			},
		},
			backups: {
				title: "バックアップ",
				description: "設定スナップショットの復元・削除を行います。",
				buttons: {
					refresh: "更新",
					selectAll: "すべて選択",
					clear: "クリア",
					deleteSelected: "選択した項目を削除（{{count}}）",
					restore: "復元",
					delete: "削除",
				},
				empty: "バックアップはありません。",
				emptyDisabledByPolicy: "システムポリシーにより現在バックアップは無効です。",
				bulk: {
					title: "選択したバックアップを削除",
					description:
						"{{count}} 件のバックアップを削除しますか？この操作は元に戻せません。",
				},
			},
			logs: {
				title: "ログ",
				description:
					"このクライアントに関連する実行ログと監査イベントを表示します。",
				searchPlaceholder: "ログを検索...",
				refresh: "ログを更新",
				expand: "ログを展開",
				collapse: "ログを折りたたむ",
				loading: "ログを読み込み中...",
				headers: {
					timestamp: "時刻",
					action: "アクション",
					category: "カテゴリ",
					status: "ステータス",
					target: "対象",
				},
				empty: "まだログが記録されていません。",
			},
			confirm: {
				deleteTitle: "バックアップを削除",
				restoreTitle: "バックアップを復元",
				deleteDescription:
					"このバックアップを削除しますか？この操作は元に戻せません。",
				restoreDescription:
					"選択したバックアップからローカルのクライアント設定ファイルを復元しますか？MCPMate の管理モードと能力設定は変更されません。",
				deleteLabel: "削除",
				restoreLabel: "復元",
				cancelLabel: "キャンセル",
			},
			policy: {
				title: "バックアップポリシー",
				fields: {
					policy: "ポリシー",
					policyDescription:
						'バックアップ保持方針です。現在は "keep_n" のみ対応し、最新 N 件を保持して古いものを削除します。',
					options: {
						keepN: "keep_n",
					},
					limit: "上限",
					limitDescription:
						"このクライアントで保持するバックアップ数の上限です。0 にすると無制限です。",
				},
				buttons: {
					save: "ポリシーを保存",
				},
			},
			importPreview: {
				title: "インポートプレビュー",
				description: "現在の設定から検出したサーバーの概要です。",
				fields: {
					attempted: "試行",
					imported: "インポート済み",
					skipped: "スキップ",
					failed: "失敗",
				},
				noPreview: "プレビューはありません。",
				sections: {
					servers: "インポート対象サーバー",
					errors: "エラー",
					raw: "プレビュー JSON",
					stats:
						"ツール: {{tools}} • リソース: {{resources}} • テンプレート: {{templates}} • プロンプト: {{prompts}}",
				},
				buttons: {
					close: "閉じる",
					apply: "インポートを適用",
					preview: "プレビュー",
				},
				states: {
					noImportNeeded: "インポート不要",
				},
			},
				notifications: {
					reviewSuccess: {
						title: "成功",
						messageApproved: "レコードを承認しました。",
						messageRejected: "レコードを拒否しました。",
					},
					reviewFailed: {
						title: "レビューに失敗しました",
					},
					previewReady: {
						title: "プレビュー準備完了",
						message: "適用前に差分を確認してください。",
					noChanges: "この設定では変更はありませんでした。",
				},
				applied: {
					title: "適用しました",
					message: "設定を適用しました",
				},
				managementSaved: {
					title: "保存しました",
					message: "管理設定を MCPMate に保存しました。ローカルのクライアント設定は更新していません。",
				},
				applyFailed: {
					title: "適用に失敗しました",
				},
				imported: {
					title: "インポート完了",
					message: "{{count}} 件のサーバーをインポートしました",
				},
				nothingToImport: {
					title: "インポート不要",
					message:
						"すべての項目がスキップされたか、インポート可能なサーバーがありません。",
				},
				importFailed: {
					title: "インポートに失敗しました",
				},
				refreshed: {
					title: "更新しました",
					message: "検出状態を更新しました",
				},
				refreshFailed: {
					title: "更新に失敗しました",
				},
				managedUpdated: {
					title: "更新しました",
					message: "管理状態が変更されました",
				},
				managedFailed: {
					title: "更新に失敗しました",
				},
					governanceUpdated: {
						title: "更新しました",
						message: "クライアントのガバナンス状態を変更しました",
					},
				governanceFailed: {
					title: "更新に失敗しました",
				},
				governanceAllowed: {
					title: "更新しました",
					message: "このクライアントは許可状態になりました。",
				},
				governanceDenied: {
					title: "更新しました",
					message: "このクライアントは拒否状態になりました。",
				},
				previewFailed: {
					title: "プレビューに失敗しました",
				},
				restored: {
					title: "復元しました",
					message: "バックアップから設定を復元しました",
				},
				restoreFailed: {
					title: "復元に失敗しました",
				},
				deleted: {
					title: "削除しました",
					message: "バックアップを削除しました",
				},
				deleteFailed: {
					title: "削除に失敗しました",
				},
				bulkDeleted: {
					title: "削除しました",
					message: "選択したバックアップを削除しました",
				},
				bulkDeleteFailed: {
					title: "一括削除に失敗しました",
				},
				saved: {
					title: "保存しました",
					message: "バックアップポリシーを更新しました",
				},
				saveFailed: {
					title: "保存に失敗しました",
				},
			},
		},
	},
};
