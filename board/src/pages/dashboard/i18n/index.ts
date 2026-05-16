export const dashboardTranslations = {
	en: {
		cards: {
			systemStatus: "System Status",
			profiles: "Profiles",
			servers: "Servers",
			clients: "Clients",
			metrics: "Metrics",
		},
		labels: {
			status: "Status",
			uptime: "Uptime",
			version: "Version",
			totalProfiles: "Total Profiles",
			activeProfiles: "Active Profiles",
			totalServers: "Total Servers",
			connected: "Connected",
			totalClients: "Total Clients",
			managed: "Managed",
				approved: "Approved",
		},
		metrics: {
			title: "Metrics",
			description:
				"MCPMate process CPU and memory utilization sampled every 30 seconds",
			noData: "No metrics have been reported yet.",
			waitingFirstSample:
				"Leave this page open; the chart fills as samples arrive (about every 30 seconds).",
			mcpmateCpu: "CPU (%)",
			mcpmateMemory: "Memory (%)",
		},
		core: {
			title: "Local Core",
			modeService: "Service",
			modeDesktopManaged: "Desktop",
			localServiceStatus: {
				not_installed: "Not installed",
				stopped: "Stopped",
				running: "Running",
				running_unhealthy: "Running (unhealthy)",
			},
			localServiceDetail: {
				desktop_managed: {
					not_installed: "MCPMate Desktop will start the local core when needed.",
					stopped:
						"The local core is stopped. Starting it keeps it alive while MCPMate Desktop is running.",
					running:
						"MCPMate Desktop is managing the local core and will stop it when the app quits.",
					running_unhealthy:
						"MCPMate Desktop started the local core, but health checks are failing.",
				},
				service: {
					not_installed: "The local core service has not been installed yet.",
					stopped:
						"The local core service is installed but not running.",
					running:
						"The local core service is running and responding to health checks.",
					running_unhealthy:
						"The service manager reports the local core as running, but health checks are failing.",
				},
			},
			localServiceDetailFallback:
				"The configured local core status will appear here.",
			statusAction: "Status",
			startAction: "Start",
			restartAction: "Restart",
			stopAction: "Stop",
		},
		tokenSavings: {
			title: "Token Savings",
			description: "Estimated context savings from profile filtering",
			infoLabel: "How token savings are estimated",
			infoLine1:
				"Current values are recalculated from active profiles using tokenizer-based capability payloads.",
			infoLine2:
				"Each successful MCP list or call event in activity logs is matched to its profile and contributes that profile's current savings.",
			infoLine3:
				"This is not a frozen historical ledger yet: when profile configuration changes, earlier totals can be recomputed.",
			infoLine4:
				"That keeps the logic dynamic and closer to real usage, while finer time-slice reconstruction is still being improved.",
			beforeFiltering: "Before Filtering",
			afterFiltering: "After Filtering",
			collectingData: "Collecting data...",
			collectingDataHint:
				"Estimates appear once servers and profiles finish loading.",
			savedPercent: "Saved",
			activeProfiles: "Active",
			savedPerCall: "Saved per call",
			enabled: "Enabled",
			saved: "saved",
			noData: "No data",
			calls: "Calls",
			emptyOrg:
				"Add a server or profile to estimate token savings from capability filtering.",
		},
	},
	"zh-CN": {
		cards: {
			systemStatus: "系统状态",
			profiles: "配置集",
			servers: "服务器",
			clients: "客户端",
			metrics: "指标",
		},
		labels: {
			status: "状态",
			uptime: "运行时间",
			version: "内核版本",
			totalProfiles: "总配置集",
			activeProfiles: "已激活",
			totalServers: "总服务器",
			connected: "已连接",
			totalClients: "总客户端",
			managed: "被托管",
				approved: "已允许",
		},
		metrics: {
			title: "资源指标",
			description: "MCPMate 进程 CPU 与内存占用，每 30 秒采样一次",
			noData: "尚未报告任何指标数据。",
			waitingFirstSample:
				"请保持本页打开；图表会随采样逐步填充（约每 30 秒一次）。",
			mcpmateCpu: "CPU (%)",
			mcpmateMemory: "内存 (%)",
		},
		core: {
			title: "本地 Core",
			modeService: "服务",
			modeDesktopManaged: "桌面",
			localServiceStatus: {
				not_installed: "未安装",
				stopped: "已停止",
				running: "运行中",
				running_unhealthy: "运行中（异常）",
			},
			localServiceDetail: {
				desktop_managed: {
					not_installed: "MCPMate Desktop 会在需要时启动本地 Core。",
					stopped:
						"本地 Core 当前已停止；启动后会在 MCPMate Desktop 运行期间保持可用。",
					running:
						"本地 Core 由 MCPMate Desktop 管理，并会在应用退出时停止。",
					running_unhealthy:
						"MCPMate Desktop 已启动本地 Core，但健康检查失败。",
				},
				service: {
					not_installed: "本地 Core 服务尚未安装。",
					stopped: "本地 Core 服务已安装，但当前未运行。",
					running: "本地 Core 服务正在运行，并且健康检查正常。",
					running_unhealthy:
						"系统服务管理器显示本地 Core 正在运行，但健康检查失败。",
				},
			},
			localServiceDetailFallback: "这里会显示已配置的本地 Core 状态。",
			statusAction: "状态",
			startAction: "启动",
			restartAction: "重启",
			stopAction: "停止",
		},
		tokenSavings: {
			title: "Token 节省",
			description: "通过配置集过滤节省的预估上下文空间",
			infoLabel: "查看 Token 节省估算说明",
			infoLine1: "当前数值会基于活跃 profile 的能力载荷，并用 tokenizer 重新计算。",
			infoLine2: "活动日志里每一次成功的 MCP list 或调用，都会关联到对应 profile 并累加该 profile 当前的节省值。",
			infoLine3: "这还不是冻结的历史账本：当 profile 配置变化时，之前的累计值也可能被重新计算。",
			infoLine4:
				"这样会更贴近真实使用，但更细粒度的时点重建仍在继续完善。",
			beforeFiltering: "过滤前",
			afterFiltering: "过滤后",
			collectingData: "数据收集中...",
			collectingDataHint: "服务器与配置集加载完成后即可显示估算。",
			savedPercent: "节省",
			activeProfiles: "已激活",
			savedPerCall: "每次节省",
			enabled: "已启用",
			saved: "节省",
			noData: "暂无数据",
			calls: "调用次数",
			emptyOrg: "添加服务器或配置集后，即可估算能力过滤带来的 Token 节省。",
		},
	},
	"ja-JP": {
		cards: {
			systemStatus: "システム状態",
			profiles: "プロファイル",
			servers: "サーバー",
			clients: "クライアント",
			metrics: "メトリクス",
		},
		labels: {
			status: "状態",
			uptime: "稼働時間",
			version: "カーネルバージョン",
			totalProfiles: "総プロファイル",
			activeProfiles: "アクティブ済み",
			totalServers: "総サーバー",
			connected: "接続済み",
			totalClients: "総クライアント",
			managed: "管理対象",
				approved: "許可済み",
		},
		metrics: {
			title: "リソースメトリクス",
			description:
				"MCPMate プロセスの CPU とメモリ使用率、30 秒ごとにサンプリング",
			noData: "まだメトリクスが報告されていません。",
			waitingFirstSample:
				"このページを開いたままにしてください。約 30 秒ごとにサンプルが溜まりチャートが表示されます。",
			mcpmateCpu: "CPU (%)",
			mcpmateMemory: "メモリ (%)",
		},
		core: {
			title: "Local Core",
			modeService: "Service",
			modeDesktopManaged: "Desktop",
			localServiceStatus: {
				not_installed: "未インストール",
				stopped: "停止中",
				running: "実行中",
				running_unhealthy: "実行中（異常）",
			},
			localServiceDetail: {
				desktop_managed: {
					not_installed:
						"MCPMate Desktop は必要に応じてローカル Core を起動します。",
					stopped:
						"ローカル Core は停止中です。起動すると MCPMate Desktop の実行中は維持されます。",
					running:
						"ローカル Core は MCPMate Desktop により管理され、アプリ終了時に停止します。",
					running_unhealthy:
						"MCPMate Desktop はローカル Core を起動しましたが、ヘルスチェックに失敗しています。",
				},
				service: {
					not_installed:
						"ローカル Core サービスはまだインストールされていません。",
					stopped:
						"ローカル Core サービスはインストール済みですが、現在は停止しています。",
					running:
						"ローカル Core サービスは実行中で、ヘルスチェックにも応答しています。",
					running_unhealthy:
						"サービスマネージャー上はローカル Core が実行中ですが、ヘルスチェックに失敗しています。",
				},
			},
			localServiceDetailFallback:
				"設定済みのローカル Core 状態がここに表示されます。",
			statusAction: "状態",
			startAction: "開始",
			restartAction: "再起動",
			stopAction: "停止",
		},
		tokenSavings: {
			title: "トークン節約",
			description: "プロファイルフィルタリングによる推定コンテキスト節約",
			infoLabel: "トークン節約の推定方法を見る",
			infoLine1:
				"現在の値は、アクティブな profile の capability payload を tokenizer ベースで再計算したものです。",
			infoLine2:
				"アクティビティログ内の成功した MCP list / call は対応する profile に紐づけられ、その profile の現在の節約量として積み上げられます。",
			infoLine3:
				"まだ固定化された履歴台帳ではないため、profile 設定が変わると過去分の累計も再計算されることがあります。",
			infoLine4:
				"その分、実利用には近づいていますが、より細かい時点再構成は引き続き改善中です。",
			beforeFiltering: "フィルタ前",
			afterFiltering: "フィルタ後",
			collectingData: "データ収集中...",
			collectingDataHint:
				"サーバーとプロファイルの読み込みが終わると推定値が表示されます。",
			savedPercent: "節約",
			activeProfiles: "アクティブ",
			savedPerCall: "1回あたりの節約",
			enabled: "有効",
			saved: "節約",
			noData: "データなし",
			calls: "呼び出し",
			emptyOrg:
				"サーバーまたはプロファイルを追加すると、機能フィルタによるトークン節約を推定できます。",
		},
	},
} as const;
