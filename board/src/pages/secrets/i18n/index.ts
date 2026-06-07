export const secretsTranslations = {
	en: {
		title: "Secure Store",
		toolbar: {
			search: {
				placeholder: "Search secrets...",
				fields: {
					alias: "Alias",
					label: "Label",
					kind: "Kind",
				},
			},
			sort: {
				options: {
					alias: "Alias",
					kind: "Kind",
					usage: "Usage",
				},
			},
			actions: {
				refresh: "Refresh",
				add: "Add Secret",
			},
		},
		kind: {
			generic: "Generic",
			token: "Token",
			api_key: "API key",
			password: "Password",
			oauth_access_token: "OAuth access",
			oauth_refresh_token: "OAuth refresh",
			url_credential: "URL credential",
			header_value: "Header value",
		},
		provider: {
			operating_system_keychain: "OS Keychain",
			passphrase_wrapped_root_key: "Password",
			local_file_root_key: "Local File",
			local_encrypted_vault: "Development Vault",
		},
		list: {
			error: "Failed to load secrets. The secure store may be unavailable.",
			retry: "Retry",
			stats: {
				provider: "Provider",
				usage: "Usage",
				version: "Version",
			},
			actions: {
				viewUsage: "View usage",
				edit: "Edit secret",
				delete: "Delete secret",
			},
		},
		empty: {
			title: "No secrets stored",
			filteredTitle: "No matching secrets",
			description: "Store write-only values for server runtime placeholders.",
			filteredDescription:
				"Adjust the search or sort controls to find a secret.",
			action: "Add First Secret",
		},
		editor: {
			createTitle: "Add Secret",
			editTitle: "Edit Secret",
			description:
				"The value is write-only. It will not be shown again after save.",
			tabs: {
				general: "General",
				usage: "Usage",
			},
			fields: {
				alias: "Alias",
				kind: "Kind",
				label: "Label",
				value: "Value",
			},
			placeholders: {
				alias: "server-context7-url-parameters-token",
				label: "context7 · URL parameter · token",
				keepValue: "Leave blank to keep existing value",
				value: "Secret value",
			},
			actions: {
				cancel: "Cancel",
				copyPlaceholder: "Copy placeholder",
				save: "Save",
			},
		},
		usage: {
			title: "Secret Usage",
			loading: "Loading usages",
			empty: "No server usage recorded",
			sections: {
				active: "Active bindings",
				activeDescription:
					"Servers that currently reference this secret in runtime config.",
				stale: "Historical bindings",
				staleDescription:
					"Former references left after a server was deleted or the secret was removed from config.",
			},
			status: {
				active: "Active",
				stale: "Stale",
			},
			columns: {
				server: "Server",
				location: "Location",
			},
			location: {
				stdioEnv: "stdio env {{name}}",
				stdioArgument: "stdio arg {{index}}",
				httpHeader: "http header {{name}}",
				stdioCommand: "stdio command",
				httpUrl: "http url",
				oauthToken: "oauth token",
			},
		},
		delete: {
			title: "Delete secret?",
			description:
				"This removes the encrypted value only when no server usage is recorded.",
			actions: {
				cancel: "Cancel",
				confirm: "Delete",
			},
		},
		notifications: {
			saveSuccess: "Secret saved",
			saveError: "Failed to save secret",
			deleteSuccess: "Secret deleted",
			deleteError: "Failed to delete secret",
			copySuccess: "Placeholder copied",
			copyError: "Failed to copy placeholder",
		},
		originLabel: {
			serverFallback: "Server",
			urlParameter: "URL parameter",
			environmentVariable: "Environment variable",
			httpHeader: "HTTP header",
			argument: "Argument",
			command: "Command",
			stdioField: "Stdio field",
			serverUrl: "Server URL",
			field: "Field",
		},
		status: {
			error: {
				title: "Store status check failed",
				description:
					"Could not determine store status. Operations are disabled.",
			},
			unavailable: {
				title: "Secure store unavailable",
				description:
					"The secret store is not ready. Create and update operations are disabled until the issue is resolved.",
			},
		},
		stats: {
			stored: {
				title: "Stored Secrets",
				description: "in secure store",
			},
			inUse: {
				title: "In Use",
				description: "linked to servers",
			},
			usageRefs: {
				title: "Usage References",
				description: "runtime bindings",
			},
			store: {
				title: "Secure Store",
				checking: "checking status",
				ready: "Ready",
				readyDescription: "available for use",
				locked: "Locked",
				lockedDescription: "unlock required",
				issue: "Issue",
				issueDescription: "needs attention",
			},
		},
	},
	"zh-CN": {
		title: "安全存储",
		toolbar: {
			search: {
				placeholder: "搜索安全记录...",
				fields: {
					alias: "别名",
					label: "标签",
					kind: "类型",
				},
			},
			sort: {
				options: {
					alias: "别名",
					kind: "类型",
					usage: "使用次数",
				},
			},
			actions: {
				refresh: "刷新",
				add: "添加安全记录",
			},
		},
		kind: {
			generic: "通用",
			token: "令牌",
			api_key: "API Key",
			password: "密码",
			oauth_access_token: "OAuth access",
			oauth_refresh_token: "OAuth refresh",
			url_credential: "URL 凭据",
			header_value: "Header 值",
		},
		provider: {
			operating_system_keychain: "系统钥匙串",
			passphrase_wrapped_root_key: "密码",
			local_file_root_key: "本地文件",
			local_encrypted_vault: "开发加密库",
		},
		list: {
			error: "加载密钥失败，安全存储可能不可用。",
			retry: "重试",
			stats: {
				provider: "提供方",
				usage: "使用",
				version: "版本",
			},
			actions: {
				viewUsage: "查看使用情况",
				edit: "编辑安全记录",
				delete: "删除安全记录",
			},
		},
		empty: {
			title: "暂无安全记录",
			filteredTitle: "没有匹配的安全记录",
			description: "为服务器运行时占位符保存写入后不可读的值。",
			filteredDescription: "调整搜索或排序条件以查找安全记录。",
			action: "添加第一条安全记录",
		},
		editor: {
			createTitle: "添加安全记录",
			editTitle: "编辑安全记录",
			description: "值保存后不可读取，也不会再次显示。",
			tabs: {
				general: "常规",
				usage: "使用情况",
			},
			fields: {
				alias: "别名",
				kind: "类型",
				label: "标签",
				value: "值",
			},
			placeholders: {
				alias: "server-context7-url-parameters-token",
				label: "context7 · URL 参数 · token",
				keepValue: "留空以保留当前值",
				value: "安全值",
			},
			actions: {
				cancel: "取消",
				copyPlaceholder: "复制占位符",
				save: "保存",
			},
		},
		usage: {
			title: "安全记录使用情况",
			loading: "正在加载使用情况",
			empty: "暂无服务器使用记录",
			sections: {
				active: "生效中的引用",
				activeDescription: "当前在服务器运行时配置中仍引用此安全记录的位置。",
				stale: "历史引用",
				staleDescription: "服务器已删除，或配置中已移除此安全记录后遗留的引用。",
			},
			status: {
				active: "生效中",
				stale: "已失效",
			},
			columns: {
				server: "服务器",
				location: "位置",
			},
			location: {
				stdioEnv: "stdio env {{name}}",
				stdioArgument: "stdio 参数 {{index}}",
				httpHeader: "http header {{name}}",
				stdioCommand: "stdio 命令",
				httpUrl: "http url",
				oauthToken: "oauth token",
			},
		},
		delete: {
			title: "删除安全记录？",
			description: "仅当没有服务器使用记录时，才会移除加密值。",
			actions: {
				cancel: "取消",
				confirm: "删除",
			},
		},
		notifications: {
			saveSuccess: "安全记录已保存",
			saveError: "保存安全记录失败",
			deleteSuccess: "安全记录已删除",
			deleteError: "删除安全记录失败",
			copySuccess: "占位符已复制",
			copyError: "复制占位符失败",
		},
		originLabel: {
			serverFallback: "服务器",
			urlParameter: "URL 参数",
			environmentVariable: "环境变量",
			httpHeader: "HTTP 头",
			argument: "参数",
			command: "命令",
			stdioField: "Stdio 字段",
			serverUrl: "服务器 URL",
			field: "字段",
		},
		status: {
			error: {
				title: "存储状态检查失败",
				description: "无法确定存储状态，相关操作已禁用。",
			},
			unavailable: {
				title: "安全存储不可用",
				description: "密钥存储尚未就绪，在问题解决前无法创建或更新。",
			},
		},
		stats: {
			stored: {
				title: "已存储密钥",
				description: "在安全存储中",
			},
			inUse: {
				title: "使用中",
				description: "已关联服务器",
			},
			usageRefs: {
				title: "使用引用",
				description: "运行时绑定",
			},
			store: {
				title: "安全存储",
				checking: "正在检查状态",
				ready: "就绪",
				readyDescription: "可供使用",
				locked: "已锁定",
				lockedDescription: "需要解锁",
				issue: "异常",
				issueDescription: "需要处理",
			},
		},
	},
	"ja-JP": {
		title: "セキュアストア",
		toolbar: {
			search: {
				placeholder: "シークレットを検索...",
				fields: {
					alias: "エイリアス",
					label: "ラベル",
					kind: "種別",
				},
			},
			sort: {
				options: {
					alias: "エイリアス",
					kind: "種別",
					usage: "使用数",
				},
			},
			actions: {
				refresh: "更新",
				add: "シークレットを追加",
			},
		},
		kind: {
			generic: "汎用",
			token: "トークン",
			api_key: "API キー",
			password: "パスワード",
			oauth_access_token: "OAuth access",
			oauth_refresh_token: "OAuth refresh",
			url_credential: "URL 認証情報",
			header_value: "Header 値",
		},
		provider: {
			operating_system_keychain: "OS キーチェーン",
			passphrase_wrapped_root_key: "パスワード",
			local_file_root_key: "ローカルファイル",
			local_encrypted_vault: "開発用ボルト",
		},
		list: {
			error: "シークレットの読み込みに失敗しました。セキュアストアが利用できない可能性があります。",
			retry: "再試行",
			stats: {
				provider: "プロバイダー",
				usage: "使用",
				version: "バージョン",
			},
			actions: {
				viewUsage: "使用状況を表示",
				edit: "シークレットを編集",
				delete: "シークレットを削除",
			},
		},
		empty: {
			title: "保存済みシークレットはありません",
			filteredTitle: "一致するシークレットはありません",
			description:
				"サーバー実行時プレースホルダー用の書き込み専用値を保存します。",
			filteredDescription: "検索または並び替え条件を調整してください。",
			action: "最初のシークレットを追加",
		},
		editor: {
			createTitle: "シークレットを追加",
			editTitle: "シークレットを編集",
			description: "値は書き込み専用です。保存後に再表示されません。",
			tabs: {
				general: "一般",
				usage: "使用状況",
			},
			fields: {
				alias: "エイリアス",
				kind: "種別",
				label: "ラベル",
				value: "値",
			},
			placeholders: {
				alias: "server-context7-url-parameters-token",
				label: "context7 · URL パラメータ · token",
				keepValue: "空欄のままにすると既存値を維持します",
				value: "シークレット値",
			},
			actions: {
				cancel: "キャンセル",
				copyPlaceholder: "プレースホルダーをコピー",
				save: "保存",
			},
		},
		usage: {
			title: "シークレットの使用状況",
			loading: "使用状況を読み込み中",
			empty: "サーバー使用記録はありません",
			sections: {
				active: "有効な参照",
				activeDescription: "現在サーバーのランタイム設定でこのシークレットを参照している場所。",
				stale: "履歴参照",
				staleDescription:
					"サーバー削除後、または設定からシークレットを外した後に残った参照。",
			},
			status: {
				active: "有効",
				stale: "失効",
			},
			columns: {
				server: "サーバー",
				location: "場所",
			},
			location: {
				stdioEnv: "stdio env {{name}}",
				stdioArgument: "stdio arg {{index}}",
				httpHeader: "http header {{name}}",
				stdioCommand: "stdio command",
				httpUrl: "http url",
				oauthToken: "oauth token",
			},
		},
		delete: {
			title: "シークレットを削除しますか？",
			description:
				"サーバー使用記録がない場合のみ、暗号化された値を削除します。",
			actions: {
				cancel: "キャンセル",
				confirm: "削除",
			},
		},
		notifications: {
			saveSuccess: "シークレットを保存しました",
			saveError: "シークレットの保存に失敗しました",
			deleteSuccess: "シークレットを削除しました",
			deleteError: "シークレットの削除に失敗しました",
			copySuccess: "プレースホルダーをコピーしました",
			copyError: "プレースホルダーのコピーに失敗しました",
		},
		originLabel: {
			serverFallback: "サーバー",
			urlParameter: "URL パラメータ",
			environmentVariable: "環境変数",
			httpHeader: "HTTP ヘッダー",
			argument: "引数",
			command: "コマンド",
			stdioField: "Stdio フィールド",
			serverUrl: "サーバー URL",
			field: "フィールド",
		},
		status: {
			error: {
				title: "ストア状態の確認に失敗しました",
				description:
					"ストア状態を取得できません。操作は無効になっています。",
			},
			unavailable: {
				title: "セキュアストアは利用できません",
				description:
					"シークレットストアの準備ができていません。問題が解決するまで作成・更新は無効です。",
			},
		},
		stats: {
			stored: {
				title: "保存済みシークレット",
				description: "セキュアストア内",
			},
			inUse: {
				title: "使用中",
				description: "サーバーにリンク済み",
			},
			usageRefs: {
				title: "使用参照",
				description: "ランタイムバインディング",
			},
			store: {
				title: "セキュアストア",
				checking: "状態を確認中",
				ready: "準備完了",
				readyDescription: "利用可能",
				locked: "ロック中",
				lockedDescription: "アンロックが必要",
				issue: "問題あり",
				issueDescription: "要対応",
			},
		},
	},
};
