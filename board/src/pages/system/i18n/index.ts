export const systemTranslations = {
	en: {
		title: "System",
		actions: {
			refresh: "Refresh",
		},
		status: {
			title: "Status",
		},
		cpu: {
			title: "CPU",
			description: "Current CPU Usage",
		},
		memory: {
			title: "Memory",
			description: "Current Memory Usage",
		},
		uptime: {
			title: "Uptime",
			description: "System Running Time",
		},
		resourceChart: {
			title: "CPU & Memory Usage",
			description: "System resource utilization over time",
			timeAxis: "Time (minutes)",
			usageAxis: "Usage %",
			cpuSeries: "CPU",
			memorySeries: "Memory",
		},
		networkChart: {
			title: "Network Traffic",
			description: "Inbound and outbound network traffic",
			timeAxis: "Time (minutes)",
			throughputAxis: "KB/s",
			inboundSeries: "Inbound",
			outboundSeries: "Outbound",
		},
		apiChart: {
			title: "API Requests",
			description: "Request volume and error rate",
			hourAxis: "Hour",
			countAxis: "Count",
			requestsSeries: "Requests",
			errorsSeries: "Errors",
		},
		connections: {
			title: "Active Connections",
			description: "Current active connections to the MCPMate Proxy",
			valueLabel: "Active connections",
		},
	},
	"zh-CN": {
		title: "系统",
		actions: {
			refresh: "刷新",
		},
		status: {
			title: "状态",
		},
		cpu: {
			title: "CPU",
			description: "当前 CPU 使用率",
		},
		memory: {
			title: "内存",
			description: "当前内存使用量",
		},
		uptime: {
			title: "运行时间",
			description: "系统运行时长",
		},
		resourceChart: {
			title: "CPU 与内存使用情况",
			description: "系统资源使用趋势",
			timeAxis: "时间（分钟）",
			usageAxis: "使用率 %",
			cpuSeries: "CPU",
			memorySeries: "内存",
		},
		networkChart: {
			title: "网络流量",
			description: "入站与出站网络流量",
			timeAxis: "时间（分钟）",
			throughputAxis: "KB/s",
			inboundSeries: "入站",
			outboundSeries: "出站",
		},
		apiChart: {
			title: "API 请求",
			description: "请求量与错误率",
			hourAxis: "小时",
			countAxis: "数量",
			requestsSeries: "请求",
			errorsSeries: "错误",
		},
		connections: {
			title: "活动连接",
			description: "当前连接到 MCPMate Proxy 的活动连接数",
			valueLabel: "活动连接",
		},
	},
	"ja-JP": {
		title: "システム",
		actions: {
			refresh: "再読み込み",
		},
		status: {
			title: "状態",
		},
		cpu: {
			title: "CPU",
			description: "現在の CPU 使用率",
		},
		memory: {
			title: "メモリ",
			description: "現在のメモリ使用量",
		},
		uptime: {
			title: "稼働時間",
			description: "システム稼働時間",
		},
		resourceChart: {
			title: "CPU とメモリ使用量",
			description: "システムリソース使用率の推移",
			timeAxis: "時間（分）",
			usageAxis: "使用率 %",
			cpuSeries: "CPU",
			memorySeries: "メモリ",
		},
		networkChart: {
			title: "ネットワークトラフィック",
			description: "受信および送信ネットワークトラフィック",
			timeAxis: "時間（分）",
			throughputAxis: "KB/s",
			inboundSeries: "受信",
			outboundSeries: "送信",
		},
		apiChart: {
			title: "API リクエスト",
			description: "リクエスト数とエラー率",
			hourAxis: "時刻",
			countAxis: "件数",
			requestsSeries: "リクエスト",
			errorsSeries: "エラー",
		},
		connections: {
			title: "アクティブ接続",
			description: "現在 MCPMate Proxy に接続している数",
			valueLabel: "アクティブ接続",
		},
	},
} as const;
