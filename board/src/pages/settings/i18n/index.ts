export const settingsTranslations = {
en: {
    title: "Tune dashboard preferences and defaults",
		tabs: {
			general: "General",
			appearance: "Appearance",
			system: "System",
			serverControls: "Server Controls",
			clientDefaults: "Client Defaults",
			market: "MCP Market",
			developer: "Developer",
			about: "About & Licenses",
		},
        system: {
            title: "System",
            description: "Configure backend ports used by the local MCP proxy and API.",
            apiPortTitle: "Backend API Port",
            apiPortDescription: "Port for REST and dashboard access (default 8080).",
            mcpPortTitle: "Backend MCP Server Port",
            mcpPortDescription: "Port for MCP transports (/mcp for HTTP, /sse for SSE). Default 8000.",
            apply: "Apply & Restart",
            helperTauri: "Tauri: Apply ports and restart backend in-place.",
            helperWeb: "Web: Change ports then restart the backend process externally.",
            webDialogTitle: "Apply & Restart (Web)",
            webDialogDesc: "The browser cannot restart the backend. Use one of the commands below with the selected ports.",
            optionCargoTitle: "Option A — cargo run (dev)",
            optionBinaryTitle: "Option B — binary (release)",
            copy: "Copy",
            stopCurrent: "Stop current backend",
            close: "Close",
        },
		general: {
			title: "General",
			description: "Baseline preferences for the main workspace views.",
			defaultView: "Default View",
			defaultViewDescription: "Choose the default layout for displaying items.",
			appMode: "Application Mode",
			appModeDescription: "Select the interface complexity level.",
			language: "Language",
			languageDescription: "Select the dashboard language.",
			languagePlaceholder: "Select language",
		},
		appearance: {
			title: "Appearance",
			description: "Customize the look and feel of the dashboard.",
			themeTitle: "Theme",
			themeDescription: "Switch between light and dark mode.",
			systemPreferenceTitle: "System Preference",
			systemPreferenceDescription:
				"Follow the operating system preference automatically.",
			menuBarTitle: "Menu Bar Icon",
			menuBarDescription: "Control the visibility of the menu bar icon.",
			menuBarIconTitle: "Menu Bar Icon Mode",
			dockTitle: "Dock Icon",
			dockDescription:
				"Display MCPMate in the macOS Dock or run silently from the menu bar.",
			dockIconTitle: "Dock Icon Mode",
			dockHiddenNotice:
				"The Dock icon is hidden. The menu bar icon will remain visible so you can reopen MCPMate.",
			menuBarPlaceholder: "Select menu bar icon mode",
			wipLabel: "Work in Progress",
			defaultMarketPlaceholder: "Select default market",
		},
    options: {
			theme: {
				light: "Light",
				dark: "Dark",
			},
			defaultView: {
				list: "List",
				grid: "Grid",
			},
			appMode: {
				express: "Express",
				expert: "Expert",
			},
			clientMode: {
				hosted: "Hosted",
				transparent: "Transparent",
			},
			backup: {
				keepN: "Keep N",
				keepLast: "Keep Last",
				none: "None",
			},
			menuBar: {
				runtime: "Visible When Running",
				hidden: "Hidden",
			},
		},
		servers: {
			title: "Server Controls",
			description: "Decide how server operations propagate across clients.",
			syncTitle: "Sync Global Start/Stop",
			syncDescription: "Push global enable state to managed clients instantly.",
			autoAddTitle: "Auto Add To Default Profile",
			autoAddDescription:
				"Include new servers in the default profile automatically.",
		},
		clients: {
			title: "Client Defaults",
			description:
				"Configure default rollout and backup behavior for client apps.",
			defaultVisibilityTitle: "Default Client Visibility",
			defaultVisibilityDescription:
				"Choose which client statuses are shown by default on the Clients page.",
			defaultVisibility: {
				all: "All",
				detected: "Detected",
				managed: "Managed",
			},
            modeTitle: "Client Management Mode",
            modeDescription:
                "Choose how client configurations should be managed by default.",
			backupStrategyTitle: "Client Backup Strategy",
			backupStrategyDescription:
				"Define how client configurations should be backed up.",
			backupLimitTitle: "Maximum Backup Copies",
			backupLimitDescription:
				"Set the maximum number of backup copies to keep. Applied when the strategy is set to Keep N. Values below 1 are rounded up.",
		},
		developer: {
			title: "Developer",
			description:
				"Experimental toggles for internal inspection and navigation visibility.",
			enableServerDebugTitle: "Enable Server Inspection",
			enableServerDebugDescription:
				"Expose inspection instrumentation for newly added servers.",
			openDebugInNewWindowTitle: "Open Inspect Views In New Window",
			openDebugInNewWindowDescription:
				"When enabled, Inspect buttons launch a separate tab instead of navigating the current view.",
			showApiDocsTitle: "Show API Docs Menu",
			showApiDocsDescription:
				"Display API documentation menu in the navigation.",
			showDefaultHeadersTitle: "Show Default HTTP Headers",
			showDefaultHeadersDescription:
				"Display the server's default HTTP headers (values are redacted) in Server Details. Use only for inspection.",
			showRawJsonTitle: "Show Raw Capability JSON",
			showRawJsonDescription:
				"Display raw JSON payloads under Details in capability lists (Server details and Uni‑Import preview).",
		},
		market: {
			title: "MCP Market",
			description:
				"Configure default market and manage hidden marketplace servers.",
			defaultMarketTitle: "Default Market",
			defaultMarketDescription:
				"Choose which market appears first and cannot be closed.",
			officialPortal: "Official MCP Registry",
			enableBlacklistTitle: "Enable Blacklist",
			enableBlacklistDescription:
				"Hide quality-poor or unavailable content from the market to keep it clean",
			searchHiddenServers: "Search hidden servers",
			sortHiddenServers: "Sort hidden servers",
			sortPlaceholder: "Sort",
			emptyTitle: "No hidden servers currently.",
			emptyDescription:
				"Hide servers from the Market list to keep this space tidy. They will appear here for recovery.",
			noNotes: "No notes added.",
			hiddenOn: "Hidden on {{value}}",
			restore: "Restore",
		},
		about: {
			title: "About MCPMate",
			description:
				"Open-source acknowledgements for the MCPMate preview build.",
			lastUpdated: "Last updated: {{date}}",
			backendTitle: "Backend (Rust workspace)",
			desktopShellTitle: "Desktop Shell (Tauri)",
			dashboardTitle: "Dashboard (Web)",
			components: "{{count}} components",
			repository: "Repository",
			homepage: "Homepage",
			noPackages: "No third-party packages detected during the latest update.",
		},
    notices: {
			dockHidden:
				"The Dock icon is hidden. The menu bar icon will remain visible so you can reopen MCPMate.",
		},
	},
"zh-CN": {
    title: "调整面板偏好与默认行为",
		tabs: {
			general: "通用",
			appearance: "外观",
			system: "系统",
			serverControls: "服务器",
			clientDefaults: "客户端",
			market: "服务源",
			developer: "开发者",
			about: "关于",
		},
        system: {
            title: "系统",
            description: "配置本地 MCP 代理与 API 使用的后端端口。",
            apiPortTitle: "后端 API 端口",
            apiPortDescription: "用于 REST 与控制台访问的端口（默认 8080）。",
            mcpPortTitle: "后端 MCP 服务端口",
            mcpPortDescription: "用于 MCP 传输（/mcp HTTP、/sse SSE）的端口（默认 8000）。",
            apply: "应用并重启",
            helperTauri: "Tauri：就地应用端口并重启后端。",
            helperWeb: "Web：修改端口后，请在外部重启后端进程。",
            webDialogTitle: "网页环境应用与重启",
            webDialogDesc: "浏览器无法直接重启后端，请复制以下命令并在终端执行（使用上方端口）。",
            optionCargoTitle: "方案 A — cargo run（开发）",
            optionBinaryTitle: "方案 B — 二进制（发布）",
            copy: "复制",
            stopCurrent: "停止当前后端",
            close: "关闭",
        },
		general: {
			title: "通用",
			description: "管理工作区的默认视图与基础偏好。",
			defaultView: "默认视图",
			defaultViewDescription: "选择条目显示的默认布局方式。",
			appMode: "应用模式",
			appModeDescription: "选择界面复杂度和信息层级。",
			language: "界面语言",
			languageDescription: "切换控制台显示语言。",
			languagePlaceholder: "请选择语言",
		},
		appearance: {
			title: "外观",
			description: "自定义仪表盘的外观和感觉。",
			themeTitle: "主题",
			themeDescription: "在浅色和深色模式之间切换。",
			systemPreferenceTitle: "系统偏好",
			systemPreferenceDescription: "自动跟随操作系统偏好设置。",
			menuBarTitle: "菜单栏图标",
			menuBarDescription: "控制菜单栏图标的可见性。",
			menuBarIconTitle: "菜单栏图标模式",
			dockTitle: "Dock 图标",
			dockDescription: "在 macOS Dock 中显示 MCPMate 或从菜单栏静默运行。",
			dockIconTitle: "Dock 图标模式",
			dockHiddenNotice:
				"Dock 图标已隐藏，菜单栏图标会保持可见以便重新打开 MCPMate。",
			menuBarPlaceholder: "选择菜单栏图标模式",
			wipLabel: "开发中",
			defaultMarketPlaceholder: "选择默认市场",
		},
		options: {
			theme: {
				light: "浅色",
				dark: "深色",
			},
			defaultView: {
				list: "列表",
				grid: "网格",
			},
			appMode: {
				express: "简洁",
				expert: "专业",
			},
			clientMode: {
				hosted: "托管",
				transparent: "透明",
			},
			backup: {
				keepN: "保留 N 个",
				keepLast: "保留最新",
				none: "不保留",
			},
			menuBar: {
				runtime: "运行时可见",
				hidden: "隐藏",
			},
		},
		servers: {
			title: "服务器",
			description: "决定服务操作如何在客户端之间传播。",
			syncTitle: "同步全局启停",
			syncDescription: "立即将全局启用状态推送到管理的客户端。",
			autoAddTitle: "自动添加到默认配置文件",
			autoAddDescription: "自动将新服务包含在默认配置文件中。",
		},
		clients: {
			title: "客户端",
			description: "配置客户端应用的默认部署和备份行为。",
			defaultVisibilityTitle: "默认显示内容",
			defaultVisibilityDescription:
				"选择在“客户端”页面默认展示哪些状态的记录。",
			defaultVisibility: {
				all: "全部",
				detected: "已检测",
				managed: "已管理",
			},
            modeTitle: "客户端管理模式",
            modeDescription: "选择客户端配置默认应如何由 MCPMate 管理。",
			backupStrategyTitle: "客户端备份策略",
			backupStrategyDescription: "定义客户端配置应如何备份。",
			backupLimitTitle: "最大备份副本数",
			backupLimitDescription: "设置要保留的最大备份副本数。",
		},
		developer: {
			title: "开发者",
			description: "用于内部检视和导航可见性的实验性开关。",
			enableServerDebugTitle: "启用服务器检视",
			enableServerDebugDescription: "为新添加的服务器公开检视工具。",
			openDebugInNewWindowTitle: "在新窗口中打开检视视图",
			openDebugInNewWindowDescription:
				"启用后，检视按钮将启动单独的标签页而不是导航当前视图。",
			showApiDocsTitle: "显示 API 文档菜单",
			showApiDocsDescription: "在导航中显示 API 文档菜单。",
			showDefaultHeadersTitle: "显示默认 HTTP 头",
			showDefaultHeadersDescription:
				"在服务器详细信息中显示服务器的默认 HTTP 头（值已脱敏）。仅用于检视。",
			showRawJsonTitle: "显示原始能力 JSON",
			showRawJsonDescription:
				"在能力列表中显示原始 JSON 负载（服务器详情和统一导入预览）。",
		},
		market: {
			title: "服务源",
			description: "配置默认服务源并管理隐藏的服务源服务器。",
			defaultMarketTitle: "默认服务源",
			defaultMarketDescription: "选择哪个服务源首先显示且无法关闭。",
			officialPortal: "官方 MCP 注册中心",
			enableBlacklistTitle: "启用黑名单",
			enableBlacklistDescription: "隐藏质量差或不可用的内容以保持服务源清洁",
			searchHiddenServers: "搜索隐藏服务器",
			sortHiddenServers: "排序隐藏服务器",
			sortPlaceholder: "排序",
			emptyTitle: "当前没有隐藏的服务器。",
			emptyDescription:
				"从服务源列表中隐藏服务器以保持此空间整洁。它们将出现在这里以便恢复。",
			noNotes: "未添加备注。",
			hiddenOn: "隐藏于 {{value}}",
			restore: "恢复",
		},
		about: {
			title: "关于 MCPMate",
			description: "MCPMate 预览版本的开源致谢信息。",
			lastUpdated: "最后更新：{{date}}",
			backendTitle: "后端 (Rust 工作区)",
			desktopShellTitle: "桌面外壳 (Tauri)",
			dashboardTitle: "仪表盘 (Web)",
			components: "{{count}} 个组件",
			repository: "Repository",
			homepage: "主页",
			noPackages: "在最新更新期间未检测到第三方包。",
		},
		notices: {
			dockHidden: "Dock 图标已隐藏，菜单栏图标会保持可见以便重新打开 MCPMate。",
		},
	},
"ja-JP": {
    title: "ダッシュボード設定と既定値の調整",
		tabs: {
			general: "一般",
			appearance: "外観",
			serverControls: "サーバー",
			clientDefaults: "クライアント",
			market: "MCP マーケット",
			developer: "開発者",
			about: "情報とライセンス",
		},
		general: {
			title: "一般",
			description: "ワークスペースの基本設定を管理します。",
			defaultView: "既定のビュー",
			defaultViewDescription: "項目の表示レイアウトを選択します。",
			appMode: "アプリモード",
			appModeDescription: "インターフェースの複雑さを選択します。",
			language: "表示言語",
			languageDescription: "ダッシュボードで使用する言語を切り替えます。",
			languagePlaceholder: "言語を選択",
		},
		appearance: {
			title: "外観",
			description: "ダッシュボードの外観と操作性をカスタマイズします。",
			themeTitle: "テーマ",
			themeDescription: "ライトモードとダークモードを切り替えます。",
			systemPreferenceTitle: "システム設定",
			systemPreferenceDescription:
				"オペレーティングシステムの設定を自動的に追従します。",
			menuBarTitle: "メニューバーアイコン",
			menuBarDescription: "メニューバーアイコンの表示を制御します。",
			menuBarIconTitle: "メニューバーアイコンモード",
			dockTitle: "Dock アイコン",
			dockDescription:
				"macOS Dock で MCPMate を表示するか、メニューバーからサイレントに実行します。",
			dockIconTitle: "Dock アイコンモード",
			dockHiddenNotice:
				"Dock アイコンは非表示です。メニューアイコンは引き続き表示され、MCPMate を開き直せます。",
			menuBarPlaceholder: "メニューバーアイコンモードを選択",
			wipLabel: "開発中",
			defaultMarketPlaceholder: "デフォルトマーケットを選択",
		},
		options: {
			theme: {
				light: "ライト",
				dark: "ダーク",
			},
			defaultView: {
				list: "リスト",
				grid: "グリッド",
			},
			appMode: {
				express: "ライト",
				expert: "エキスパート",
			},
			clientMode: {
				hosted: "ホスト",
				transparent: "トランスペアレント",
			},
			backup: {
				keepN: "N 件保持",
				keepLast: "最新のみ",
				none: "保持しない",
			},
			menuBar: {
				runtime: "稼働中のみ表示",
				hidden: "非表示",
			},
		},
		servers: {
			title: "サーバー",
			description:
				"サーバー操作がクライアント間でどのように伝播するかを決定します。",
			syncTitle: "グローバル開始/停止の同期",
			syncDescription:
				"グローバル有効状態を管理されたクライアントに即座にプッシュします。",
			autoAddTitle: "デフォルトプロファイルに自動追加",
			autoAddDescription:
				"新しいサーバーをデフォルトプロファイルに自動的に含めます。",
		},
		clients: {
			title: "クライアント",
			description:
				"クライアントアプリのデフォルトロールアウトとバックアップ動作を設定します。",
			defaultVisibilityTitle: "既定の表示内容",
			defaultVisibilityDescription:
				"クライアントページで既定として表示するステータスを選択します。",
			defaultVisibility: {
				all: "すべて",
				detected: "検出済み",
				managed: "管理中",
			},
            modeTitle: "クライアント管理モード",
            modeDescription:
                "クライアント設定をデフォルトでどのように管理するかを選択します。",
			backupStrategyTitle: "クライアントバックアップ戦略",
			backupStrategyDescription:
				"クライアント設定をどのようにバックアップするかを定義します。",
			backupLimitTitle: "最大バックアップコピー数",
			backupLimitDescription: "保持する最大バックアップコピー数を設定します。",
		},
		developer: {
			title: "開発者",
			description: "内部検査とナビゲーション可視性のための実験的トグル。",
			enableServerDebugTitle: "サーバー検査を有効化",
			enableServerDebugDescription:
				"新しく追加されたサーバーの検査計装を公開します。",
			openDebugInNewWindowTitle: "新しいウィンドウで検査ビューを開く",
			openDebugInNewWindowDescription:
				"有効にすると、検査ボタンは現在のビューをナビゲートする代わりに別のタブを起動します。",
			showApiDocsTitle: "API ドキュメントメニューを表示",
			showApiDocsDescription:
				"ナビゲーションに API ドキュメントメニューを表示します。",
			showDefaultHeadersTitle: "デフォルト HTTP ヘッダーを表示",
			showDefaultHeadersDescription:
				"サーバー詳細でサーバーのデフォルト HTTP ヘッダー（値は編集済み）を表示します。検査専用です。",
		},
		market: {
			title: "MCP マーケット",
			description:
				"デフォルトマーケットを設定し、非表示のマーケットプレイスサーバーを管理します。",
			defaultMarketTitle: "デフォルトマーケット",
			defaultMarketDescription:
				"最初に表示され、閉じることができないマーケットを選択します。",
			officialPortal: "公式 MCP レジストリ",
			enableBlacklistTitle: "ブラックリストを有効化",
			enableBlacklistDescription:
				"品質の悪いまたは利用できないコンテンツを非表示にしてマーケットを清潔に保ちます",
			searchHiddenServers: "非表示サーバーを検索",
			sortHiddenServers: "非表示サーバーを並べ替え",
			sortPlaceholder: "並べ替え",
			emptyTitle: "現在非表示のサーバーはありません。",
			emptyDescription:
				"マーケットリストからサーバーを非表示にして、このスペースを整理します。復元のためにここに表示されます。",
			noNotes: "メモが追加されていません。",
			hiddenOn: "非表示日時：{{value}}",
			restore: "復元",
		},
		about: {
			title: "MCPMate について",
			description: "MCPMate プレビュービルドのオープンソース謝辞。",
			lastUpdated: "最終更新：{{date}}",
			backendTitle: "バックエンド (Rust ワークスペース)",
			desktopShellTitle: "デスクトップシェル (Tauri)",
			dashboardTitle: "ダッシュボード (Web)",
			components: "{{count}} コンポーネント",
			repository: "リポジトリ",
			homepage: "ホームページ",
			noPackages:
				"最新の更新中にサードパーティパッケージが検出されませんでした。",
		},
		notices: {
			dockHidden:
				"Dock アイコンは非表示です。メニューアイコンは引き続き表示され、MCPMate を開き直せます。",
		},
    },
    ja: {
        tabs: {
            general: "一般",
            appearance: "外観",
            system: "システム",
            serverControls: "サーバー",
            clientDefaults: "クライアント",
            market: "MCP マーケット",
            developer: "開発者",
            about: "概要",
        },
        system: {
            title: "システム",
            description: "ローカル MCP プロキシと API が使用するバックエンドポートを設定します。",
            apiPortTitle: "バックエンド API ポート",
            apiPortDescription: "REST とダッシュボード用ポート（既定 8080）。",
            mcpPortTitle: "バックエンド MCP サーバーポート",
            mcpPortDescription: "MCP トランスポート用ポート（/mcp は HTTP、/sse は SSE、既定 8000）。",
            apply: "適用して再起動",
            helperTauri: "Tauri：ポートを適用してバックエンドをその場で再起動します。",
            helperWeb: "Web：ポート変更後に外部でバックエンドを再起動してください。",
            webDialogTitle: "Web 環境での適用と再起動",
            webDialogDesc: "ブラウザからバックエンドは再起動できません。以下のコマンドを選択したポートで実行してください。",
            optionCargoTitle: "方法 A — cargo run（開発）",
            optionBinaryTitle: "方法 B — バイナリ（リリース）",
            copy: "コピー",
            stopCurrent: "現在のバックエンドを停止",
            close: "閉じる",
        },
    },
} as const;
