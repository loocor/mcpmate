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
      consent: "Allow scanning local runtimes and MCP server configurations",
      consentRequired: "Please accept the scanning authorization to continue",
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
        "We scanned every detected MCP client that has a local config file. Choose the servers you'd like MCPMate to import.",
      noScannableClients:
        "No detected MCP clients have a local configuration path to scan yet. You can skip this step or finish client setup first.",
      empty: "No importable MCP servers were found across your detected clients.",
      sources: "Found in",
      importErrorTitle: "Server import failed",
      official: "Official",
    },
    community: {
      title: "Join the Community",
      description:
        "Connect with other MCPMate users, get help, and stay up to date.",
      openExternalAria: "Open {{title}} in a new tab",
      discord: {
        title: "Discord",
        description:
          "Chat with the community, get support, and follow product updates.",
      },
      github: {
        title: "GitHub Issues",
        description: "Report bugs, request features, and browse open issues.",
      },
      discussions: {
        title: "GitHub Discussions",
        description:
          "Ask questions, share ideas, and discuss MCPMate with maintainers and users.",
      },
    },
    language: {
      select: "Language",
    },
    complete: {
      applyClientsErrorTitle: "Failed to apply MCP client configurations",
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
      consent: "允许扫描本地运行时和 MCP 服务器配置",
      consentRequired: "请先同意扫描授权后再继续",
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
      description:
        "已扫描本机所有已检测到且具备本地配置路径的 MCP 客户端，请选择希望 MCPMate 导入的服务器。",
      noScannableClients:
        "当前没有已检测且具备本地配置路径的客户端可供扫描。可跳过此步骤，或先完成某个客户端的配置。",
      empty: "在已检测的客户端中未发现可导入的 MCP 服务器。",
      sources: "来自",
      importErrorTitle: "服务器导入失败",
      official: "官方",
    },
    community: {
      title: "加入社区",
      description: "和 MCPMate 用户交流、获取帮助、了解最新动态。",
      openExternalAria: "在新标签页打开：{{title}}",
      discord: {
        title: "Discord",
        description: "与社区交流、获取支持并关注产品动态。",
      },
      github: {
        title: "GitHub Issues",
        description: "提交 Bug、功能请求，浏览公开问题。",
      },
      discussions: {
        title: "GitHub Discussions",
        description: "提问、分享想法，与维护者和其他用户讨论 MCPMate。",
      },
    },
    language: {
      select: "语言",
    },
    complete: {
      applyClientsErrorTitle: "应用 MCP 客户端配置失败",
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
      consent: "ローカルランタイムと MCP サーバー設定のスキャンを許可する",
      consentRequired: "スキャン許可に同意してから続行してください",
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
      description:
        "ローカル設定ファイルがある検出済み MCP クライアントをすべてスキャンしました。MCPMate にインポートするサーバーを選択してください。",
      noScannableClients:
        "スキャンできるローカル設定パスを持つ検出済みクライアントがありません。この手順をスキップするか、クライアントのセットアップを完了してください。",
      empty: "検出済みクライアントにインポート可能な MCP サーバーは見つかりませんでした。",
      sources: "検出元",
      importErrorTitle: "サーバーのインポートに失敗しました",
      official: "公式",
    },
    community: {
      title: "コミュニティに参加",
      description: "他の MCPMate ユーザーと交流し、ヘルプや最新情報を入手しましょう。",
      openExternalAria: "新しいタブで {{title}} を開く",
      discord: {
        title: "Discord",
        description: "コミュニティとチャットし、サポートや製品アップデートを入手。",
      },
      github: {
        title: "GitHub Issues",
        description: "バグ報告、機能リクエスト、公開 issue の閲覧。",
      },
      discussions: {
        title: "GitHub Discussions",
        description:
          "質問やアイデアを共有し、メンテナーやユーザーと MCPMate について話し合う。",
      },
    },
    language: {
      select: "言語",
    },
    complete: {
      applyClientsErrorTitle: "MCP クライアント設定の適用に失敗しました",
    },
  },
};
