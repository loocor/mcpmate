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
					detected: "Detected",
					managed: "Managed",
				},
			},
			sort: {
				options: {
					displayName: "Name",
					detected: "Detection Status",
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
			overview: {
				labels: {
					configPath: "Config Path",
					lastModified: "Last Modified",
					homepage: "Homepage",
					docs: "Docs",
					support: "Support",
				},
				buttons: {
					refresh: "Refresh",
					enable: "Enable",
					disable: "Disable",
				},
				noDetails: "No details available",
				currentServers: {
					title: "Current Servers",
					import: "Import from Config",
					configuredLabel: "configured",
					empty: "No servers extracted from current config.",
				},
			},
			configuration: {
				title: "Configuration Mode",
				description:
					"If you don't understand what this means, please don't make any changes and keep the current settings.",
				reapply: "Re-apply",
				sections: {
					mode: {
						title: "1. Management Mode",
						descriptions: {
							smart:
								"Smart starts with builtin MCP tools only and keeps its working set inside the current session.",
							hosted:
								"Hosted keeps a durable managed configuration for this client and remembers the selected working state.",
							transparent:
								"MCPMate writes the selected profile servers directly into this client's MCP configuration and does not preserve capability-level controls.",
						},
						options: {
							smart: "Smart",
							hosted: "Hosted",
							transparent: "Transparent",
						},
					},
					source: {
						title: "2. Configuration",
						titleTransparent: "2. Configuration",
						descriptions: {
							smart:
								"Builtin MCP tools will select profiles on demand from the configured library during the current session.",
							default: "Review the profiles that are currently active for this client runtime.",
							profile: "Browse the shared scene library and choose the exact working set for this client.",
							custom: "Create client-specific adjustments on top of the current smart-mode working state.",
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
							custom: "Customize",
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
							smart:
								"Profiles shown here are the configured sources Smart Mode can draw from during the current session.",
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
					smart: {
						title: "2. Configuration",
						description:
							"Smart starts with builtin MCP tools only. It keeps the working set inside the current MCP session and resets when the session ends.",
						items: {
							builtinOnly: "Builtin tools only",
							sessionScoped: "Session-scoped workspace",
							noFurtherSetup: "No further setup in the dashboard",
						},
					},
				},
				labels: {
					noDescription: "No description",
					servers: "Servers",
					tools: "Tools",
					resources: "Resources",
					prompts: "Prompts",
				},
				transportOptions: {
					stdio: "STDIO",
					streamableHttp: "Streamable HTTP",
					sseLegacy: "SSE (Legacy)",
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
					"Restore configuration from the selected backup? Current config may be overwritten.",
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
				previewReady: {
					title: "Preview ready",
					message: "Review the diff before applying.",
					noChanges: "No changes detected in this configuration.",
				},
				applied: {
					title: "Applied",
					message: "Configuration applied",
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
					detected: "已检测",
					managed: "已管理",
				},
			},
			sort: {
				options: {
					displayName: "名称",
					detected: "检测状态",
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
			overview: {
				labels: {
					configPath: "配置路径",
					lastModified: "最近修改",
					homepage: "主页",
					docs: "文档",
					support: "支持",
				},
				buttons: {
					refresh: "刷新",
					enable: "启用",
					disable: "停用",
				},
				noDetails: "暂无详细信息",
				currentServers: {
					title: "当前服务器",
					import: "从配置导入",
					configuredLabel: "已配置",
					empty: "未从当前配置解析到服务器。",
				},
			},
			configuration: {
				title: "配置模式",
				description: "若不清楚含义，请勿修改并保持现有设置。",
				reapply: "重新应用",
				sections: {
					mode: {
						title: "1. 管理模式",
						descriptions: {
							smart:
								"Smart 模式初始仅提供内建 MCP 工具，并将工作集保留在当前会话中。",
							hosted:
								"Hosted 会为该客户端保留持久化托管配置，并记住当前选择的工作状态。",
							transparent:
								"MCPMate 会将所选配置集中的服务器直接写入该客户端的 MCP 配置，且不会保留 capabilities 层级的控制。",
						},
						options: {
							smart: "Smart",
							hosted: "Hosted",
							transparent: "透明模式",
						},
					},
					source: {
						title: "2. 配置详情",
						titleTransparent: "2. 配置详情",
						descriptions: {
							smart: "当前会话中，内建 MCP 工具会从已配置的场景库中按需挑选 Profiles。",
							default: "查看当前已对该客户端运行态生效的配置集。",
							profile: "浏览共享场景库，并为该客户端选择精确的工作集。",
							custom: "在当前 Smart 模式工作状态之上创建客户端专属调整。",
						transparentDefault:
							"将当前所有已激活配置集中的服务器直接写入该客户端的 MCP 配置。",
						transparentProfile:
							"将所选共享配置集中的服务器直接写入该客户端的 MCP 配置。",
						transparentCustom:
							"将该客户端专属自定义配置集中的服务器直接写入该客户端的 MCP 配置。",
						},
						options: {
							default: "Active",
							profile: "Profiles",
							custom: "Customize",
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
							smart: "此处展示的是 Smart 模式在当前会话中可按需使用的已配置 Profiles。",
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
							titleDefault: "打开场景库",
							subtitleCustom: "为当前工作集创建并管理客户端专属覆盖项",
							subtitleCustomTransparent: "配置哪些服务器会被直接写入当前客户端。",
							subtitleDefault: "浏览可复用共享场景，并在配置集页面中维护它们",
						},
					},
					smart: {
						title: "2. 配置详情",
						description:
							"Smart 模式初始仅提供内建 MCP 工具。它会将工作集保留在当前 MCP 会话中，并在会话结束后自动重置。",
						items: {
							builtinOnly: "仅内建工具",
							sessionScoped: "会话级工作集",
							noFurtherSetup: "仪表板中无需进一步设置",
						},
					},
				},
				labels: {
					noDescription: "暂无描述",
					servers: "服务器",
					tools: "工具",
					resources: "资源",
					prompts: "提示",
				},
				transportOptions: {
					stdio: "STDIO",
					streamableHttp: "Streamable HTTP",
					sseLegacy: "SSE（旧版兼容）",
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
				restoreDescription: "要从选定备份恢复配置吗？当前配置可能被覆盖。",
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
				previewReady: {
					title: "预览已就绪",
					message: "请在应用前先查看差异。",
					noChanges: "当前配置未产生任何变化。",
				},
				applied: {
					title: "已应用",
					message: "配置已应用",
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
					detected: "検出済み",
					managed: "管理中",
				},
			},
			sort: {
				options: {
					displayName: "名前",
					detected: "検出状況",
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
			overview: {
				labels: {
					configPath: "設定パス",
					lastModified: "最終更新",
					homepage: "ホームページ",
					docs: "ドキュメント",
					support: "サポート",
				},
				buttons: {
					refresh: "更新",
					enable: "有効化",
					disable: "無効化",
				},
				noDetails: "詳細情報がありません",
				currentServers: {
					title: "現在のサーバー",
					import: "設定からインポート",
					configuredLabel: "設定済み",
					empty: "現在の設定からサーバーを取得できませんでした。",
				},
			},
			configuration: {
				title: "設定モード",
				description: "意味が不明な場合は変更せず現状の設定を維持してください。",
				reapply: "再適用",
				sections: {
					mode: {
						title: "1. 管理モード",
						descriptions: {
							smart:
								"Smart は内蔵 MCP ツールのみで開始し、ワークセットを現在のセッション内に保持します。",
							hosted:
								"Hosted はこのクライアントの持続的な管理設定を保持し、現在のワーク状態を記憶します。",
							transparent:
								"MCPMate は選択したプロファイルのサーバーをこのクライアントの MCP 設定へ直接書き込み、capability 単位の制御は保持しません。",
						},
						options: {
							smart: "Smart",
							hosted: "Hosted",
							transparent: "トランスペアレントモード",
						},
					},
					source: {
						title: "2. 設定",
						titleTransparent: "2. 設定",
						descriptions: {
							smart:
								"現在のセッションでは、内蔵 MCP ツールが設定済みプロファイルライブラリから必要な Profiles を動的に選択します。",
							default: "このクライアントの実行時に現在有効なプロファイルを確認します。",
							profile: "共有シーンライブラリを参照し、このクライアントの正確なワークセットを選択します。",
							custom: "現在の Smart Mode ワーク状態の上にクライアント専用の調整を作成します。",
						transparentDefault:
							"現在有効なすべてのプロファイルのサーバーをこのクライアントの MCP 設定へ直接書き込みます。",
						transparentProfile:
							"選択した共有プロファイルのサーバーをこのクライアントの MCP 設定へ直接書き込みます。",
						transparentCustom:
							"このクライアント専用のカスタムプロファイルのサーバーをこのクライアントの MCP 設定へ直接書き込みます。",
						},
						options: {
							default: "Active",
							profile: "Profiles",
							custom: "Customize",
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
							smart:
								"ここには、Smart モードが現在のセッションで必要に応じて利用できる設定済み Profiles が表示されます。",
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
					smart: {
						title: "2. 設定",
						description:
							"Smart は内蔵 MCP ツールのみで開始します。ワークセットは現在の MCP セッションに保持され、セッション終了時に自動的にリセットされます。",
						items: {
							builtinOnly: "内蔵ツールのみ",
							sessionScoped: "セッション単位のワークスペース",
							noFurtherSetup: "ダッシュボードで追加設定は不要",
						},
					},
				},
				labels: {
					noDescription: "説明なし",
					servers: "サーバー",
					tools: "ツール",
					resources: "リソース",
					prompts: "プロンプト",
				},
				transportOptions: {
					stdio: "STDIO",
					streamableHttp: "ストリーミング HTTP",
					sseLegacy: "SSE（レガシー互換）",
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
					"選択したバックアップから設定を復元しますか？現在の設定は上書きされる可能性があります。",
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
				previewReady: {
					title: "プレビュー準備完了",
					message: "適用前に差分を確認してください。",
					noChanges: "この設定では変更はありませんでした。",
				},
				applied: {
					title: "適用しました",
					message: "設定を適用しました",
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
