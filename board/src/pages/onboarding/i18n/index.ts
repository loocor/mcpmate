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
          "We'll check which runtimes on your system are usable by MCPMate.",
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
        replaceWithManaged: "Replace with managed",
        ready: "Ready",
        installed: "Installed",
        openOfficialSite: "Open official website",
        successTitle: "Install complete",
        successDescription: "{{runtime}} installation finished.",
        errorTitle: "Install failed",
        required: "Install runtimes to continue",
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
      title: "Set Up MCP Clients",
      description:
        "Choose clients to manage now or pre-select popular ones to set up after installation. You can change this anytime from the Clients page.",
      tabs: {
        detected: "From your device",
        popular: "Popular Clients",
      },
      rescan: "Rescan",
      refresh: "Refresh",
      error: "Failed to detect MCP clients. Please retry.",
      retry: "Retry detection",
      recommendationError:
        "MCPMate could not load the popular client catalog. You can continue setup and add clients manually later.",
      recommendationNotice:
        "Pre-selected clients stay pending until installed. After installation, return here to rescan or finish binding from the Clients page.",
      recommendationPartialWarning:
        "{{count}} client presets were skipped because their discovery data is invalid.",
      presetMissing: "Client preset '{{identifier}}' was not found.",
      selectedAria: "{{name}} pre-selected for setup",
      unselectedAria: "{{name}} not pre-selected",
      installAria: "Open {{name}} official site to install",
      installTooltip: "Open the official site to download and install",
      empty:
        "No MCP clients detected on this device yet.",
      emptyAction: "Browse popular clients",
      detectedNotice:
        "We scan your device automatically. Some clients may not appear due to version or compatibility—you can add them manually later.",
      emptyFiltered: "No clients match this category.",
      badges: {
        detected: "Detected",
        installable: "Installable",
      },
      tags: {
        all: "All",
        editor: "Editor",
        agent: "Agent",
        application: "Application",
        cli: "CLI",
        desktop: "Desktop",
        browser: "Browser",
      },
    },
    servers: {
      title: "Set Up MCP Servers",
      description:
        "Import servers found in local client configs or add MCPMate presets directly. You can change this anytime from the Servers page.",
      tabs: {
        local: "From your device",
        popular: "Popular Servers",
      },
      rescan: "Rescan",
      refresh: "Refresh",
      noScannableClients:
        "No detected MCP clients have a local configuration path to scan yet. Try the Popular Servers tab or finish client setup first.",
      empty: "No importable MCP servers were found in your local client configs.",
      emptyFiltered: "No servers match this category.",
      emptyAction: "Browse popular servers",
      sources: "Found in",
      localNotice:
        "We scan your device automatically. Some servers may not appear due to version or compatibility—you can add them manually later.",
      recommendationError: "MCPMate could not load preset server data. You can continue setup and add servers manually later.",
      recommendationNotice:
        "These presets can be imported directly without an existing local client config.",
      importErrorTitle: "Server import failed",
      official: "Official",
      selectedAria: "{{name}} server selected",
      unselectedAria: "{{name}} server not selected",
      tags: {
        all: "All",
        memory: "Memory",
        "developer-tools": "Developer Tools",
        browser: "Browser",
        documentation: "Documentation",
        database: "Database",
        design: "Design",
        automation: "Automation",
        filesystem: "Filesystem",
        debugging: "Debugging",
        knowledge: "Knowledge",
        "3d": "3D",
        content: "Content",
        "creative-tools": "Creative Tools",
        frontend: "Frontend",
        web: "Web",
      },
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
      feishu: {
        title: "Feishu Community",
        description:
          "Join the Chinese user community for support, tips, and product updates.",
      },
      discordFallback: "International users can join our",
      discordFallbackSuffix: " community.",
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
      operatorPanelErrorTitle: "Could not open tray operator panel",
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
          "检测你的系统中哪些运行时可以被 MCPMate 正常使用。",
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
        replaceWithManaged: "替换为托管版本",
        ready: "已就位",
        installed: "已安装",
        openOfficialSite: "打开官网",
        successTitle: "安装完成",
        successDescription: "{{runtime}} 安装完成。",
        errorTitle: "安装失败",
        required: "请先完成运行时安装",
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
      title: "设置 MCP 客户端",
      description:
        "现在选择要管理的客户端，或预选常用客户端并在安装后继续配置。之后可随时在「客户端」页面重新绑定。",
      tabs: {
        detected: "本机客户端",
        popular: "常用客户端",
      },
      rescan: "重新扫描",
      refresh: "刷新",
      error: "检测 MCP 客户端失败，请重试。",
      retry: "重试检测",
      recommendationError: "MCPMate 无法加载常用客户端目录。你可以继续完成设置，之后再手动添加客户端。",
      recommendationNotice:
        "预选的客户端在安装完成前不会写入配置。安装后可返回此页重新扫描，或在「客户端」页面完成绑定。",
      recommendationPartialWarning: "{{count}} 条客户端预置因发现数据无效而被跳过。",
      presetMissing: "未找到客户端预置「{{identifier}}」。",
      selectedAria: "已预选 {{name}}",
      unselectedAria: "未预选 {{name}}",
      installAria: "打开 {{name}} 官网进行安装",
      installTooltip: "点击打开官网下载安装",
      empty: "暂未在本机检测到 MCP 客户端。",
      emptyAction: "浏览常用客户端",
      detectedNotice:
        "已自动扫描本机。因版本或兼容原因，部分客户端可能未被识别，可稍后手动添加。",
      emptyFiltered: "当前分类下没有匹配的客户端。",
      badges: {
        detected: "已检测",
        installable: "可安装",
      },
      tags: {
        all: "全部",
        editor: "编辑器",
        agent: "Agent",
        application: "应用",
        cli: "CLI",
        desktop: "桌面",
        browser: "浏览器",
      },
    },
    servers: {
      title: "设置 MCP 服务器",
      description:
        "导入本机客户端配置中的服务器，或直接添加 MCPMate 预置。之后可随时在「服务器」页面调整。",
      tabs: {
        local: "本机服务器",
        popular: "常用服务器",
      },
      rescan: "重新扫描",
      refresh: "刷新",
      noScannableClients:
        "当前没有已检测且具备本地配置路径的客户端可供扫描。可切换到「常用服务器」，或先完成客户端设置。",
      empty: "在本机客户端配置中未发现可导入的 MCP 服务器。",
      emptyFiltered: "当前分类下没有匹配的服务器。",
      emptyAction: "浏览常用服务器",
      sources: "来自",
      localNotice:
        "已自动扫描本机。因版本或兼容原因，部分服务器可能未被识别，可稍后手动添加。",
      recommendationError: "MCPMate 无法加载服务器预置数据。你可以继续完成设置，之后再手动添加服务器。",
      recommendationNotice: "这些预置可直接导入，无需本机已有客户端配置。",
      importErrorTitle: "服务器导入失败",
      official: "官方",
      selectedAria: "服务器 {{name}} 已选中",
      unselectedAria: "服务器 {{name}} 未选中",
      tags: {
        all: "全部",
        memory: "记忆",
        "developer-tools": "开发者工具",
        browser: "浏览器",
        documentation: "文档",
        database: "数据库",
        design: "设计",
        automation: "自动化",
        filesystem: "文件系统",
        debugging: "调试",
        knowledge: "知识",
        "3d": "3D",
        content: "内容",
        "creative-tools": "创意工具",
        frontend: "前端",
        web: "Web",
      },
    },
    community: {
      title: "加入社区",
      description: "和 MCPMate 用户交流、获取帮助、了解最新动态。",
      openExternalAria: "在新标签页打开：{{title}}",
      discord: {
        title: "Discord",
        description: "与社区交流、获取支持并关注产品动态。",
      },
      feishu: {
        title: "飞书社群",
        description: "加入中文用户社群，交流使用经验、获取支持并关注产品动态。",
      },
      discordFallback: "国际用户可加入",
      discordFallbackSuffix: " 社区。",
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
      operatorPanelErrorTitle: "无法打开托盘操作面板",
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
        replaceWithManaged: "管理版に置き換え",
        ready: "準備完了",
        installed: "インストール済み",
        openOfficialSite: "公式サイトを開く",
        successTitle: "インストール完了",
        successDescription: "{{runtime}} のインストールが完了しました。",
        errorTitle: "インストール失敗",
        required: "ランタイムをインストールして続行",
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
      title: "MCP クライアントを設定",
      description:
        "今すぐ管理するクライアントを選ぶか、人気クライアントを事前選択してインストール後に設定できます。後から「クライアント」ページでいつでも変更できます。",
      tabs: {
        detected: "ローカル",
        popular: "人気クライアント",
      },
      rescan: "再スキャン",
      refresh: "更新",
      error: "MCP クライアントの検出に失敗しました。再試行してください。",
      retry: "検出を再試行",
      recommendationError:
        "MCPMate は人気クライアントカタログを読み込めませんでした。セットアップを続行し、あとで手動でクライアントを追加できます。",
      recommendationNotice:
        "事前選択したクライアントはインストール完了まで保留状態です。インストール後はここで再スキャンするか、「クライアント」ページでバインドを完了してください。",
      recommendationPartialWarning:
        "検出データが無効なため、{{count}} 件のクライアントプリセットをスキップしました。",
      presetMissing: "クライアントプリセット「{{identifier}}」が見つかりません。",
      selectedAria: "{{name}} は事前選択済みです",
      unselectedAria: "{{name}} は未選択です",
      installAria: "{{name}} の公式サイトを開いてインストール",
      installTooltip: "公式サイトを開いてダウンロード・インストール",
      empty: "このデバイスではまだ MCP クライアントが検出されていません。",
      emptyAction: "人気クライアントを見る",
      detectedNotice:
        "デバイスを自動スキャンしました。バージョンや互換性により検出できないクライアントがある場合は、後から手動で追加できます。",
      emptyFiltered: "このカテゴリに一致するクライアントはありません。",
      badges: {
        detected: "検出済み",
        installable: "インストール可",
      },
      tags: {
        all: "すべて",
        editor: "エディタ",
        agent: "エージェント",
        application: "アプリ",
        cli: "CLI",
        desktop: "デスクトップ",
        browser: "ブラウザ",
      },
    },
    servers: {
      title: "MCP サーバーを設定",
      description:
        "ローカルクライアント設定からサーバーをインポートするか、MCPMate プリセットを直接追加できます。後から「サーバー」ページでいつでも変更できます。",
      tabs: {
        local: "ローカル",
        popular: "人気サーバー",
      },
      rescan: "再スキャン",
      refresh: "更新",
      noScannableClients:
        "スキャンできるローカル設定パスを持つ検出済みクライアントがありません。「人気サーバー」タブを試すか、クライアント設定を先に完了してください。",
      empty: "ローカルクライアント設定にインポート可能な MCP サーバーは見つかりませんでした。",
      emptyFiltered: "このカテゴリに一致するサーバーはありません。",
      emptyAction: "人気サーバーを見る",
      sources: "検出元",
      localNotice:
        "デバイスを自動スキャンしました。バージョンや互換性により検出できないサーバーがある場合は、後から手動で追加できます。",
      recommendationError: "MCPMate はサーバープリセットを読み込めませんでした。セットアップを続行し、あとで手動でサーバーを追加できます。",
      recommendationNotice:
        "これらのプリセットは、既存のローカルクライアント設定がなくても直接インポートできます。",
      importErrorTitle: "サーバーのインポートに失敗しました",
      official: "公式",
      selectedAria: "サーバー {{name}} は選択済みです",
      unselectedAria: "サーバー {{name}} は未選択です",
      tags: {
        all: "すべて",
        memory: "メモリ",
        "developer-tools": "開発者ツール",
        browser: "ブラウザ",
        documentation: "ドキュメント",
        database: "データベース",
        design: "デザイン",
        automation: "自動化",
        filesystem: "ファイルシステム",
        debugging: "デバッグ",
        knowledge: "ナレッジ",
        "3d": "3D",
        content: "コンテンツ",
        "creative-tools": "クリエイティブツール",
        frontend: "フロントエンド",
        web: "Web",
      },
    },
    community: {
      title: "コミュニティに参加",
      description: "他の MCPMate ユーザーと交流し、ヘルプや最新情報を入手しましょう。",
      openExternalAria: "新しいタブで {{title}} を開く",
      discord: {
        title: "Discord",
        description: "コミュニティとチャットし、サポートや製品アップデートを入手。",
      },
      feishu: {
        title: "Feishu コミュニティ",
        description: "中国語ユーザー向けコミュニティで、サポートや最新情報を入手。",
      },
      discordFallback: "海外ユーザーは",
      discordFallbackSuffix: " コミュニティに参加できます。",
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
      operatorPanelErrorTitle: "トレイ Operator Panel を開けませんでした",
    },
  },
};
