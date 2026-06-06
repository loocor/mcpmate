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
		list: {
			error: "Failed to load secrets. The secure store may be unavailable.",
			retry: "Retry",
			stats: {
				provider: "Provider",
				usage: "Usage",
				version: "Version",
			},
			actions: {
				copy: "Copy placeholder",
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
			fields: {
				alias: "Alias",
				kind: "Kind",
				label: "Label",
				value: "Value",
			},
			placeholders: {
				keepValue: "Leave blank to keep existing value",
				value: "Secret value",
			},
			actions: {
				cancel: "Cancel",
				save: "Save",
			},
		},
		usage: {
			title: "Secret Usage",
			loading: "Loading usages",
			empty: "No server usage recorded",
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
		list: {
			error: "加载密钥失败，安全存储可能不可用。",
			retry: "重试",
			stats: {
				provider: "Provider",
				usage: "使用",
				version: "版本",
			},
			actions: {
				copy: "复制占位符",
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
			fields: {
				alias: "别名",
				kind: "类型",
				label: "标签",
				value: "值",
			},
			placeholders: {
				keepValue: "留空以保留当前值",
				value: "安全值",
			},
			actions: {
				cancel: "取消",
				save: "保存",
			},
		},
		usage: {
			title: "安全记录使用情况",
			loading: "正在加载使用情况",
			empty: "暂无服务器使用记录",
			columns: {
				server: "服务器",
				location: "位置",
			},
			location: {
				stdioEnv: "stdio 环境变量 {{name}}",
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
		list: {
			error: "シークレットの読み込みに失敗しました。セキュアストアが利用できない可能性があります。",
			retry: "再試行",
			stats: {
				provider: "Provider",
				usage: "使用",
				version: "バージョン",
			},
			actions: {
				copy: "プレースホルダーをコピー",
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
			fields: {
				alias: "エイリアス",
				kind: "種別",
				label: "ラベル",
				value: "値",
			},
			placeholders: {
				keepValue: "空欄のままにすると既存値を維持します",
				value: "シークレット値",
			},
			actions: {
				cancel: "キャンセル",
				save: "保存",
			},
		},
		usage: {
			title: "シークレットの使用状況",
			loading: "使用状況を読み込み中",
			empty: "サーバー使用記録はありません",
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
		},
	},
};
