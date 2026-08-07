export const commonTranslations = {
	en: {
		wip: "WIP",
		wipTag: "(WIP)",
		yes: "Yes",
		no: "No",
		loading: "Loading…",
		reload: "Reload",
		user: "User",
		roles: {
			user: "User",
			admin: "Admin",
			defaultAnchor: "Default Anchor",
		},
		status: {
			ready: "Ready",
			error: "Error",
			disconnected: "Disconnected",
			initializing: "Initializing",
			idle: "Idle",
			unknown: "Unknown",
			status: "Status",
			enabled: "Enabled",
			disabled: "Disabled",
		},
		placeholders: {
			menuBarVisibility: "Menu bar visibility",
			dockIconVisibility: "Dock icon visibility",
			selectLanguage: "Select language",
			selectMarket: "Select market",
			searchHiddenServers: "Search hidden servers...",
		},
		sort: {
			recent: "Most Recently Hidden",
			name: "Name (A-Z)",
		},
		serverImport: {
			nameList: {
				more: "+{{count}} more",
			},
			queryLabels: {
				existing: "Existing query",
				incoming: "Incoming query",
			},
			skippedReasons: {
				duplicateName: "Name already in use",
				duplicateFingerprint: "Already installed",
				urlQueryMismatch: "URL query mismatch",
				configUnrecognized: "Unrecognized config entry",
				configInvalidEntry: "Invalid config entry",
				configMissingCommand: "Missing command",
				configMissingUrl: "Missing URL",
				unknown: "Unknown reason",
			},
		},
		profileSyncErrors: {
			importFailedTitle: "Import failed",
			pendingPublishTitle: "Server publish failed",
			importedServerMissing:
				"An imported server could not be found. Refresh the server list and try again.",
			importedServerAmbiguous:
				"More than one server matches an imported name. Rename the duplicate servers and try again.",
			profileAuthoringStateMissing:
				"The latest Profile state is not loaded. Refresh the Profile and try again.",
			profileAuthoringStateMismatch:
				"The selected capabilities were loaded from different Profile versions. Refresh and try again.",
			catalogSnapshotMissing:
				"Capability catalog state is not loaded. Refresh and try again.",
			catalogSnapshotMismatch:
				"The selected capabilities were loaded from different catalog versions. Refresh and try again.",
			capabilitySnapshotMissing:
				"The server capability catalog is not ready. Refresh the server and try again.",
			profileAuthoringChanged:
				"This Profile changed elsewhere. Your draft is preserved; review the latest state and save again.",
			catalogDependencyChanged:
				"A related server capability catalog changed. The affected data was refreshed; review and try again.",
			consumerBindingChanged:
				"A related client binding changed. Refresh the Profile and try again.",
			invalidTarget:
				"The selected Profile target is no longer available. Refresh and choose another target.",
			unexpected:
				"The Profile update could not be completed. Refresh and try again.",
		},
		backendReadiness: {
			title: "MCPMate is starting",
			starting: "Starting MCPMate Core...",
			waitingForBackend: "Waiting for MCPMate backend",
			confirmingReadiness: "Confirming backend readiness",
			issueTitle: "Startup needs attention",
			exportDiagnostics: "Export diagnostics",
			exportingDiagnostics: "Exporting diagnostics...",
			exportSuccess: "Diagnostics exported to {{path}}",
			exportFailed: "Unable to export diagnostics: {{error}}",
			issue: {
				backendStarting: "Backend process is starting{{target}}",
				coreService: "{{label}}{{detail}}",
				networkError: "Backend API is not reachable{{target}}: {{error}}",
				notReady: "Backend readiness is {{statusKey}}{{reason}}",
				unknown: "{{detail}}",
			},
		},
		pagination: {
			first: "First",
			previous: "Previous",
			next: "Next",
			last: "Last",
			perPage: "Per page",
			page: "Page {{page}}",
			showing: "Showing {{start}}-{{end}} items",
			showingOfTotal: "Showing {{start}}-{{end}} of {{total}} items",
			showingEmpty: "No items on this page",
			totalItems: "Total {{total}} items",
			summary: "{{start}}–{{end}} · {{page}}",
			summaryTitle: "Items {{start}}–{{end}}, page {{page}}",
			pageWord: "Page",
			pageSuffix: "",
			ofTotal: "of {{total}}",
			goToPage: "Go to page",
		},
		bulkSelection: {
			bulkModeEnter: "Bulk select",
			bulkModeExit: "Exit bulk select",
			bulkModeDescription: "{{count}} selected for bulk actions",
		},
		lock: {
			title: "MCPMate",
			unlock: "Unlock",
			verifying: "Verifying...",
			login: {
				description: "Enter your login password to continue.",
				passwordPlaceholder: "Login password",
				wrongPassword: "Incorrect login password. Please try again.",
				verifyError: "Could not verify login password. Please try again.",
			},
			encryption: {
				description: "Enter your encryption password to unlock the secure store.",
				passwordPlaceholder: "Encryption password",
				unlockError:
					"Could not unlock the secure store. Check your encryption password and try again.",
			},
		},
	},
	"zh-CN": {
		wip: "开发中",
		wipTag: "(开发中)",
		yes: "是",
		no: "否",
		loading: "加载中…",
		reload: "重新加载",
		user: "用户",
		roles: {
			user: "用户",
			admin: "管理员",
			defaultAnchor: "默认锚点",
		},
		status: {
			ready: "就绪",
			error: "错误",
			disconnected: "已断开",
			initializing: "初始化中",
			idle: "空闲",
			unknown: "未知",
			status: "状态",
			enabled: "已启用",
			disabled: "已禁用",
		},
		placeholders: {
			menuBarVisibility: "菜单栏可见性",
			dockIconVisibility: "Dock 图标可见性",
			selectLanguage: "选择语言",
			selectMarket: "选择市场",
			searchHiddenServers: "搜索隐藏服务器...",
		},
		sort: {
			recent: "最近隐藏时间",
			name: "名称 (A-Z)",
		},
		serverImport: {
			nameList: {
				more: "另有 {{count}} 项",
			},
			queryLabels: {
				existing: "现有查询参数",
				incoming: "传入查询参数",
			},
			skippedReasons: {
				duplicateName: "名称已被占用",
				duplicateFingerprint: "已安装",
				urlQueryMismatch: "URL 查询参数不匹配",
				configUnrecognized: "未识别的配置项",
				configInvalidEntry: "无效的配置项",
				configMissingCommand: "缺少命令",
				configMissingUrl: "缺少 URL",
				unknown: "未知原因",
			},
		},
		profileSyncErrors: {
			importFailedTitle: "导入失败",
			pendingPublishTitle: "服务器发布失败",
			importedServerMissing: "找不到已导入的服务器。请刷新服务器列表后重试。",
			importedServerAmbiguous:
				"多个服务器使用了相同的导入名称。请重命名重复服务器后重试。",
			profileAuthoringStateMissing:
				"尚未加载最新的 Profile 状态。请刷新 Profile 后重试。",
			profileAuthoringStateMismatch:
				"所选能力来自不同的 Profile 版本。请刷新后重试。",
			catalogSnapshotMissing: "尚未加载能力目录状态。请刷新后重试。",
			catalogSnapshotMismatch:
				"所选能力来自不同的目录版本。请刷新后重试。",
			capabilitySnapshotMissing:
				"服务器能力目录尚未就绪。请刷新服务器后重试。",
			profileAuthoringChanged:
				"此 Profile 已在其他位置更改。你的草稿已保留；请检查最新状态后再次保存。",
			catalogDependencyChanged:
				"相关服务器的能力目录已更改。受影响的数据已刷新；请检查后重试。",
			consumerBindingChanged:
				"相关客户端绑定已更改。请刷新 Profile 后重试。",
			invalidTarget:
				"所选 Profile 目标已不可用。请刷新并选择其他目标。",
			unexpected: "无法完成 Profile 更新。请刷新后重试。",
		},
		backendReadiness: {
			title: "MCPMate 正在启动",
			starting: "正在启动 MCPMate Core...",
			waitingForBackend: "正在等待 MCPMate 后端",
			confirmingReadiness: "正在确认后端就绪状态...",
			issueTitle: "启动需要处理",
			exportDiagnostics: "导出诊断日志",
			exportingDiagnostics: "正在导出诊断日志...",
			exportSuccess: "诊断日志已导出到 {{path}}",
			exportFailed: "无法导出诊断日志：{{error}}",
			issue: {
				backendStarting: "后端进程正在启动{{target}}",
				coreService: "{{label}}{{detail}}",
				networkError: "无法访问后端 API{{target}}：{{error}}",
				notReady: "后端就绪状态为 {{statusKey}}{{reason}}",
				unknown: "{{detail}}",
			},
		},
		pagination: {
			first: "首页",
			previous: "上一页",
			next: "下一页",
			last: "末页",
			perPage: "每页",
			page: "第 {{page}} 页",
			showing: "显示第 {{start}}-{{end}} 项",
			showingOfTotal: "显示第 {{start}}-{{end}} 项，共 {{total}} 条",
			showingEmpty: "本页无记录",
			totalItems: "共 {{total}} 条",
			summary: "{{start}}–{{end}} · {{page}}",
			summaryTitle: "第 {{start}}–{{end}} 条，第 {{page}} 页",
			pageWord: "第",
			pageSuffix: "页",
			ofTotal: "，共 {{total}} 页",
			goToPage: "跳转到页码",
		},
		bulkSelection: {
			bulkModeEnter: "批量选择",
			bulkModeExit: "退出批量选择",
			bulkModeDescription: "已选择 {{count}} 项，可进行批量操作",
		},
		lock: {
			title: "MCPMate",
			unlock: "解锁",
			verifying: "验证中...",
			login: {
				description: "请输入登录密码以继续。",
				passwordPlaceholder: "登录密码",
				wrongPassword: "登录密码不正确，请重试。",
				verifyError: "无法验证登录密码，请重试。",
			},
			encryption: {
				description: "请输入加密密码以解锁安全存储。",
				passwordPlaceholder: "加密密码",
				unlockError: "无法解锁安全存储，请检查加密密码后重试。",
			},
		},
	},
	"ja-JP": {
		wip: "開発中",
		wipTag: "(開発中)",
		yes: "はい",
		no: "いいえ",
		loading: "読み込み中…",
		reload: "再読み込み",
		user: "ユーザー",
		roles: {
			user: "ユーザー",
			admin: "管理者",
			defaultAnchor: "デフォルトアンカー",
		},
		status: {
			ready: "準備完了",
			error: "エラー",
			disconnected: "切断済み",
			initializing: "初期化中",
			idle: "アイドル",
			unknown: "不明",
			status: "状態",
			enabled: "有効",
			disabled: "無効",
		},
		placeholders: {
			menuBarVisibility: "メニューバー表示",
			dockIconVisibility: "Dock アイコン表示",
			selectLanguage: "言語を選択",
			selectMarket: "マーケットを選択",
			searchHiddenServers: "非表示サーバーを検索...",
		},
		sort: {
			recent: "最近非表示",
			name: "名前 (A-Z)",
		},
		serverImport: {
			nameList: {
				more: "他 {{count}} 件",
			},
			queryLabels: {
				existing: "既存のクエリ",
				incoming: "入力クエリ",
			},
			skippedReasons: {
				duplicateName: "名前が既に使用中",
				duplicateFingerprint: "インストール済み",
				urlQueryMismatch: "URL クエリ不一致",
				configUnrecognized: "未識別の設定項目",
				configInvalidEntry: "無効な設定項目",
				configMissingCommand: "コマンドがありません",
				configMissingUrl: "URL がありません",
				unknown: "不明な理由",
			},
		},
		profileSyncErrors: {
			importFailedTitle: "インポートに失敗しました",
			pendingPublishTitle: "サーバーの公開に失敗しました",
			importedServerMissing:
				"インポートしたサーバーが見つかりません。サーバー一覧を更新して再試行してください。",
			importedServerAmbiguous:
				"同じインポート名に複数のサーバーが一致します。重複するサーバー名を変更して再試行してください。",
			profileAuthoringStateMissing:
				"最新の Profile 状態が読み込まれていません。Profile を更新して再試行してください。",
			profileAuthoringStateMismatch:
				"選択した capability は異なる Profile バージョンから読み込まれています。更新して再試行してください。",
			catalogSnapshotMissing:
				"capability catalog の状態が読み込まれていません。更新して再試行してください。",
			catalogSnapshotMismatch:
				"選択した capability は異なる catalog バージョンから読み込まれています。更新して再試行してください。",
			capabilitySnapshotMissing:
				"サーバーの capability catalog はまだ準備できていません。サーバーを更新して再試行してください。",
			profileAuthoringChanged:
				"この Profile は別の場所で変更されました。下書きは保持されています。最新の状態を確認して再度保存してください。",
			catalogDependencyChanged:
				"関連サーバーの capability catalog が変更されました。対象データを更新しました。確認して再試行してください。",
			consumerBindingChanged:
				"関連する client binding が変更されました。Profile を更新して再試行してください。",
			invalidTarget:
				"選択した Profile は利用できなくなりました。更新して別の対象を選択してください。",
			unexpected:
				"Profile の更新を完了できませんでした。更新して再試行してください。",
		},
		backendReadiness: {
			title: "MCPMate を起動しています",
			starting: "MCPMate Core を起動しています...",
			waitingForBackend: "MCPMate backend を待機中",
			confirmingReadiness: "backend readiness を確認中...",
			issueTitle: "起動に確認が必要です",
			exportDiagnostics: "診断ログをエクスポート",
			exportingDiagnostics: "診断ログをエクスポート中...",
			exportSuccess: "診断ログを {{path}} にエクスポートしました",
			exportFailed: "診断ログをエクスポートできません: {{error}}",
			issue: {
				backendStarting: "backend process を起動中です{{target}}",
				coreService: "{{label}}{{detail}}",
				networkError: "backend API に到達できません{{target}}: {{error}}",
				notReady: "backend readiness は {{statusKey}}{{reason}} です",
				unknown: "{{detail}}",
			},
		},
		pagination: {
			first: "最初",
			previous: "前へ",
			next: "次へ",
			last: "最後",
			perPage: "件数",
			page: "ページ {{page}}",
			showing: "{{start}}-{{end}} 件を表示",
			showingOfTotal: "全 {{total}} 件中 {{start}}-{{end}} 件を表示",
			showingEmpty: "このページに項目はありません",
			totalItems: "合計 {{total}} 件",
			summary: "{{start}}–{{end}} · {{page}}",
			summaryTitle: "{{start}}–{{end}} 件目、ページ {{page}}",
			pageWord: "",
			pageSuffix: "",
			ofTotal: "/ {{total}} ページ",
			goToPage: "ページへ移動",
		},
		bulkSelection: {
			bulkModeEnter: "一括選択",
			bulkModeExit: "一括選択を終了",
			bulkModeDescription: "一括操作の対象 {{count}} 件",
		},
		lock: {
			title: "MCPMate",
			unlock: "ロック解除",
			verifying: "確認中...",
			login: {
				description: "続行するにはログインパスワードを入力してください。",
				passwordPlaceholder: "ログインパスワード",
				wrongPassword:
					"ログインパスワードが正しくありません。もう一度お試しください。",
				verifyError:
					"ログインパスワードを確認できませんでした。もう一度お試しください。",
			},
			encryption: {
				description:
					"セキュアストアのロックを解除するには暗号化パスワードを入力してください。",
				passwordPlaceholder: "暗号化パスワード",
				unlockError:
					"セキュアストアのロック解除に失敗しました。暗号化パスワードを確認して再試行してください。",
			},
		},
	},
} as const;
