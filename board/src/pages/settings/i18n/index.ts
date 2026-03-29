export const settingsTranslations = {
  en: {
    title: "Tune dashboard preferences and defaults",
    tabs: {
      general: "General",
      appearance: "Appearance",
      audit: "Audit",
      profile: "Profile",
      system: "System",
      serverControls: "Server",
      clientDefaults: "Client",
      market: "Market",
      developer: "Developer",
      about: "About",
    },
    system: {
      title: "System",
      description:
        "Manage how MCPMate Desktop connects to and controls its service, including runtime mode and local ports.",
      sourceTitle: "Service Target",
      sourceDescription:
        "Choose whether Desktop should attach to the built-in local service or a remote service endpoint.",
      sourceOptions: {
        localhost: "Local",
        remote: "Remote",
      },
      runtimeModeTitle: "Local Runtime Mode",
      runtimeModeDescription:
        "Choose whether the local service runs as an OS service or follows the MCPMate desktop lifecycle.",
      runtimeModeCurrent: "Current runtime mode: {{value}}",
      moreToggle: "More",
      lessToggle: "Less",
      runtimeModeOptions: {
        service: "Service",
        desktopManaged: "Desktop",
      },
      remoteUrlTitle: "Remote Core URL",
      remoteUrlDescription:
        "Store the remote core endpoint for future attach support. This phase still prioritizes localhost service management.",
      remoteUrlPlaceholder: "https://your-core.example.com",
      apiPortTitle: "Local Service API Port",
      apiPortDescription:
        "Port for local REST and dashboard access (default 8080).",
      mcpPortTitle: "Local Service MCP Port",
      mcpPortDescription:
        "Port for the local MCP proxy endpoint (/mcp). Default 8000.",
      apply: "Apply Core Source",
      helperTauri:
        "Tauri: Save the selected core source, manage the localhost core through OS service commands, and reconnect on next launch.",
      helperWeb:
        "Web: Change ports then restart the backend process externally.",
      webDialogTitle: "Apply & Restart (Web)",
      webDialogDesc:
        "The browser cannot restart the backend. Use one of the commands below with the selected ports.",
      optionCargoTitle: "Option A — cargo run (dev)",
      optionBinaryTitle: "Option B — binary (release)",
      copy: "Copy",
      stopCurrent: "Stop current backend",
      close: "Close",
      applyButtonBusy: "Applying…",
      applyProgressHint:
        "Updating the selected core source. API requests may fail briefly while the desktop reconnects.",
      applySuccessTitle: "Core source updated",
      applySuccessDescription:
        "Desktop is now attached to the selected {{source}} core. Existing service definitions were refreshed if needed.",
      applyFailedTitle: "Could not update core source",
      serviceStatusTitle: "Local Service Status",
      serviceStatusFallback:
        "Desktop will attach when the configured local service becomes available.",
      serviceLevel: "Service level: {{value}}",
      statusAction: "Status",
      startAction: "Start",
      restartAction: "Restart",
      stopAction: "Stop",
      installAction: "Install",
      uninstallAction: "Uninstall",
      serviceActionBusy: "Working…",
      serviceActionSuccessTitle: "Local core service updated",
      serviceActionSuccessDescription:
        "Local core service action {{action}} completed. Current status: {{status}}.",
      serviceActionFailedTitle: "Could not manage local core service",
      portsReloadFailedTitle: "Could not load ports from shell",
      portsReloadFailedDescription:
        "Showing cached values if any. Check the desktop app is healthy and try Reload again.",
      localhostRunning:
        "The local service is running. In Desktop mode it stops only when MCPMate truly quits.",
      localhostStopped:
        "The local service is currently stopped. In Desktop mode, applying this source will start it automatically.",
    },
    general: {
      title: "General",
      description: "Baseline preferences for the main workspace views.",
      defaultView: "Default View",
      defaultViewDescription: "Choose the default layout for displaying items.",
      appMode: "Application Mode",
      appModeDescription: "Select the interface complexity level.",
      language: "Language",
      languageDescription: "Select the dashboard language.",
      languagePlaceholder: "Select language",
    },
    appearance: {
      title: "Appearance",
      description: "Customize the look and feel of the dashboard.",
      themeTitle: "Theme",
      themeDescription: "Switch between light and dark mode.",
      systemPreferenceTitle: "System Preference",
      systemPreferenceDescription:
        "Follow the operating system preference automatically.",
      menuBarTitle: "Menu Bar Icon",
      menuBarDescription: "Control the visibility of the menu bar icon.",
      menuBarIconTitle: "Menu Bar Icon Mode",
      dockTitle: "Dock / Taskbar Icon",
      dockDescription:
        "Show MCPMate in the Dock (macOS), taskbar (Windows/Linux), or run from the tray or menu bar only.",
      dockIconTitle: "Dock Icon Mode",
      dockHiddenNotice:
        "The Dock or taskbar entry is hidden. The tray icon stays visible so you can reopen MCPMate.",
      menuBarPlaceholder: "Select menu bar icon mode",
      wipLabel: "Work in Progress",
      defaultMarketPlaceholder: "Select default market",
    },
    options: {
      theme: {
        light: "Light",
        dark: "Dark",
      },
      defaultView: {
        list: "List",
        grid: "Grid",
      },
      appMode: {
        express: "Express",
        expert: "Expert",
      },
      clientMode: {
        hosted: "Hosted",
        transparent: "Transparent",
      },
      backup: {
        keepN: "Keep N",
        keepLast: "Keep Last",
        none: "None",
      },
      menuBar: {
        runtime: "Visible When Running",
        hidden: "Hidden",
      },
    },
    servers: {
      title: "Server",
      description: "Decide how server operations propagate across clients.",
      syncTitle: "Sync Global Start/Stop",
      syncDescription: "Push global enable state to managed clients instantly.",
      autoAddTitle: "Auto Add To Default Profile",
      autoAddDescription:
        "Include new servers in the default profile automatically.",
      liveLogsTitle: "Server Detail Logs",
      liveLogsDescription:
        "Show paginated live logs on the Server detail page.",
    },
    clients: {
      title: "Client",
      description:
        "Configure default rollout and backup behavior for client apps.",
      defaultVisibilityTitle: "Default Client Visibility",
      defaultVisibilityDescription:
        "Choose which client statuses are shown by default on the Clients page.",
      defaultVisibility: {
        all: "All",
        detected: "Detected",
        managed: "Managed",
      },
      modeTitle: "Client Management Mode",
      modeDescription:
        "Choose how client configurations should be managed by default.",
      backupStrategyTitle: "Client Backup Strategy",
      backupStrategyDescription:
        "Define how client configurations should be backed up.",
      backupLimitTitle: "Maximum Backup Copies",
      backupLimitDescription:
        "Set the maximum number of backup copies to keep. Applied when the strategy is set to Keep N. Values below 1 are rounded up.",
      liveLogsTitle: "Client Detail Logs",
      liveLogsDescription:
        "Show paginated live logs on the Client detail page.",
    },
    profile: {
      title: "Profile",
      description: "Token estimates, profile detail logs, and related options.",
      liveLogsTitle: "Profile Detail Logs",
      liveLogsDescription:
        "Show paginated live logs on the Profile detail page.",
      tokenEstimateTitle: "Profile token estimate",
      tokenEstimateDescription:
        "Tokenizer used for profile capability size on the chart and dashboard.",
      tokenEstimateOpenAI: "OpenAI (cl100k_base)",
      tokenEstimateAnthropic: "Anthropic Claude",
    },
    developer: {
      title: "Developer",
      description:
        "Experimental toggles for internal inspection and navigation visibility.",
      enableServerDebugTitle: "Enable Server Inspection",
      enableServerDebugDescription:
        "Expose inspection instrumentation for newly added servers.",
      openDebugInNewWindowTitle: "Open Inspect Views In New Window",
      openDebugInNewWindowDescription:
        "When enabled, Inspect buttons launch a separate tab instead of navigating the current view.",
      showApiDocsTitle: "Show API Docs Menu",
      showApiDocsDescription:
        "Display API documentation menu in the navigation.",
      showDefaultHeadersTitle: "Show Default HTTP Headers",
      showDefaultHeadersDescription:
        "Display the server's default HTTP headers (values are redacted) in Server Details. Use only for inspection.",
      showRawJsonTitle: "Show Raw Capability JSON",
      showRawJsonDescription:
        "Display raw JSON payloads under Details in capability lists (Server details and Uni‑Import preview).",
    },
    market: {
      title: "Market",
      description:
        "Configure default market and manage hidden marketplace servers.",
      defaultMarketTitle: "Default Market",
      defaultMarketDescription:
        "Choose which market appears first and cannot be closed.",
      officialPortal: "Official MCP Registry",
      enableBlacklistTitle: "Enable Blacklist",
      enableBlacklistDescription:
        "Hide quality-poor or unavailable content from the market to keep it clean",
      searchHiddenServers: "Search hidden servers",
      sortHiddenServers: "Sort hidden servers",
      sortPlaceholder: "Sort",
      emptyTitle: "No hidden servers currently.",
      emptyDescription:
        "Hide servers from the Market list to keep this space tidy. They will appear here for recovery.",
      noNotes: "No notes added.",
      browserExtensionsTitle: "Browser Extensions",
      browserExtensionsDescription:
        "Install MCPMate browser extensions to detect importable MCP server snippets and one-click import them into MCPMate.",
      installChromeExtension: "Install Chrome Extension",
      installChromeExtensionDescription:
        "Detect MCP server snippets and import to MCPMate in one click.",
      installEdgeExtension: "Install Edge Extension",
      installEdgeExtensionDescription:
        "Find MCP server configs on web pages and import with one click.",
      browserExtensionsReviewHint:
        "Extensions are under store review. Install when approved.",
      browserExtensionsPlaceholderHint:
        "Placeholder links for now. They will be replaced with official extension store URLs.",
      hiddenOn: "Hidden on {{value}}",
      restore: "Restore",
    },
    audit: {
      title: "Audit Policy",
      description: "Manage how long audit events are retained in the database.",
      liveLogsTitle: "Detail Live Logs",
      liveLogsDescription:
        "Control whether audit-backed live logs are shown in the Profile detail page.",
      liveLogsClientTitle: "Client Detail Logs",
      liveLogsClientDescription:
        "Show paginated live logs on the Client detail page.",
      liveLogsServerTitle: "Server Detail Logs",
      liveLogsServerDescription:
        "Show paginated live logs on the Server detail page.",
      liveLogsProfileTitle: "Profile Detail Logs",
      liveLogsProfileDescription:
        "Show paginated live logs on the Profile detail page.",
      saved: "Retention policy saved",
      saveFailed: "Failed to save policy",
      typeTitle: "Retention Strategy",
      typeDescription: "Select how events are automatically pruned.",
      typeCombined: "Combined (days + count)",
      typeDays: "Keep by days",
      typeCount: "Keep by count",
      typeOff: "Disabled (keep all)",
      daysTitle: "Days to keep",
      daysDescription: "Events older than this number of days will be deleted.",
      countTitle: "Max events",
      countDescription:
        "If event count exceeds this limit, oldest events will be deleted.",
      saving: "Saving...",
      save: "Save Policy",
    },
    about: {
      title: "About MCPMate",
      description:
        "Open-source acknowledgements for the MCPMate preview build.",
      lastUpdated: "Last updated: {{date}}",
      backendTitle: "Backend (Rust workspace)",
      desktopShellTitle: "Desktop Shell (Tauri)",
      dashboardTitle: "Dashboard (Web)",
      components: "{{count}} components",
      repository: "Repository",
      homepage: "Homepage",
      noPackages: "No third-party packages detected during the latest update.",
    },
    notices: {
      dockHidden:
        "The Dock or taskbar entry is hidden. The tray icon stays visible so you can reopen MCPMate.",
    },
  },
  "zh-CN": {
    title: "调整面板偏好与默认行为",
    tabs: {
      general: "通用",
      appearance: "外观",
      audit: "审计",
      profile: "配置集",
      system: "系统",
      serverControls: "服务器",
      clientDefaults: "客户端",
      market: "服务源",
      developer: "开发者",
      about: "关于",
    },
    system: {
      title: "系统",
      description:
        "管理 MCPMate Desktop 如何连接并控制服务，包括运行模式与本地端口。",
      sourceTitle: "服务连接目标",
      sourceDescription:
        "选择 Desktop 连接到内置本地服务，或远程服务端点。",
      sourceOptions: {
        localhost: "本地",
        remote: "远程",
      },
      runtimeModeTitle: "本地运行模式",
      runtimeModeDescription:
        "选择本地服务以操作系统服务运行，或跟随 MCPMate Desktop 生命周期运行。",
      runtimeModeCurrent: "当前运行模式：{{value}}",
      moreToggle: "更多",
      lessToggle: "收起",
      runtimeModeOptions: {
        service: "服务",
        desktopManaged: "桌面",
      },
      remoteUrlTitle: "远程 Core URL",
      remoteUrlDescription:
        "先保存远程 core 地址以便后续接入；当前阶段仍以本地 core 常驻管理为主。",
      remoteUrlPlaceholder: "https://your-core.example.com",
      apiPortTitle: "本地服务 API 端口",
      apiPortDescription: "本地 REST 与控制台访问端口（默认 8080）。",
      mcpPortTitle: "本地服务 MCP 端口",
      mcpPortDescription: "本地 MCP 代理端点端口（/mcp），默认 8000。",
      apply: "应用 Core 服务源",
      helperTauri:
        "Tauri：保存所选 core 服务源，并通过操作系统服务命令管理 localhost core，在下次启动时自动重新挂载。",
      helperWeb: "Web：修改端口后，请在外部重启后端进程。",
      webDialogTitle: "网页环境应用与重启",
      webDialogDesc:
        "浏览器无法直接重启后端，请复制以下命令并在终端执行（使用上方端口）。",
      optionCargoTitle: "方案 A — cargo run（开发）",
      optionBinaryTitle: "方案 B — 二进制（发布）",
      copy: "复制",
      stopCurrent: "停止当前后端",
      close: "关闭",
      applyButtonBusy: "正在应用…",
      applyProgressHint:
        "正在更新所选 core 服务源。桌面端重新挂载期间，API 可能短暂失败。",
      applySuccessTitle: "Core 服务源已更新",
      applySuccessDescription:
        "桌面端现已挂载到所选 {{source}} core，如有需要也已刷新本地服务定义。",
      applyFailedTitle: "无法更新 Core 服务源",
      serviceStatusTitle: "本地服务状态",
      serviceStatusFallback:
        "当已配置的本地服务可用时，桌面端会自动连接。",
      serviceLevel: "服务级别：{{value}}",
      statusAction: "状态",
      startAction: "启动",
      restartAction: "重启",
      stopAction: "停止",
      installAction: "安装",
      uninstallAction: "卸载",
      serviceActionBusy: "处理中…",
      serviceActionSuccessTitle: "本地 Core 服务已更新",
      serviceActionSuccessDescription:
        "本地 Core 服务操作 {{action}} 已完成，当前状态：{{status}}。",
      serviceActionFailedTitle: "无法管理本地 Core 服务",
      portsReloadFailedTitle: "无法从桌面壳读取端口",
      portsReloadFailedDescription:
        "如有缓存将显示缓存值。请确认应用正常后再次点击重新加载。",
      localhostRunning:
        "本地服务正在运行；在 Desktop 模式下，只有真正退出 MCPMate 才会停止。",
      localhostStopped:
        "本地服务当前未运行；在 Desktop 模式下，应用该服务源后会自动启动。",
    },
    general: {
      title: "通用",
      description: "管理工作区的默认视图与基础偏好。",
      defaultView: "默认视图",
      defaultViewDescription: "选择条目显示的默认布局方式。",
      appMode: "应用模式",
      appModeDescription: "选择界面复杂度和信息层级。",
      language: "界面语言",
      languageDescription: "切换控制台显示语言。",
      languagePlaceholder: "请选择语言",
    },
    appearance: {
      title: "外观",
      description: "自定义仪表盘的外观和感觉。",
      themeTitle: "主题",
      themeDescription: "在浅色和深色模式之间切换。",
      systemPreferenceTitle: "系统偏好",
      systemPreferenceDescription: "自动跟随操作系统偏好设置。",
      menuBarTitle: "菜单栏图标",
      menuBarDescription: "控制菜单栏图标的可见性。",
      menuBarIconTitle: "菜单栏图标模式",
      dockTitle: "Dock / 任务栏图标",
      dockDescription:
        "在 macOS Dock 或 Windows/Linux 任务栏中显示 MCPMate，或仅从托盘或菜单栏运行。",
      dockIconTitle: "Dock 图标模式",
      dockHiddenNotice:
        "Dock 或任务栏入口已隐藏，托盘图标保持可见以便重新打开 MCPMate。",
      menuBarPlaceholder: "选择菜单栏图标模式",
      wipLabel: "开发中",
      defaultMarketPlaceholder: "选择默认市场",
    },
    options: {
      theme: {
        light: "浅色",
        dark: "深色",
      },
      defaultView: {
        list: "列表",
        grid: "网格",
      },
      appMode: {
        express: "简洁",
        expert: "专业",
      },
      clientMode: {
        hosted: "托管",
        transparent: "透明",
      },
      backup: {
        keepN: "保留 N 个",
        keepLast: "保留最新",
        none: "不保留",
      },
      menuBar: {
        runtime: "运行时可见",
        hidden: "隐藏",
      },
    },
    servers: {
      title: "服务器",
      description: "决定服务操作如何在客户端之间传播。",
      syncTitle: "同步全局启停",
      syncDescription: "立即将全局启用状态推送到管理的客户端。",
      autoAddTitle: "自动添加到默认配置文件",
      autoAddDescription: "自动将新服务包含在默认配置文件中。",
      liveLogsTitle: "服务器详情日志",
      liveLogsDescription: "在服务器详情页显示分页现场日志。",
    },
    clients: {
      title: "客户端",
      description: "配置客户端应用的默认部署和备份行为。",
      defaultVisibilityTitle: "默认显示内容",
      defaultVisibilityDescription:
        "选择在“客户端”页面默认展示哪些状态的记录。",
      defaultVisibility: {
        all: "全部",
        detected: "已检测",
        managed: "已管理",
      },
      modeTitle: "客户端管理模式",
      modeDescription: "选择客户端配置默认应如何由 MCPMate 管理。",
      backupStrategyTitle: "客户端备份策略",
      backupStrategyDescription: "定义客户端配置应如何备份。",
      backupLimitTitle: "最大备份副本数",
      backupLimitDescription: "设置要保留的最大备份副本数。",
      liveLogsTitle: "客户端详情日志",
      liveLogsDescription: "在客户端详情页显示分页现场日志。",
    },
    profile: {
      title: "配置集",
      description: "Token 估算、配置集详情日志等选项。",
      liveLogsTitle: "配置集详情日志",
      liveLogsDescription: "在配置集详情页显示分页现场日志。",
      tokenEstimateTitle: "Profile token 估算",
      tokenEstimateDescription:
        "用于估算能力规模的分词方式（详情页图表与仪表盘）。",
      tokenEstimateOpenAI: "OpenAI（cl100k_base）",
      tokenEstimateAnthropic: "Anthropic Claude",
    },
    developer: {
      title: "开发者",
      description: "用于内部检视和导航可见性的实验性开关。",
      enableServerDebugTitle: "启用服务器检视",
      enableServerDebugDescription: "为新添加的服务器公开检视工具。",
      openDebugInNewWindowTitle: "在新窗口中打开检视视图",
      openDebugInNewWindowDescription:
        "启用后，检视按钮将启动单独的标签页而不是导航当前视图。",
      showApiDocsTitle: "显示 API 文档菜单",
      showApiDocsDescription: "在导航中显示 API 文档菜单。",
      showDefaultHeadersTitle: "显示默认 HTTP 头",
      showDefaultHeadersDescription:
        "在服务器详细信息中显示服务器的默认 HTTP 头（值已脱敏）。仅用于检视。",
      showRawJsonTitle: "显示原始能力 JSON",
      showRawJsonDescription:
        "在能力列表中显示原始 JSON 负载（服务器详情和统一导入预览）。",
    },
    market: {
      title: "服务源",
      description: "配置默认服务源并管理隐藏的服务源服务器。",
      defaultMarketTitle: "默认服务源",
      defaultMarketDescription: "选择哪个服务源首先显示且无法关闭。",
      officialPortal: "官方 MCP 注册中心",
      enableBlacklistTitle: "启用黑名单",
      enableBlacklistDescription: "隐藏质量差或不可用的内容以保持服务源清洁",
      searchHiddenServers: "搜索隐藏服务器",
      sortHiddenServers: "排序隐藏服务器",
      sortPlaceholder: "排序",
      emptyTitle: "当前没有隐藏的服务器。",
      emptyDescription:
        "从服务源列表中隐藏服务器以保持此空间整洁。它们将出现在这里以便恢复。",
      noNotes: "未添加备注。",
      browserExtensionsTitle: "浏览器扩展",
      browserExtensionsDescription:
        "安装 MCPMate 浏览器扩展后，可在网页中发现可导入的 MCP Server 配置并一键导入到 MCPMate。",
      installChromeExtension: "安装 Chrome 扩展",
      installChromeExtensionDescription:
        "检测网页中的 MCP Server 片段并一键导入 MCPMate。",
      installEdgeExtension: "安装 Edge 扩展",
      installEdgeExtensionDescription:
        "发现网页中的 MCP Server 配置并一键导入 MCPMate。",
      browserExtensionsReviewHint:
        "扩展正在商店审核中，审核通过后可安装。",
      browserExtensionsPlaceholderHint:
        "当前为占位链接，后续会替换为官方扩展商店地址。",
      hiddenOn: "隐藏于 {{value}}",
      restore: "恢复",
    },
    audit: {
      title: "审计",
      description: "管理审计事件在数据库中的保留时长。",
      liveLogsTitle: "详情页现场日志",
      liveLogsDescription: "控制 Profile 详情页是否显示基于审计事件的分页日志。",
      liveLogsClientTitle: "客户端详情日志",
      liveLogsClientDescription: "在客户端详情页显示分页现场日志。",
      liveLogsServerTitle: "服务器详情日志",
      liveLogsServerDescription: "在服务器详情页显示分页现场日志。",
      liveLogsProfileTitle: "Profile 详情日志",
      liveLogsProfileDescription: "在 Profile 详情页显示分页现场日志。",
      saved: "审计保留策略已保存",
      saveFailed: "保存审计策略失败",
      typeTitle: "保留策略",
      typeDescription: "选择自动清理审计事件的方式。",
      typeCombined: "组合模式（天数 + 数量）",
      typeDays: "按天数保留",
      typeCount: "按数量保留",
      typeOff: "禁用（全部保留）",
      daysTitle: "保留天数",
      daysDescription: "超过该天数的事件将被删除。",
      countTitle: "最大事件数",
      countDescription: "当事件数量超过该上限时，将删除最早的事件。",
      saving: "保存中...",
      save: "保存策略",
    },
    about: {
      title: "关于 MCPMate",
      description: "MCPMate 预览版本的开源致谢信息。",
      lastUpdated: "最后更新：{{date}}",
      backendTitle: "后端 (Rust 工作区)",
      desktopShellTitle: "桌面外壳 (Tauri)",
      dashboardTitle: "仪表盘 (Web)",
      components: "{{count}} 个组件",
      repository: "Repository",
      homepage: "主页",
      noPackages: "在最新更新期间未检测到第三方包。",
    },
    notices: {
      dockHidden:
        "Dock 或任务栏入口已隐藏，托盘图标保持可见以便重新打开 MCPMate。",
    },
  },
  "ja-JP": {
    title: "ダッシュボード設定と既定値の調整",
    tabs: {
      general: "一般",
      appearance: "外観",
      audit: "監査ポリシー",
      profile: "プロファイル管理",
      system: "システム",
      serverControls: "サーバー",
      clientDefaults: "クライアント管理",
      market: "MCP マーケット",
      developer: "開発者",
      about: "情報とライセンス",
    },
    system: {
      title: "システム",
      description:
        "MCPMate Desktop がサービスへ接続・制御する方法（実行モードとローカルポート）を管理します。",
      sourceTitle: "サービス接続先",
      sourceDescription:
        "Desktop の接続先を内蔵ローカルサービスまたは remote サービスエンドポイントから選択します。",
      sourceOptions: {
        localhost: "Local",
        remote: "Remote",
      },
      runtimeModeTitle: "ローカル実行モード",
      runtimeModeDescription:
        "ローカルサービスを OS サービスとして実行するか、MCPMate Desktop のライフサイクルに従わせるかを選択します。",
      runtimeModeCurrent: "現在の実行モード: {{value}}",
      moreToggle: "詳細",
      lessToggle: "折りたたむ",
      runtimeModeOptions: {
        service: "Service",
        desktopManaged: "Desktop",
      },
      remoteUrlTitle: "Remote Core URL",
      remoteUrlDescription:
        "将来の接続用に remote core の URL を保存します。現段階では localhost core の常駐管理を優先します。",
      remoteUrlPlaceholder: "https://your-core.example.com",
      apiPortTitle: "Local Service API ポート",
      apiPortDescription:
        "ローカル REST とダッシュボード用ポート（既定 8080）。",
      mcpPortTitle: "Local Service MCP ポート",
      mcpPortDescription:
        "ローカル MCP プロキシエンドポイント用ポート（/mcp）、既定 8000。",
      apply: "Core サービスを適用",
      helperTauri:
        "Tauri：選択した core サービスを保存し、OS のサービスコマンドで localhost core を管理し、次回起動時に再接続します。",
      helperWeb: "Web：ポート変更後に外部でバックエンドを再起動してください。",
      webDialogTitle: "Web 環境での適用と再起動",
      webDialogDesc:
        "ブラウザからバックエンドは再起動できません。以下のコマンドを選択したポートで実行してください。",
      optionCargoTitle: "方法 A — cargo run（開発）",
      optionBinaryTitle: "方法 B — バイナリ（リリース）",
      copy: "コピー",
      stopCurrent: "現在のバックエンドを停止",
      close: "閉じる",
      applyButtonBusy: "適用中…",
      applyProgressHint:
        "選択した core サービスを更新しています。デスクトップの再接続中は API が一時的に失敗することがあります。",
      applySuccessTitle: "Core サービスを更新しました",
      applySuccessDescription:
        "デスクトップは選択した {{source}} core に接続しました。必要に応じて localhost サービス定義も更新されました。",
      applyFailedTitle: "Core サービスを更新できませんでした",
      serviceStatusTitle: "Local Service 状態",
      serviceStatusFallback:
        "設定済みのローカルサービスが利用可能になると、Desktop は自動で接続します。",
      serviceLevel: "サービスレベル: {{value}}",
      statusAction: "状態",
      startAction: "開始",
      restartAction: "再起動",
      stopAction: "停止",
      installAction: "インストール",
      uninstallAction: "アンインストール",
      serviceActionBusy: "処理中…",
      serviceActionSuccessTitle: "Local core サービスを更新しました",
      serviceActionSuccessDescription:
        "Local core サービス操作 {{action}} が完了しました。現在の状態: {{status}}。",
      serviceActionFailedTitle: "Local core サービスを管理できませんでした",
      portsReloadFailedTitle: "シェルからポートを読み込めませんでした",
      portsReloadFailedDescription:
        "キャッシュがあればそれを表示します。アプリの状態を確認のうえ、再読み込みを試してください。",
      localhostRunning:
        "ローカルサービスは実行中です。Desktop モードでは MCPMate を本当に終了したときだけ停止します。",
      localhostStopped:
        "ローカルサービスは停止中です。Desktop モードではこの設定を適用すると自動起動します。",
    },
    general: {
      title: "一般",
      description: "ワークスペースの基本設定を管理します。",
      defaultView: "既定のビュー",
      defaultViewDescription: "項目の表示レイアウトを選択します。",
      appMode: "アプリモード",
      appModeDescription: "インターフェースの複雑さを選択します。",
      language: "表示言語",
      languageDescription: "ダッシュボードで使用する言語を切り替えます。",
      languagePlaceholder: "言語を選択",
    },
    appearance: {
      title: "外観",
      description: "ダッシュボードの外観と操作性をカスタマイズします。",
      themeTitle: "テーマ",
      themeDescription: "ライトモードとダークモードを切り替えます。",
      systemPreferenceTitle: "システム設定",
      systemPreferenceDescription:
        "オペレーティングシステムの設定を自動的に追従します。",
      menuBarTitle: "メニューバーアイコン",
      menuBarDescription: "メニューバーアイコンの表示を制御します。",
      menuBarIconTitle: "メニューバーアイコンモード",
      dockTitle: "Dock / タスクバーアイコン",
      dockDescription:
        "macOS の Dock、Windows/Linux のタスクバーに表示するか、トレイ／メニューバーのみで実行します。",
      dockIconTitle: "Dock アイコンモード",
      dockHiddenNotice:
        "Dock／タスクバーからの表示をオフにしました。トレイアイコンは残るため、そこから MCPMate を開き直せます。",
      menuBarPlaceholder: "メニューバーアイコンモードを選択",
      wipLabel: "開発中",
      defaultMarketPlaceholder: "デフォルトマーケットを選択",
    },
    options: {
      theme: {
        light: "ライト",
        dark: "ダーク",
      },
      defaultView: {
        list: "リスト",
        grid: "グリッド",
      },
      appMode: {
        express: "ライト",
        expert: "エキスパート",
      },
      clientMode: {
        hosted: "ホスト",
        transparent: "トランスペアレント",
      },
      backup: {
        keepN: "N 件保持",
        keepLast: "最新のみ",
        none: "保持しない",
      },
      menuBar: {
        runtime: "稼働中のみ表示",
        hidden: "非表示",
      },
    },
    servers: {
      title: "サーバー",
      description:
        "サーバー操作がクライアント間でどのように伝播するかを決定します。",
      syncTitle: "グローバル開始/停止の同期",
      syncDescription:
        "グローバル有効状態を管理されたクライアントに即座にプッシュします。",
      autoAddTitle: "デフォルトプロファイルに自動追加",
      autoAddDescription:
        "新しいサーバーをデフォルトプロファイルに自動的に含めます。",
      liveLogsTitle: "サーバー詳細ログ",
      liveLogsDescription:
        "サーバー詳細ページにページング対応ライブログを表示します。",
    },
    clients: {
      title: "クライアント管理",
      description:
        "クライアントアプリのデフォルトロールアウトとバックアップ動作を設定します。",
      defaultVisibilityTitle: "既定の表示内容",
      defaultVisibilityDescription:
        "クライアントページで既定として表示するステータスを選択します。",
      defaultVisibility: {
        all: "すべて",
        detected: "検出済み",
        managed: "管理中",
      },
      modeTitle: "クライアント管理モード",
      modeDescription:
        "クライアント設定をデフォルトでどのように管理するかを選択します。",
      backupStrategyTitle: "クライアントバックアップ戦略",
      backupStrategyDescription:
        "クライアント設定をどのようにバックアップするかを定義します。",
      backupLimitTitle: "最大バックアップコピー数",
      backupLimitDescription: "保持する最大バックアップコピー数を設定します。",
      liveLogsTitle: "クライアント詳細ログ",
      liveLogsDescription:
        "クライアント詳細ページにページング対応ライブログを表示します。",
    },
    profile: {
      title: "プロファイル管理",
      description: "トークン推定、詳細ログなどプロファイル関連のオプション。",
      liveLogsTitle: "プロファイル詳細ログ",
      liveLogsDescription:
        "プロファイル詳細ページにページング対応ライブログを表示します。",
      tokenEstimateTitle: "プロファイルのトークン推定",
      tokenEstimateDescription:
        "能力サイズ推定に使うトークナイザ（チャートとダッシュボード）。",
      tokenEstimateOpenAI: "OpenAI（cl100k_base）",
      tokenEstimateAnthropic: "Anthropic Claude",
    },
    developer: {
      title: "開発者",
      description: "内部検査とナビゲーション可視性のための実験的トグル。",
      enableServerDebugTitle: "サーバー検査を有効化",
      enableServerDebugDescription:
        "新しく追加されたサーバーの検査計装を公開します。",
      openDebugInNewWindowTitle: "新しいウィンドウで検査ビューを開く",
      openDebugInNewWindowDescription:
        "有効にすると、検査ボタンは現在のビューをナビゲートする代わりに別のタブを起動します。",
      showApiDocsTitle: "API ドキュメントメニューを表示",
      showApiDocsDescription:
        "ナビゲーションに API ドキュメントメニューを表示します。",
      showDefaultHeadersTitle: "デフォルト HTTP ヘッダーを表示",
      showDefaultHeadersDescription:
        "サーバー詳細でサーバーのデフォルト HTTP ヘッダー（値は編集済み）を表示します。検査専用です。",
    },
    market: {
      title: "MCP マーケット",
      description:
        "デフォルトマーケットを設定し、非表示のマーケットプレイスサーバーを管理します。",
      defaultMarketTitle: "デフォルトマーケット",
      defaultMarketDescription:
        "最初に表示され、閉じることができないマーケットを選択します。",
      officialPortal: "公式 MCP レジストリ",
      enableBlacklistTitle: "ブラックリストを有効化",
      enableBlacklistDescription:
        "品質の悪いまたは利用できないコンテンツを非表示にしてマーケットを清潔に保ちます",
      searchHiddenServers: "非表示サーバーを検索",
      sortHiddenServers: "非表示サーバーを並べ替え",
      sortPlaceholder: "並べ替え",
      emptyTitle: "現在非表示のサーバーはありません。",
      emptyDescription:
        "マーケットリストからサーバーを非表示にして、このスペースを整理します。復元のためにここに表示されます。",
      noNotes: "メモが追加されていません。",
      browserExtensionsTitle: "ブラウザー拡張",
      browserExtensionsDescription:
        "MCPMate ブラウザー拡張をインストールすると、Web ページ上のインポート可能な MCP Server 設定を検出し、MCPMate へワンクリックで取り込めます。",
      installChromeExtension: "Chrome 拡張をインストール",
      installChromeExtensionDescription:
        "Web 上の MCP Server スニペットを検出してワンクリックで取り込めます。",
      installEdgeExtension: "Edge 拡張をインストール",
      installEdgeExtensionDescription:
        "Web ページ上の MCP Server 設定を見つけてワンクリックで取り込めます。",
      browserExtensionsReviewHint:
        "拡張は現在ストア審査中です。承認後にインストールできます。",
      browserExtensionsPlaceholderHint:
        "現在はプレースホルダーリンクです。後で公式ストア URL に置き換えます。",
      hiddenOn: "非表示日時：{{value}}",
      restore: "復元",
    },
    audit: {
      title: "監査ポリシー",
      description: "監査イベントをデータベースに保持する期間を管理します。",
      liveLogsTitle: "詳細ページのライブログ",
      liveLogsDescription:
        "Profile 詳細ページで監査ベースのページングログを表示するかを制御します。",
      liveLogsClientTitle: "クライアント詳細ログ",
      liveLogsClientDescription:
        "クライアント詳細ページにページング対応ライブログを表示します。",
      liveLogsServerTitle: "サーバー詳細ログ",
      liveLogsServerDescription:
        "サーバー詳細ページにページング対応ライブログを表示します。",
      liveLogsProfileTitle: "Profile 詳細ログ",
      liveLogsProfileDescription:
        "Profile 詳細ページにページング対応ライブログを表示します。",
      saved: "監査保持ポリシーを保存しました",
      saveFailed: "監査ポリシーの保存に失敗しました",
      typeTitle: "保持戦略",
      typeDescription: "監査イベントを自動的に整理する方法を選択してください。",
      typeCombined: "複合（期間 + 件数）",
      typeDays: "日数で保持",
      typeCount: "件数で保持",
      typeOff: "無効（すべて保持）",
      daysTitle: "保持日数",
      daysDescription: "この日数より古いイベントは削除されます。",
      countTitle: "最大イベント数",
      countDescription:
        "イベント数がこの上限を超えると、最も古いイベントから削除されます。",
      saving: "保存中...",
      save: "ポリシーを保存",
    },
    about: {
      title: "MCPMate について",
      description: "MCPMate プレビュービルドのオープンソース謝辞。",
      lastUpdated: "最終更新：{{date}}",
      backendTitle: "バックエンド (Rust ワークスペース)",
      desktopShellTitle: "デスクトップシェル (Tauri)",
      dashboardTitle: "ダッシュボード (Web)",
      components: "{{count}} コンポーネント",
      repository: "リポジトリ",
      homepage: "ホームページ",
      noPackages:
        "最新の更新中にサードパーティパッケージが検出されませんでした。",
    },
    notices: {
      dockHidden:
        "Dock／タスクバーからの表示をオフにしました。トレイアイコンは残るため、そこから MCPMate を開き直せます。",
    },
  },
} as const;
