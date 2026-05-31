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
		operator: {
			title: "Operator Panel",
			description: "High-frequency MCP control in one compact column.",
			fullConsole: "Full Console",
			status: {
				ready: "Ready",
				warning: "Review",
				error: "Attention",
				idle: "Idle",
			},
			actions: {
				openRuntime: "Runtime",
				manage: "Manage",
				review: "Review",
				install: "Install",
				openLogs: "Logs",
				inspect: "Inspect",
				discover: "Discover servers",
			},
			rows: {
				core: {
					title: "Core",
					ready: "MCPMate Core is ready",
					notReady: "Core needs attention",
					meta: "{{status}} · {{uptime}} uptime",
				},
				profiles: {
					title: "Profiles",
					summary: "{{active}} active of {{total}} profiles",
					noDefault: "No default profile selected",
				},
				clients: {
					title: "Clients",
					summary: "{{total}} clients connected",
					meta: "{{approved}} approved · {{pending}} pending",
					pending: "{{count}} pending",
				},
				servers: {
					title: "Servers",
					summary: "{{total}} servers installed",
					meta: "{{connected}} connected · {{attention}} needs attention",
					attention: "{{count}} needs attention",
				},
				traffic: {
					title: "Traffic",
					summary: "{{requests}} MCP requests",
					meta: "{{cpu}}% CPU · {{memory}} memory",
				},
				attention: {
					title: "Attention",
					summary: "{{count}} items need review",
					clear: "No urgent operator actions",
					noEvents: "No recent activity",
				},
			},
			detail: {
				focus: "Current focus",
				next: "Next actions",
				fullConsoleHint:
					"Use Full Console when you need raw capability data, detailed editors, or inspector workflows.",
				core: {
					ready: "Confirm local Core status before changing configuration.",
					runtime: "Open Runtime for service control, ports, and health checks.",
					logs: "Use Logs only when the compact status needs explanation.",
				},
				profiles: {
					active:
						"Keep the active profile visible without capability bulk controls.",
					filter: "Open Full Console for server and capability selection.",
					tokens:
						"Use token estimates to decide whether a profile is too broad.",
				},
				clients: {
					detect: "Detect local clients and approve first contact.",
					configure:
						"Keep config-file editing inside the full client console.",
					review:
						"Review pending clients before they receive profile access.",
				},
				servers: {
					import: "Install from Market, Uni-Import, or dropped configuration.",
					health: "Watch connection health before opening Inspector details.",
					profile:
						"Add successful installs to the active profile when needed.",
				},
				traffic: {
					runtime: "Check live usage without starting from raw logs.",
					tokens: "Use charts for trend review and Logs for exact events.",
					time: "Time-window controls stay in the full analytics surfaces.",
				},
				attention: {
					pending: "Pending clients and unhealthy servers rise to the top.",
					audit:
						"Open Logs when the compact row is not enough to explain an event.",
					resolve: "Resolve detailed failures in the owning full console page.",
				},
			},
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
		operator: {
			title: "操作面板",
			description: "把高频 MCP 控制压缩到一列里。",
			fullConsole: "完整控制台",
			status: {
				ready: "就绪",
				warning: "待检查",
				error: "需处理",
				idle: "空闲",
			},
			actions: {
				openRuntime: "运行时",
				manage: "管理",
				review: "查看",
				install: "安装",
				openLogs: "日志",
				inspect: "检查",
				discover: "发现服务",
			},
			rows: {
				core: {
					title: "Core",
					ready: "MCPMate Core 已就绪",
					notReady: "Core 需要处理",
					meta: "{{status}} · 已运行 {{uptime}}",
				},
				profiles: {
					title: "配置集",
					summary: "{{total}} 个配置集中 {{active}} 个已启用",
					noDefault: "未选择默认配置集",
				},
				clients: {
					title: "客户端",
					summary: "已连接 {{total}} 个客户端",
					meta: "{{approved}} 个已允许 · {{pending}} 个待处理",
					pending: "{{count}} 个待处理",
				},
				servers: {
					title: "服务器",
					summary: "已安装 {{total}} 个服务器",
					meta: "{{connected}} 个已连接 · {{attention}} 个需处理",
					attention: "{{count}} 个需处理",
				},
				traffic: {
					title: "流量",
					summary: "{{requests}} 次 MCP 请求",
					meta: "{{cpu}}% CPU · {{memory}} 内存",
				},
				attention: {
					title: "关注项",
					summary: "{{count}} 项需要查看",
					clear: "暂无紧急操作",
					noEvents: "暂无最近活动",
				},
			},
			detail: {
				focus: "当前关注点",
				next: "下一步操作",
				fullConsoleHint:
					"需要原始能力数据、详细编辑器或 Inspector 流程时，请使用完整控制台。",
				core: {
					ready: "调整配置前先确认本地 Core 状态。",
					runtime: "打开运行时页面管理服务、端口和健康状态。",
					logs: "只有当紧凑状态解释不够时再进入日志。",
				},
				profiles: {
					active: "保持活跃配置集可见，但不暴露能力批量控制。",
					filter: "服务和能力选择放在完整控制台中处理。",
					tokens: "通过 Token 估算判断配置集是否过宽。",
				},
				clients: {
					detect: "检测本地客户端，并审批首次连接。",
					configure: "配置文件编辑保留在完整客户端控制台里。",
					review: "待处理客户端审批后才获得配置集访问。",
				},
				servers: {
					import: "从服务源、Uni-Import 或拖拽配置安装。",
					health: "打开 Inspector 详情前先观察连接健康状态。",
					profile: "安装成功后按需加入活跃配置集。",
				},
				traffic: {
					runtime: "不用从原始日志开始，也能查看实时使用情况。",
					tokens: "用图表看趋势，用日志定位精确事件。",
					time: "时间窗口控制仍保留在完整分析界面。",
				},
				attention: {
					pending: "待审批客户端和异常服务器会优先浮现。",
					audit: "当紧凑条目解释不够时，打开日志查看。",
					resolve: "详细故障在对应的完整控制台页面处理。",
				},
			},
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
		operator: {
			title: "オペレーターパネル",
			description: "高頻度の MCP 操作を1列にまとめます。",
			fullConsole: "フルコンソール",
			status: {
				ready: "準備完了",
				warning: "確認",
				error: "対応必要",
				idle: "アイドル",
			},
			actions: {
				openRuntime: "ランタイム",
				manage: "管理",
				review: "確認",
				install: "インストール",
				openLogs: "ログ",
				inspect: "検査",
				discover: "サーバーを探す",
			},
			rows: {
				core: {
					title: "Core",
					ready: "MCPMate Core は準備完了です",
					notReady: "Core の確認が必要です",
					meta: "{{status}} · 稼働 {{uptime}}",
				},
				profiles: {
					title: "プロファイル",
					summary: "{{total}} 件中 {{active}} 件がアクティブ",
					noDefault: "デフォルトプロファイル未選択",
				},
				clients: {
					title: "クライアント",
					summary: "{{total}} クライアントが接続済み",
					meta: "{{approved}} 承認済み · {{pending}} 保留中",
					pending: "{{count}} 保留中",
				},
				servers: {
					title: "サーバー",
					summary: "{{total}} サーバーがインストール済み",
					meta: "{{connected}} 接続済み · {{attention}} 要確認",
					attention: "{{count}} 要確認",
				},
				traffic: {
					title: "トラフィック",
					summary: "{{requests}} MCP リクエスト",
					meta: "{{cpu}}% CPU · {{memory}} メモリ",
				},
				attention: {
					title: "注意",
					summary: "{{count}} 件の確認が必要",
					clear: "緊急の操作はありません",
					noEvents: "最近のアクティビティはありません",
				},
			},
			detail: {
				focus: "現在のフォーカス",
				next: "次の操作",
				fullConsoleHint:
					"生の capability データ、詳細エディタ、Inspector が必要な場合はフルコンソールを使います。",
				core: {
					ready: "設定変更の前にローカル Core の状態を確認します。",
					runtime: "サービス、ポート、ヘルスチェックはランタイムで管理します。",
					logs: "コンパクト状態だけで説明できない場合にログを開きます。",
				},
				profiles: {
					active:
						"アクティブなプロファイルを表示し、capability の一括操作は隠します。",
					filter:
						"サーバーと capability の選択はフルコンソールで行います。",
					tokens:
						"トークン推定でプロファイルが広すぎないか判断します。",
				},
				clients: {
					detect: "ローカルクライアントを検出し、初回接続を承認します。",
					configure:
						"設定ファイル編集はフルクライアントコンソール内に残します。",
					review:
						"保留中のクライアントは profile access の前に確認します。",
				},
				servers: {
					import:
						"Market、Uni-Import、またはドロップした設定からインストールします。",
					health: "Inspector 詳細の前に接続状態を確認します。",
					profile:
						"インストール成功後、必要に応じてアクティブプロファイルへ追加します。",
				},
				traffic: {
					runtime: "生ログから始めずにライブ使用状況を確認します。",
					tokens: "傾向はチャート、正確なイベントはログで確認します。",
					time: "時間範囲の操作はフル分析画面に残します。",
				},
				attention: {
					pending: "保留クライアントと異常サーバーを優先表示します。",
					audit:
						"コンパクト行だけでは不足する場合、ログを開いて確認します。",
					resolve: "詳細な障害は対応するフルコンソール画面で解決します。",
				},
			},
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
