export const onboardingTranslations = {
	en: {
		steps: {
			welcome: "Welcome",
			runtime: "Runtime",
			clients: "Clients",
			servers: "Servers",
			community: "Community",
		},
		nav: {
			back: "Back",
			skip: "Skip",
			next: "Next",
			finish: "Finish Setup",
			finishing: "Finishing…",
		},
		welcome: {
			title: "Welcome to MCPMate",
			description:
				"Let's get you set up in a few quick steps. We'll check your environment, detect clients, and add some useful servers.",
			getStarted: "Get Started",
			chooseLanguage: "Choose your language to continue",
		},
		runtime: {
			title: "Check Your Environment",
			description:
				"MCP servers need a JavaScript or Python runtime. We'll check what's available on your system.",
			allGood: "All required runtimes detected. You're ready to go!",
			install: {
				nodeTitle: "Node.js",
				nodeDescription: "JavaScript runtime for npm-based MCP servers.",
				bunTitle: "Bun",
				bunDescription: "Fast JavaScript runtime and package manager.",
				uvTitle: "uv",
				uvDescription: "Python runtime manager for Python-based MCP servers.",
				clickToInstall: "Click to install",
				installing: "Installing...",
				ready: "Ready",
				installed: "Installed",
				openOfficialSite: "Open official website",
				successTitle: "Install complete",
				successDescription: "{{runtime}} installation finished.",
				errorTitle: "Install failed",
			},
			noJs: {
				title: "No JavaScript runtime found.",
				description:
					"Install Node.js (https://nodejs.org) or Bun (https://bun.sh) to run npm-based MCP servers.",
			},
			noPython: {
				title: "No Python runtime found.",
				description:
					"Install Python 3 (https://python.org) or uv (https://docs.astral.sh/uv) to run Python-based MCP servers.",
			},
		},
		clients: {
			title: "Detected MCP Clients",
			description:
				"We found these MCP clients on your system. Select the ones you'd like MCPMate to manage.",
			error: "Failed to detect MCP clients. Please retry.",
			retry: "Retry detection",
			empty:
				"No MCP clients detected on this system. You can add clients manually later from the Clients page.",
		},
		servers: {
			title: "Import Existing Servers",
			description:
				"We scanned the MCP clients you selected. Choose the servers you'd like MCPMate to import.",
			selectClientsFirst:
				"Select at least one detected client first, or skip this step.",
			empty: "No importable MCP servers were found in the selected clients.",
			sources: "Found in",
			importErrorTitle: "Server import failed",
			official: "Official",
		},
		community: {
			title: "Join the Community",
			description:
				"Connect with other MCPMate users, get help, and stay up to date.",
			github: {
				title: "GitHub Issues",
				description: "Report bugs, request features, and browse open issues.",
			},
			docs: {
				title: "Documentation",
				description: "Guides, tutorials, and API references.",
			},
			chrome: {
				title: "Chrome Extension",
				description: "Detect and import MCP server snippets from web pages.",
			},
			edge: {
				title: "Edge Extension",
				description: "Import MCP server configurations directly from Edge.",
			},
		},
		language: {
			select: "Language",
		},
	},
	"zh-CN": {
		steps: {
			welcome: "欢迎",
			runtime: "环境",
			clients: "客户端",
			servers: "服务器",
			community: "社区",
		},
		nav: {
			back: "返回",
			skip: "跳过",
			next: "下一步",
			finish: "完成设置",
			finishing: "正在完成…",
		},
		welcome: {
			title: "欢迎使用 MCPMate",
			description:
				"接下来几步帮你快速完成设置：检查运行环境、发现已有客户端，并添加常用 MCP 服务器。",
			getStarted: "开始设置",
			chooseLanguage: "请选择语言后继续",
		},
		runtime: {
			title: "检查运行环境",
			description:
				"MCP 服务器需要 JavaScript 或 Python 运行时。先看看你的系统里有哪些可用。",
			allGood: "已检测到所有必要运行时，可以直接继续。",
			install: {
				nodeTitle: "Node.js",
				nodeDescription: "用于 npm 类 MCP 服务器的 JavaScript 运行时。",
				bunTitle: "Bun",
				bunDescription: "快速的 JavaScript 运行时与包管理器。",
				uvTitle: "uv",
				uvDescription: "用于 Python 类 MCP 服务器的 Python 运行时管理器。",
				clickToInstall: "点击安装",
				installing: "安装中...",
				ready: "已就位",
				installed: "已安装",
				openOfficialSite: "打开官网",
				successTitle: "安装完成",
				successDescription: "{{runtime}} 安装完成。",
				errorTitle: "安装失败",
			},
			noJs: {
				title: "未检测到 JavaScript 运行时。",
				description:
					"请安装 Node.js（https://nodejs.org）或 Bun（https://bun.sh）以运行基于 npm 的 MCP 服务器。",
			},
			noPython: {
				title: "未检测到 Python 运行时。",
				description:
					"请安装 Python 3（https://python.org）或 uv（https://docs.astral.sh/uv）以运行基于 Python 的 MCP 服务器。",
			},
		},
		clients: {
			title: "已检测到的 MCP 客户端",
			description: "系统中发现了这些 MCP 客户端，选择你希望 MCPMate 管理的客户端。",
			error: "检测 MCP 客户端失败，请重试。",
			retry: "重试检测",
			empty: "未在本机检测到 MCP 客户端，后续可在「客户端」页面手动添加。",
		},
		servers: {
			title: "导入已有服务器",
			description: "已扫描你选择的 MCP 客户端，选择希望 MCPMate 导入的服务器。",
			selectClientsFirst: "请先选择至少一个已检测到的客户端，或跳过此步骤。",
			empty: "所选客户端中未发现可导入的 MCP 服务器。",
			sources: "来自",
			importErrorTitle: "服务器导入失败",
			official: "官方",
		},
		community: {
			title: "加入社区",
			description: "和 MCPMate 用户交流、获取帮助、了解最新动态。",
			github: {
				title: "GitHub Issues",
				description: "提交 Bug、功能请求，浏览公开问题。",
			},
			docs: {
				title: "文档中心",
				description: "入门指南、教程和 API 参考。",
			},
			chrome: {
				title: "Chrome 扩展",
				description: "从网页中检测并导入 MCP 服务器配置片段。",
			},
			edge: {
				title: "Edge 扩展",
				description: "从 Edge 浏览器直接导入 MCP 服务器配置。",
			},
		},
		language: {
			select: "语言",
		},
	},
	"ja-JP": {
		steps: {
			welcome: "ようこそ",
			runtime: "環境",
			clients: "クライアント",
			servers: "サーバー",
			community: "コミュニティ",
		},
		nav: {
			back: "戻る",
			skip: "スキップ",
			next: "次へ",
			finish: "セットアップ完了",
			finishing: "完了中…",
		},
		welcome: {
			title: "MCPMate へようこそ",
			description:
				"数ステップでセットアップを完了します。環境を確認し、クライアントを検出し、便利なサーバーを追加しましょう。",
			getStarted: "セットアップ開始",
			chooseLanguage: "続行する言語を選択してください",
		},
		runtime: {
			title: "環境チェック",
			description:
				"MCP サーバーには JavaScript または Python ランタイムが必要です。システムにあるランタイムを確認します。",
			allGood: "必要なランタイムがすべて検出されました。",
			install: {
				nodeTitle: "Node.js",
				nodeDescription: "npm ベースの MCP サーバー向け JavaScript ランタイム。",
				bunTitle: "Bun",
				bunDescription: "高速な JavaScript ランタイム兼パッケージマネージャー。",
				uvTitle: "uv",
				uvDescription: "Python ベースの MCP サーバー向けランタイムマネージャー。",
				clickToInstall: "クリックしてインストール",
				installing: "インストール中...",
				ready: "準備完了",
				installed: "インストール済み",
				openOfficialSite: "公式サイトを開く",
				successTitle: "インストール完了",
				successDescription: "{{runtime}} のインストールが完了しました。",
				errorTitle: "インストール失敗",
			},
			noJs: {
				title: "JavaScript ランタイムが見つかりません。",
				description:
					"Node.js（https://nodejs.org）または Bun（https://bun.sh）をインストールしてください。",
			},
			noPython: {
				title: "Python ランタイムが見つかりません。",
				description:
					"Python 3（https://python.org）または uv（https://docs.astral.sh/uv）をインストールしてください。",
			},
		},
		clients: {
			title: "検出された MCP クライアント",
			description: "このシステムで検出された MCP クライアントです。管理したいものを選択してください。",
			error: "MCP クライアントの検出に失敗しました。再試行してください。",
			retry: "検出を再試行",
			empty: "MCP クライアントが検出されませんでした。後で「クライアント」ページから手動追加できます。",
		},
		servers: {
			title: "既存サーバーをインポート",
			description: "選択した MCP クライアントをスキャンしました。MCPMate にインポートするサーバーを選択してください。",
			selectClientsFirst: "先に検出済みクライアントを 1 つ以上選択するか、この手順をスキップしてください。",
			empty: "選択したクライアントにインポート可能な MCP サーバーは見つかりませんでした。",
			sources: "検出元",
			importErrorTitle: "サーバーのインポートに失敗しました",
			official: "公式",
		},
		community: {
			title: "コミュニティに参加",
			description: "他の MCPMate ユーザーと交流し、ヘルプや最新情報を入手しましょう。",
			github: {
				title: "GitHub Issues",
				description: "バグ報告、機能リクエスト、公開 issue の閲覧。",
			},
			docs: {
				title: "ドキュメント",
				description: "ガイド、チュートリアル、API リファレンス。",
			},
			chrome: {
				title: "Chrome 拡張",
				description: "Web ページから MCP サーバー設定を検出してインポート。",
			},
			edge: {
				title: "Edge 拡張",
				description: "Edge ブラウザから MCP サーバー設定を直接インポート。",
			},
		},
		language: {
			select: "言語",
		},
	},
};
