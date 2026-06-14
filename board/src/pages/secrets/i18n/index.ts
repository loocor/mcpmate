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
			oauth_client_secret: "OAuth client",
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
			unavailable: "Unavailable",
		},
		lifecycle: {
			state: {
				all: "All lifecycle states",
				active: "Active",
				unused: "Unused",
				oauth_managed: "OAuth managed",
			},
			description: {
				active: "Currently referenced by at least one server.",
				unused: "Not currently referenced by any server.",
				oauth_managed: "Owned by OAuth and cleaned up with OAuth lifecycle actions.",
			},
		},
		list: {
			error: "Failed to load secrets. The secure store may be unavailable.",
			retry: "Retry",
			stats: {
				provider: "Provider",
				usage: "Usage",
				history: "History",
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
			filteredLifecycleDescription:
				"Adjust the lifecycle filter or search controls to find a secret.",
			action: "Add First Secret",
		},
		editor: {
			createTitle: "Add Secret",
			editTitle: "Edit Secret",
			description:
				"The value is write-only. It will not be shown again after save.",
			providerUnavailableDescription:
				"Provider unavailable. Record is kept; manage recovery in Security settings.",
			tabs: {
				general: "General",
				usage: "Usage",
			},
			fields: {
				alias: "Alias",
				kind: "Kind",
				label: "Label",
				value: "Value",
				storedValue:
					"Stored secret value is hidden. Focus to replace it.",
			},
			kindLockedDescription: "Kind is set at creation and cannot be changed.",
			oauthManagedDescription:
				"Managed by OAuth. Reconnect or revoke OAuth to update this credential.",
			oauthOrphanedDescription:
				"Orphaned OAuth credential. No active owner was found; delete it if it is no longer needed.",
			placeholders: {
				alias: "server-context7-url-parameters-token",
				label: "context7 · URL parameter · token",
				keepValue: "Leave blank to keep existing value",
				oauthManagedValue:
					"Managed by OAuth; reconnect to update this value",
				value: "Secret value",
			},
			actions: {
				cancel: "Cancel",
				copyPlaceholder: "Copy placeholder",
				copyPlaceholderDescription:
					"Copy the [[secret:alias]] placeholder to paste into server env, headers, or args.",
				create: "Create Record",
				save: "Save Changes",
				delete: "Delete",
				deleteDisabledTooltip:
					"Cannot delete: secret is actively used by {{count}} location(s)",
				deleteDisabledOAuthTooltip:
					"OAuth-managed credentials are removed by OAuth revoke or server deletion.",
			},
		},
		usage: {
			title: "Secret Usage",
			loading: "Loading usages",
			empty: "No server usage recorded",
			emptyDescription:
				"This secret has no active or historical server binding.",
			summary: {
				active: "Active {{count}}",
				historical: "Historical {{count}}",
				canDelete: "No active runtime binding is using this secret.",
				blocked: "Remove active bindings before deleting this secret.",
				oauthManaged:
					"OAuth credentials are cleaned up by OAuth revoke or server deletion.",
			},
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
			actions: {
				openServer: "Open server {{name}}",
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
				"This removes the encrypted value only when no active usage is recorded.",
			descriptionActive:
				"This secret is still actively used. Remove active bindings before deleting it.",
			descriptionUnused:
				"This removes the encrypted value. No active usage is recorded.",
			descriptionOAuth:
				"OAuth-managed credentials are normally removed by OAuth revoke or server deletion. Delete only orphaned OAuth records.",
			usageSummary: "Active {{active}} · Historical {{historical}}",
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
		},
		guidance: {
			providerUnavailable: {
				title: "Secure store provider unavailable",
				description:
					"The configured root-key provider could not be initialized. Retry after fixing the environment, or choose a different security mode in Settings → Security.",
				os: {
					title: "OS secure storage is unavailable",
					description:
						"MCPMate could not access the OS keychain. Grant access when prompted, unlock Keychain Access on macOS, or switch to Password or Local File mode below.",
				},
			},
			readLockFailed: {
				title: "Secure store is busy",
				description:
					"MCPMate could not read the secure store status. Wait a moment and retry.",
			},
			missingRootKey: {
				title: "Root key material is missing",
				description:
					"Existing encrypted secrets need the original root key material. Restore access to the configured provider before editing stored secrets or switching encryption mode.",
			},
			generic: {
				title: "Secure store unavailable",
				description:
					"Secret storage is not ready. Create and update operations stay disabled until the issue is resolved.",
			},
			actions: {
				retryProvider: "Retry secure storage",
				retryStatus: "Retry status check",
				openSecuritySettings: "Open Security settings",
			},
			notifications: {
				retrySuccess: "Secure store status refreshed",
				retryError: "Failed to retry secure storage",
				retryStillUnavailable: "Secure store is still unavailable",
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
			unused: {
				title: "Unused",
				description: "not linked",
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
			oauth_client_secret: "OAuth client",
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
			unavailable: "不可用",
		},
		lifecycle: {
			state: {
				all: "全部生命周期状态",
				active: "使用中",
				unused: "未使用",
				oauth_managed: "OAuth 托管",
			},
			description: {
				active: "当前至少被一个服务器引用。",
				unused: "当前没有被任何服务器引用。",
				oauth_managed: "由 OAuth 拥有，并随 OAuth 生命周期动作清理。",
			},
		},
		list: {
			error: "加载密钥失败，安全存储可能不可用。",
			retry: "重试",
			stats: {
				provider: "提供方",
				usage: "使用",
				history: "历史",
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
			filteredLifecycleDescription:
				"调整生命周期筛选或搜索条件以查找安全记录。",
			action: "添加第一条安全记录",
		},
		editor: {
			createTitle: "添加安全记录",
			editTitle: "编辑安全记录",
			description: "值保存后不可读取，也不会再次显示。",
			providerUnavailableDescription:
				"提供方不可用，记录已保留；如需恢复请前往安全设置处理。",
			tabs: {
				general: "常规",
				usage: "使用情况",
			},
			fields: {
				alias: "别名",
				kind: "类型",
				label: "标签",
				value: "值",
				storedValue: "已保存的密钥值已隐藏。聚焦后可替换。",
			},
			kindLockedDescription: "类型在创建时确定，之后不能修改。",
			oauthManagedDescription:
				"由 OAuth 自动维护。请通过重新连接或撤销 OAuth 来更新这条凭据。",
			oauthOrphanedDescription:
				"孤立的 OAuth 凭据。当前没有找到活跃归属；如果不再需要，可以删除。",
			placeholders: {
				alias: "server-context7-url-parameters-token",
				label: "context7 · URL 参数 · token",
				keepValue: "留空以保留当前值",
				oauthManagedValue: "由 OAuth 自动维护；请重新连接以更新此值",
				value: "安全值",
			},
			actions: {
				cancel: "取消",
				copyPlaceholder: "复制占位符",
				copyPlaceholderDescription:
					"复制 [[secret:alias]] 占位符，以便粘贴到服务器的 env、header 或 args 配置中。",
				create: "创建记录",
				save: "保存更改",
				delete: "删除",
				deleteDisabledTooltip:
					"无法删除：该安全记录仍被 {{count}} 处活跃使用引用",
				deleteDisabledOAuthTooltip:
					"OAuth 托管凭据请通过撤销 OAuth 或删除服务器来移除。",
			},
		},
		usage: {
			title: "安全记录使用情况",
			loading: "正在加载使用情况",
			empty: "暂无服务器使用记录",
			emptyDescription: "此安全记录没有活跃或历史服务器绑定。",
			summary: {
				active: "活跃 {{count}}",
				historical: "历史 {{count}}",
				canDelete: "当前没有运行时绑定正在使用此安全记录。",
				blocked: "删除此安全记录前，需要先移除活跃绑定。",
				oauthManaged: "OAuth 凭据会随 OAuth 撤销或服务器删除自动清理。",
			},
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
			actions: {
				openServer: "打开服务器 {{name}}",
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
			description: "仅当没有活跃使用记录时，才会移除加密值。",
			descriptionActive: "此安全记录仍在使用中。删除前请先移除活跃绑定。",
			descriptionUnused: "这会移除加密值。当前没有活跃使用记录。",
			descriptionOAuth:
				"OAuth 托管凭据通常会通过 OAuth 撤销或服务器删除自动移除。仅删除孤立的 OAuth 记录。",
			usageSummary: "活跃 {{active}} · 历史 {{historical}}",
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
		},
		guidance: {
			providerUnavailable: {
				title: "安全存储提供方不可用",
				description:
					"配置的根密钥提供方无法初始化。请修复环境后重试，或在设置 → 安全中选择其他安全模式。",
				os: {
					title: "操作系统安全存储不可用",
					description:
						"MCPMate 无法访问系统钥匙串。请在提示时授予访问权限、在 macOS 上解锁钥匙串访问，或在设置 → 安全中切换到密码或本地文件模式。",
				},
			},
			readLockFailed: {
				title: "安全存储繁忙",
				description: "MCPMate 无法读取安全存储状态，请稍候后重试。",
			},
			missingRootKey: {
				title: "缺少根密钥材料",
				description:
					"已有加密密钥需要原始根密钥材料才能恢复。请先恢复对当前提供方的访问，再编辑已存储密钥或切换加密模式。",
			},
			generic: {
				title: "安全存储不可用",
				description: "密钥存储尚未就绪，在问题解决前无法创建或更新。",
			},
			actions: {
				retryProvider: "重试安全存储",
				retryStatus: "重试状态检查",
				openSecuritySettings: "打开安全设置",
			},
			notifications: {
				retrySuccess: "安全存储状态已刷新",
				retryError: "重试安全存储失败",
				retryStillUnavailable: "安全存储仍然不可用",
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
			unused: {
				title: "未使用",
				description: "未关联",
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
			oauth_client_secret: "OAuth client",
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
			unavailable: "利用不可",
		},
		lifecycle: {
			state: {
				all: "すべてのライフサイクル状態",
				active: "使用中",
				unused: "未使用",
				oauth_managed: "OAuth 管理",
			},
			description: {
				active: "少なくとも 1 つのサーバーから現在参照されています。",
				unused: "現在どのサーバーからも参照されていません。",
				oauth_managed: "OAuth が所有し、OAuth ライフサイクル操作で削除されます。",
			},
		},
		list: {
			error: "シークレットの読み込みに失敗しました。セキュアストアが利用できない可能性があります。",
			retry: "再試行",
			stats: {
				provider: "プロバイダー",
				usage: "使用",
				history: "履歴",
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
			filteredLifecycleDescription:
				"ライフサイクルフィルターまたは検索条件を調整してください。",
			action: "最初のシークレットを追加",
		},
		editor: {
			createTitle: "シークレットを追加",
			editTitle: "シークレットを編集",
			description: "値は書き込み専用です。保存後に再表示されません。",
			providerUnavailableDescription:
				"プロバイダーは利用できません。記録は保持されます。復旧はセキュリティ設定で行ってください。",
			tabs: {
				general: "一般",
				usage: "使用状況",
			},
			fields: {
				alias: "エイリアス",
				kind: "種別",
				label: "ラベル",
				value: "値",
				storedValue:
					"保存済みのシークレット値は非表示です。フォーカスすると置き換えできます。",
			},
			kindLockedDescription: "種別は作成時に設定され、後から変更できません。",
			oauthManagedDescription:
				"OAuth によって管理されています。この認証情報を更新するには再接続または OAuth の取り消しを行ってください。",
			oauthOrphanedDescription:
				"孤立した OAuth 認証情報です。有効な所有元が見つかりません。不要であれば削除できます。",
			placeholders: {
				alias: "server-context7-url-parameters-token",
				label: "context7 · URL パラメータ · token",
				keepValue: "空欄のままにすると既存値を維持します",
				oauthManagedValue:
					"OAuth によって管理されています。更新するには再接続してください",
				value: "シークレット値",
			},
			actions: {
				cancel: "キャンセル",
				copyPlaceholder: "プレースホルダーをコピー",
				copyPlaceholderDescription:
					"[[secret:alias]] プレースホルダーをコピーし、サーバーの env、header、args に貼り付けます。",
				create: "レコードを作成",
				save: "変更を保存",
				delete: "削除",
				deleteDisabledTooltip:
					"削除できません：このシークレットは {{count}} 件の有効な使用箇所で参照されています",
				deleteDisabledOAuthTooltip:
					"OAuth 管理の認証情報は、OAuth の取り消しまたはサーバー削除で削除されます。",
			},
		},
		usage: {
			title: "シークレットの使用状況",
			loading: "使用状況を読み込み中",
			empty: "サーバー使用記録はありません",
			emptyDescription:
				"このシークレットには有効または履歴のサーバーバインディングがありません。",
			summary: {
				active: "有効 {{count}}",
				historical: "履歴 {{count}}",
				canDelete: "このシークレットを使用中のランタイムバインディングはありません。",
				blocked: "削除する前に有効なバインディングを削除してください。",
				oauthManaged:
					"OAuth 認証情報は OAuth の取り消しまたはサーバー削除で自動削除されます。",
			},
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
			actions: {
				openServer: "サーバー {{name}} を開く",
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
				"アクティブな使用記録がない場合のみ、暗号化された値を削除します。",
			descriptionActive:
				"このシークレットはまだ使用中です。削除する前に有効なバインディングを削除してください。",
			descriptionUnused:
				"暗号化された値を削除します。有効な使用記録はありません。",
			descriptionOAuth:
				"OAuth 管理の認証情報は通常、OAuth の取り消しまたはサーバー削除で削除されます。孤立した OAuth レコードのみ削除してください。",
			usageSummary: "有効 {{active}} · 履歴 {{historical}}",
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
		},
		guidance: {
			providerUnavailable: {
				title: "セキュアストアプロバイダーが利用できません",
				description:
					"設定されたルートキープロバイダーを初期化できませんでした。環境を修正して再試行するか、設定 → セキュリティで別のセキュリティモードを選択してください。",
				os: {
					title: "OS セキュアストレージが利用できません",
					description:
						"MCPMate は OS キーチェーンにアクセスできませんでした。プロンプトでアクセスを許可するか、macOS ではキーチェーンアクセスのロックを解除するか、設定 → セキュリティでパスワードまたはローカルファイルモードに切り替えてください。",
				},
			},
			readLockFailed: {
				title: "セキュアストアがビジー状態です",
				description:
					"MCPMate はセキュアストアの状態を読み取れませんでした。しばらく待ってから再試行してください。",
			},
			missingRootKey: {
				title: "ルートキー素材が見つかりません",
				description:
					"既存の暗号化済みシークレットを復元するには元のルートキー素材が必要です。保存済みシークレットの編集や暗号化モードの切り替えの前に、設定済みプロバイダーへのアクセスを復旧してください。",
			},
			generic: {
				title: "セキュアストアは利用できません",
				description:
					"シークレットストアの準備ができていません。問題が解決するまで作成・更新は無効です。",
			},
			actions: {
				retryProvider: "セキュアストレージを再試行",
				retryStatus: "状態確認を再試行",
				openSecuritySettings: "セキュリティ設定を開く",
			},
			notifications: {
				retrySuccess: "セキュアストアの状態を更新しました",
				retryError: "セキュアストレージの再試行に失敗しました",
				retryStillUnavailable: "セキュアストアはまだ利用できません",
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
			unused: {
				title: "未使用",
				description: "未リンク",
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
