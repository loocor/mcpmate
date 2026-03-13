export const marketTranslations = {
    en: {
        title: "Explore and install MCP servers",
		alpha: "alpha",
		buttons: {
			refresh: "Refresh",
			top: "Top",
		},
		search: {
			placeholder: "Search by server name",
			sort: "Sort",
			recentlyUpdated: "Recently updated",
			alphabetical: "Alphabetical",
		},
		errors: {
			failedToLoadRegistry: "Failed to load registry",
		},
		notifications: {
			serverHidden: "Server hidden",
			configurationDetected: "Configuration detected",
			importFailed: "Import failed",
			reviewImportedSnippet:
				"Review the imported snippet before completing the setup.",
		},
		emptyState: {
			noEntriesMatched:
				"No entries matched your filters. Try another name or clear the search above.",
			loading: "Loading...",
		},
		thirdParty: {
			portal: "Third-Party Portal",
			contentWillDisplay: "Third-party portal content will be displayed here",
			urlNotConfigured: "Not configured",
		},
		transport: {
			selectOption: "Select Transport Option",
			remoteEndpoint: "Remote endpoint",
			packageInstallation: "Package installation",
		},
		officialRegistry: "Official MCP Registry",
	},
    "zh-CN": {
        title: "浏览并获取 MCP 服务源",
		alpha: "alpha",
		buttons: {
			refresh: "刷新",
			top: "顶部",
		},
		search: {
			placeholder: "按服务器名称搜索",
			sort: "排序",
			recentlyUpdated: "最近更新",
			alphabetical: "按字母顺序",
		},
		errors: {
			failedToLoadRegistry: "加载注册表失败",
		},
		notifications: {
			serverHidden: "服务器已隐藏",
			configurationDetected: "检测到配置",
			importFailed: "导入失败",
			reviewImportedSnippet: "在完成设置之前，请查看导入的代码片段。",
		},
		emptyState: {
			noEntriesMatched:
				"没有条目匹配您的筛选条件。请尝试其他名称或清除上面的搜索。",
			loading: "加载中...",
		},
		thirdParty: {
			portal: "第三方门户",
			contentWillDisplay: "第三方门户内容将在这里显示",
			urlNotConfigured: "未配置",
		},
		transport: {
			selectOption: "选择传输选项",
			remoteEndpoint: "远程端点",
			packageInstallation: "包安装",
		},
		officialRegistry: "官方 MCP 注册中心",
	},
    "ja-JP": {
        title: "MCP サーバーを探索・インストール",
		alpha: "alpha",
		buttons: {
			refresh: "更新",
			top: "トップ",
		},
		search: {
			placeholder: "サーバー名で検索",
			sort: "並び替え",
			recentlyUpdated: "最近更新",
			alphabetical: "アルファベット順",
		},
		errors: {
			failedToLoadRegistry: "レジストリの読み込みに失敗しました",
		},
		notifications: {
			serverHidden: "サーバーが非表示になりました",
			configurationDetected: "設定が検出されました",
			importFailed: "インポートに失敗しました",
			reviewImportedSnippet:
				"設定を完了する前に、インポートされたスニペットを確認してください。",
		},
		emptyState: {
			noEntriesMatched:
				"フィルターに一致するエントリがありません。別の名前を試すか、上記の検索をクリアしてください。",
			loading: "読み込み中...",
		},
		thirdParty: {
			portal: "サードパーティポータル",
			contentWillDisplay:
				"サードパーティポータルのコンテンツがここに表示されます",
			urlNotConfigured: "未設定",
		},
		transport: {
			selectOption: "トランスポートオプションを選択",
			remoteEndpoint: "リモートエンドポイント",
			packageInstallation: "パッケージインストール",
		},
		officialRegistry: "公式 MCP レジストリ",
	},
} as const;
