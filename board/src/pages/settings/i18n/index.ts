export const settingsTranslations = {
  en: {
    title: "Tune dashboard preferences and defaults",
    tabs: {
      general: "General",
      audit: "Logs",
      profile: "Profile",
      security: "Security",
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
          stopped: "The local core service is installed but not running.",
          running:
            "The local core service is running and responding to health checks.",
          running_unhealthy:
            "The service manager reports the local core as running, but health checks are failing.",
        },
      },
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
        "Check the desktop app is healthy and try Reload again.",
      localhostRunning:
        "The local service is running. In Desktop mode it stops only when MCPMate truly quits.",
      localhostStopped:
        "The local service is currently stopped. In Desktop mode, applying this source will start it automatically.",
    },
    general: {
      title: "General",
      description:
        "Baseline preferences for workspace layout, theme, language, and desktop shell options.",
      defaultView: "Default View",
      defaultViewDescription: "Choose the default layout for displaying items.",
      themeTitle: "Theme",
      themeDescription: "Switch between light, dark, and system theme.",
      language: "Language",
      languageDescription: "Select the dashboard language.",
      languagePlaceholder: "Select language",
      menuBarTitle: "Menu Bar Icon",
      menuBarDescription: "Choose when the desktop tray icon should appear.",
      dockTitle: "Dock / Taskbar Icon",
      dockDescription:
        "Show MCPMate in the Dock (macOS), taskbar (Windows/Linux), or run from the tray or menu bar only.",
      dockHiddenNotice:
        "The Dock or taskbar entry is hidden. The tray icon stays visible so you can reopen MCPMate.",
    },
    options: {
      theme: {
        light: "Light",
        auto: "Auto",
        dark: "Dark",
      },
      defaultView: {
        list: "List",
        grid: "Grid",
      },
      clientMode: {
        unify: "Unify",
        hosted: "Hosted",
        transparent: "Transparent",
        transparentDisabledTooltip:
          "Transparent cannot be the workspace default. Enable it per client when a writable local path is available.",
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
        allowed: "Allowed",
        pending: "Pending",
        denied: "Denied",
      },
      modeTitle: "Client Management Mode",
      modeDescription:
        "Choose how client configurations should be managed by default.",
      firstContactTitle: "First-contact Behavior",
      firstContactDescription:
        "Control how new, unknown clients are handled when they first request an MCP connection.",
      firstContact: {
        deny: "Deny",
        review: "Review",
        allow: "Allow",
      },
      policySaved: "Default client policy updated.",
      policySaveFailed: "Failed to update default client policy",
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
      inspectorTimeoutTitle: "Inspector Timeout (ms)",
      inspectorTimeoutDescription:
        "Default timeout for tool/resource/prompt calls in the Inspector drawer.",
      inspectorTimeoutSaved: "Inspector timeout updated",
      inspectorTimeoutSaveError: "Failed to save inspector timeout",
      saving: "Saving...",
      save: "Save",
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
      browserExtensionsAvailableHint:
        "Browser extensions are now available on Chrome Web Store and Microsoft Edge Add-ons.",
      browserExtensionsStoreHint:
        "Install from Chrome Web Store or Microsoft Edge Add-ons.",
      hiddenOn: "Hidden on {{value}}",
      restore: "Restore",
    },
    audit: {
      title: "Log retention",
      description:
        "Control how long activity log events are kept in the local database.",
      liveLogsTitle: "Detail Live Logs",
      liveLogsDescription:
        "Control whether paginated live logs (backed by stored events) appear on the Profile detail page.",
      liveLogsClientTitle: "Client Detail Logs",
      liveLogsClientDescription:
        "Show paginated live logs on the Client detail page.",
      liveLogsServerTitle: "Server Detail Logs",
      liveLogsServerDescription:
        "Show paginated live logs on the Server detail page.",
      liveLogsProfileTitle: "Profile Detail Logs",
      liveLogsProfileDescription:
        "Show paginated live logs on the Profile detail page.",
      saved: "Log retention settings saved",
      saveFailed: "Failed to save log retention settings",
      typeTitle: "Retention Strategy",
      typeDescription: "Choose how stored log events are pruned automatically.",
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
    security: {
      title: "Security",
      description: "Login password and root key encryption settings.",
      loading: "Checking store status...",
      error: {
        title: "Status check failed",
        description: "Could not retrieve store status.",
        retry: "Retry",
      },
      passwordProtection: "Password Protection",
      passwordProtectionDescription:
        "Require a login password before accessing MCPMate or Settings.",
      protectionDisabled: "Protection disabled",
      protectionEnabled: "Protection enabled",
      protectionLevel: {
        startup: "On entry",
        settings: "In Settings",
        off: "None",
      },
      loginPasswordRow: "Login Password",
      loginPasswordRowDescription: "Login password used when protection is enabled.",
      encryptionMode: "Encryption Mode",
      encryptionModeDescription: "How the root encryption key is stored and protected.",
      encryptionModeStatus: "Encryption mode security level",
      providerModeUnknown:
        "Current encryption mode could not be determined. Choose a mode below to switch away from a broken provider.",
      encryptionPasswordRow: "Encryption Password",
      encryptionPasswordRowDescription:
        "Master password that wraps the root encryption key.",
      mode: {
        os: "OS Keychain",
        passphrase: "Password",
        local: "Local File",
        osDetail:
          "Root key stored in macOS Keychain, Windows Credential Manager, or Linux Secret Service. Best protection — no password needed.",
        passphraseDetail:
          "Protect secrets with a password you set. Losing the password makes stored secrets unrecoverable.",
        localDetail:
          "Root key stored as a file in the app data directory. Protected by file permissions only — not recommended for sensitive environments.",
      },
      setPassword: "Set Password",
      changePassword: "Change Password",
      setPasswordTitle: "Set Login Password",
      changePasswordTitle: "Change Login Password",
      removePasswordTitle: "Remove Login Password",
      setPasswordDescription: "Require this password to unlock MCPMate on startup.",
      changePasswordDescription: "Update the password used to unlock MCPMate.",
      clearDescription: "Enter your current password to remove protection.",
      changePasswordAction: "Change Password",
      removePasswordAction: "Remove Password",
      currentPassword: "Current Password",
      newPassword: "New Password",
      confirmPassword: "Confirm Password",
      passphraseLabel: "Master Password",
      passphrasePlaceholder: "Enter password...",
      passphraseConfirmPlaceholder: "Re-enter password...",
      passphraseRequired: "Enter a master password to continue.",
      passphraseMismatch: "Passwords do not match.",
      currentPassphraseRequired: "Enter your current master password to continue.",
      passphraseSetupTitle: "Set Master Password",
      passphraseSetupDescription:
        "This password wraps your root encryption key. It is not stored in plaintext. You will need it again only when switching away from Password encryption mode.",
      passphraseSetupContinue: "Continue",
      currentPassphraseTitle: "Enter Current Master Password",
      currentPassphraseDescription:
        "Your root key is wrapped with your current master password. Enter it to unlock the key before switching encryption mode.",
      confirmTitle: "Switch Security Mode?",
      confirmDescription:
        "This will rotate Secure Store records to a new provider. MCPMate verifies existing records first and keeps the current provider authoritative if rotation fails.",
      confirmPhraseLabel: "Type ROTATE SECRETS to continue",
      confirmPhrasePlaceholder: "ROTATE SECRETS",
      confirmPhraseDescription:
        "This migration rewrites secure-store key wrapping metadata. The phrase must be typed manually.",
      confirmCancel: "Cancel",
      confirmAction: "Switch Mode",
      switching: "Switching...",
      saving: "Saving...",
      passwordSet: "Password set successfully",
      passwordSetError: "Failed to set password",
      passwordChanged: "Password changed successfully",
      passwordChangeError: "Failed to change password",
      passwordCleared: "Password removed",
      passwordClearError: "Failed to remove password",
      passwordRequired: "Enter a password to continue.",
      passwordChangeRequired: "Enter your current and new passwords.",
      passwordClearRequired: "Enter your current password to remove protection.",
      protectionScopeUpdated: "Protection mode updated",
      protectionScopeUpdateError: "Failed to update protection mode",
      switchMissingMode: "Select an encryption mode to continue.",
      encryptionPasswordRotated: "Encryption password updated successfully",
      encryptionPasswordRotateError: "Failed to update encryption password",
      switchSuccess: "Security mode updated successfully",
      switchError: "Failed to switch security mode",
      issue: {
        title: "Store Issue",
      },
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
  },
  "zh-CN": {
    title: "调整面板偏好与默认行为",
    tabs: {
      general: "通用",
      audit: "日志",
      profile: "配置集",
      system: "系统",
      serverControls: "服务器",
      clientDefaults: "客户端",
      market: "服务源",
      developer: "开发者",
      security: "安全",
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
          running: "本地 Core 由 MCPMate Desktop 管理，并会在应用退出时停止。",
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
        "请确认应用正常后再次点击重新加载。",
      localhostRunning:
        "本地服务正在运行；在 Desktop 模式下，只有真正退出 MCPMate 才会停止。",
      localhostStopped:
        "本地服务当前未运行；在 Desktop 模式下，应用该服务源后会自动启动。",
    },
    general: {
      title: "通用",
      description: "管理工作区布局、主题、语言与桌面壳层选项。",
      defaultView: "默认视图",
      defaultViewDescription: "选择条目显示的默认布局方式。",
      themeTitle: "主题",
      themeDescription: "在浅色、深色与跟随系统之间切换。",
      language: "界面语言",
      languageDescription: "切换控制台显示语言。",
      languagePlaceholder: "请选择语言",
      menuBarTitle: "菜单栏图标",
      menuBarDescription: "选择桌面托盘图标的显示时机。",
      dockTitle: "Dock / 任务栏图标",
      dockDescription:
        "在 macOS Dock 或 Windows/Linux 任务栏中显示 MCPMate，或仅从托盘或菜单栏运行。",
      dockHiddenNotice:
        "Dock 或任务栏入口已隐藏，托盘图标保持可见以便重新打开 MCPMate。",
    },
    options: {
      theme: {
        light: "浅色",
        auto: "自动",
        dark: "深色",
      },
      defaultView: {
        list: "列表",
        grid: "网格",
      },
      clientMode: {
        unify: "统一模式",
        hosted: "托管",
        transparent: "透明",
        transparentDisabledTooltip:
          "透明模式不能作为工作区默认值。请在有可写本地路径时按客户端单独启用。",
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
        allowed: "已允许",
        pending: "待审批",
        denied: "已拒绝",
      },
      modeTitle: "客户端管理模式",
      modeDescription: "选择客户端配置默认应如何由 MCPMate 管理。",
      firstContactTitle: "首次连接行为",
      firstContactDescription:
        "控制未知客户端首次请求 MCP 连接时的处理方式。",
      firstContact: {
        deny: "拒绝",
        review: "需审批",
        allow: "允许",
      },
      policySaved: "默认客户端策略已更新。",
      policySaveFailed: "更新默认客户端策略失败",
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
      inspectorTimeoutTitle: "Inspector 超时（毫秒）",
      inspectorTimeoutDescription:
        "Inspector 抽屉中 tool/resource/prompt 调用的默认超时时间。",
      inspectorTimeoutSaved: "Inspector 超时已更新",
      inspectorTimeoutSaveError: "保存 Inspector 超时失败",
      saving: "保存中…",
      save: "保存",
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
      browserExtensionsAvailableHint:
        "浏览器扩展已上架 Chrome 网上应用店和 Microsoft Edge 加载项。",
      browserExtensionsStoreHint:
        "可从 Chrome 网上应用店或 Microsoft Edge 加载项安装。",
      hiddenOn: "隐藏于 {{value}}",
      restore: "恢复",
    },
    audit: {
      title: "日志保留",
      description: "控制活动日志事件在本地数据库中的保留时长。",
      liveLogsTitle: "详情页现场日志",
      liveLogsDescription:
        "控制是否在配置集详情页显示基于已存储事件的分页现场日志。",
      liveLogsClientTitle: "客户端详情日志",
      liveLogsClientDescription: "在客户端详情页显示分页现场日志。",
      liveLogsServerTitle: "服务器详情日志",
      liveLogsServerDescription: "在服务器详情页显示分页现场日志。",
      liveLogsProfileTitle: "Profile 详情日志",
      liveLogsProfileDescription: "在 Profile 详情页显示分页现场日志。",
      saved: "日志保留设置已保存",
      saveFailed: "保存日志保留设置失败",
      typeTitle: "保留策略",
      typeDescription: "选择如何自动清理已存储的日志事件。",
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
    security: {
      title: "安全",
      description: "登录密码与根密钥加密设置。",
      loading: "正在检查存储状态…",
      error: {
        title: "状态检查失败",
        description: "无法获取存储状态。",
        retry: "重试",
      },
      passwordProtection: "密码保护",
      passwordProtectionDescription:
        "访问 MCPMate 或设置页面前需要输入登录密码。",
      protectionDisabled: "保护已关闭",
      protectionEnabled: "保护已启用",
      protectionLevel: {
        startup: "启动时",
        settings: "进入设置时",
        off: "无",
      },
      loginPasswordRow: "登录密码",
      loginPasswordRowDescription: "启用保护后使用的登录密码。",
      encryptionMode: "加密模式",
      encryptionModeDescription: "根加密密钥的存储与保护方式。",
      encryptionModeStatus: "加密模式安全级别",
      providerModeUnknown:
        "无法确定当前加密模式。请在下方选择模式以从故障提供方切换离开。",
      encryptionPasswordRow: "加密密码",
      encryptionPasswordRowDescription: "用于包裹根加密密钥的主密码。",
      mode: {
        os: "系统钥匙串",
        passphrase: "密码",
        local: "本地文件",
        osDetail:
          "根密钥保存在 macOS 钥匙串、Windows Credential Manager 或 Linux Secret Service 中。安全性最佳，无需额外密码。",
        passphraseDetail:
          "使用你设置的密码保护密钥。丢失密码将导致已存储的密钥无法恢复。",
        localDetail:
          "根密钥以文件形式保存在应用数据目录中，仅受文件权限保护——不建议用于敏感环境。",
      },
      setPassword: "设置密码",
      changePassword: "修改密码",
      setPasswordTitle: "设置登录密码",
      changePasswordTitle: "修改登录密码",
      removePasswordTitle: "移除登录密码",
      setPasswordDescription: "启动 MCPMate 时需要输入此密码。",
      changePasswordDescription: "更新用于解锁 MCPMate 的密码。",
      clearDescription: "输入当前密码以移除保护。",
      changePasswordAction: "修改密码",
      removePasswordAction: "移除密码",
      currentPassword: "当前密码",
      newPassword: "新密码",
      confirmPassword: "确认密码",
      passphraseLabel: "主密码",
      passphrasePlaceholder: "输入密码…",
      passphraseConfirmPlaceholder: "再次输入密码…",
      passphraseRequired: "请输入主密码以继续。",
      passphraseMismatch: "两次输入的密码不一致。",
      currentPassphraseRequired: "请输入当前主密码以继续。",
      passphraseSetupTitle: "设置主密码",
      passphraseSetupDescription:
        "此密码用于包裹根加密密钥，不会以明文存储。仅在离开「密码」加密模式时需要再次输入。",
      passphraseSetupContinue: "继续",
      currentPassphraseTitle: "输入当前主密码",
      currentPassphraseDescription:
        "根密钥由当前主密码包裹。切换加密模式前请先输入以解锁密钥。",
      confirmTitle: "切换安全模式？",
      confirmDescription:
        "这会将 Secure Store 记录轮换到新的存储提供方。MCPMate 会先验证现有记录；如果轮换失败，当前提供方仍保持权威状态。",
      confirmPhraseLabel: "输入 ROTATE SECRETS 以继续",
      confirmPhrasePlaceholder: "ROTATE SECRETS",
      confirmPhraseDescription:
        "此迁移会重写 Secure Store 的密钥包裹元数据。确认短语必须手动输入。",
      confirmCancel: "取消",
      confirmAction: "切换模式",
      switching: "切换中…",
      saving: "保存中…",
      passwordSet: "密码设置成功",
      passwordSetError: "设置密码失败",
      passwordChanged: "密码修改成功",
      passwordChangeError: "修改密码失败",
      passwordCleared: "密码已移除",
      passwordClearError: "移除密码失败",
      passwordRequired: "请输入密码以继续。",
      passwordChangeRequired: "请输入当前密码和新密码。",
      passwordClearRequired: "请输入当前密码以移除保护。",
      protectionScopeUpdated: "保护模式已更新",
      protectionScopeUpdateError: "更新保护模式失败",
      switchMissingMode: "请选择加密模式以继续。",
      encryptionPasswordRotated: "加密密码已更新",
      encryptionPasswordRotateError: "更新加密密码失败",
      switchSuccess: "安全模式已更新",
      switchError: "切换安全模式失败",
      issue: {
        title: "存储问题",
      },
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
  },
  "ja-JP": {
    title: "ダッシュボード設定と既定値の調整",
    tabs: {
      general: "一般",
      audit: "ログ",
      profile: "プロファイル管理",
      system: "システム",
      serverControls: "サーバー",
      clientDefaults: "クライアント管理",
      market: "MCP マーケット",
      developer: "開発者",
      security: "セキュリティ",
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
        "アプリの状態を確認のうえ、再読み込みを試してください。",
      localhostRunning:
        "ローカルサービスは実行中です。Desktop モードでは MCPMate を本当に終了したときだけ停止します。",
      localhostStopped:
        "ローカルサービスは停止中です。Desktop モードではこの設定を適用すると自動起動します。",
    },
    general: {
      title: "一般",
      description:
        "ワークスペースのレイアウト、テーマ、言語、デスクトップシェル設定を管理します。",
      defaultView: "既定のビュー",
      defaultViewDescription: "項目の表示レイアウトを選択します。",
      themeTitle: "テーマ",
      themeDescription: "ライト、ダーク、システム設定の追従を切り替えます。",
      language: "表示言語",
      languageDescription: "ダッシュボードで使用する言語を切り替えます。",
      languagePlaceholder: "言語を選択",
      menuBarTitle: "メニューバーアイコン",
      menuBarDescription: "デスクトップトレイアイコンの表示タイミングを選択します。",
      dockTitle: "Dock / タスクバーアイコン",
      dockDescription:
        "macOS の Dock、Windows/Linux のタスクバーに表示するか、トレイ／メニューバーのみで実行します。",
      dockHiddenNotice:
        "Dock／タスクバーからの表示をオフにしました。トレイアイコンは残るため、そこから MCPMate を開き直せます。",
    },
    options: {
      theme: {
        light: "ライト",
        auto: "自動",
        dark: "ダーク",
      },
      defaultView: {
        list: "リスト",
        grid: "グリッド",
      },
      clientMode: {
        unify: "Unify",
        hosted: "ホスト",
        transparent: "トランスペアレント",
        transparentDisabledTooltip:
          "トランスペアレントはワークスペースの既定にはできません。書き込み可能なローカルパスがあるクライアントごとに有効化してください。",
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
        allowed: "許可済み",
        pending: "承認待ち",
        denied: "拒否",
      },
      modeTitle: "クライアント管理モード",
      modeDescription:
        "クライアント設定をデフォルトでどのように管理するかを選択します。",
      firstContactTitle: "初回接続時の挙動",
      firstContactDescription:
        "初めて MCP 接続を要求する未知のクライアントをどう扱うかを設定します。",
      firstContact: {
        deny: "拒否",
        review: "承認が必要",
        allow: "許可",
      },
      policySaved: "既定のクライアントポリシーを更新しました。",
      policySaveFailed: "既定のクライアントポリシーを更新できませんでした",
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
      showRawJsonTitle: "生の能力 JSON を表示",
      showRawJsonDescription:
        "能力リスト（サーバー詳細と統合インポートプレビュー）で生の JSON ペイロードを表示します。",
      inspectorTimeoutTitle: "Inspector タイムアウト（ms）",
      inspectorTimeoutDescription:
        "Inspector ドロワーでの tool/resource/prompt 呼び出しの既定タイムアウト。",
      inspectorTimeoutSaved: "Inspector タイムアウトを更新しました",
      inspectorTimeoutSaveError: "Inspector タイムアウトの保存に失敗しました",
      saving: "保存中…",
      save: "保存",
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
      browserExtensionsAvailableHint:
        "ブラウザ拡張機能は Chrome ウェブストアと Microsoft Edge アドオンで公開中です。",
      browserExtensionsStoreHint:
        "Chrome ウェブストアまたは Microsoft Edge アドオンからインストールできます。",
      hiddenOn: "非表示日時：{{value}}",
      restore: "復元",
    },
    audit: {
      title: "ログの保持",
      description:
        "アクティビティログをローカルデータベースにどのくらい保持するかを設定します。",
      liveLogsTitle: "詳細ページのライブログ",
      liveLogsDescription:
        "プロファイル詳細ページで、保存済みイベントに基づくページング付きライブログを表示するかどうかを制御します。",
      liveLogsClientTitle: "クライアント詳細ログ",
      liveLogsClientDescription:
        "クライアント詳細ページにページング対応ライブログを表示します。",
      liveLogsServerTitle: "サーバー詳細ログ",
      liveLogsServerDescription:
        "サーバー詳細ページにページング対応ライブログを表示します。",
      liveLogsProfileTitle: "Profile 詳細ログ",
      liveLogsProfileDescription:
        "Profile 詳細ページにページング対応ライブログを表示します。",
      saved: "ログ保持設定を保存しました",
      saveFailed: "ログ保持設定の保存に失敗しました",
      typeTitle: "保持戦略",
      typeDescription: "保存済みログイベントを自動的に整理する方法を選択します。",
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
    security: {
      title: "セキュリティ",
      description: "ログインパスワードとルートキー暗号化の設定。",
      loading: "ストア状態を確認中…",
      error: {
        title: "状態確認に失敗しました",
        description: "ストア状態を取得できませんでした。",
        retry: "再試行",
      },
      passwordProtection: "パスワード保護",
      passwordProtectionDescription:
        "MCPMate または設定画面にアクセスする前にログインパスワードを要求します。",
      protectionDisabled: "保護オフ",
      protectionEnabled: "保護オン",
      protectionLevel: {
        startup: "起動時",
        settings: "設定画面",
        off: "なし",
      },
      loginPasswordRow: "ログインパスワード",
      loginPasswordRowDescription: "保護が有効な場合に使用するログインパスワード。",
      encryptionMode: "暗号化モード",
      encryptionModeDescription:
        "ルート暗号化キーの保存と保護方法。",
      encryptionModeStatus: "暗号化モードのセキュリティレベル",
      providerModeUnknown:
        "現在の暗号化モードを特定できません。下からモードを選び、障害のあるプロバイダーから切り替えてください。",
      encryptionPasswordRow: "暗号化パスワード",
      encryptionPasswordRowDescription:
        "ルート暗号化キーをラップするマスターパスワード。",
      mode: {
        os: "OS キーチェーン",
        passphrase: "パスワード",
        local: "ローカルファイル",
        osDetail:
          "ルートキーは macOS Keychain、Windows Credential Manager、または Linux Secret Service に保存されます。最も安全で、追加パスワードは不要です。",
        passphraseDetail:
          "設定したパスワードでシークレットを保護します。パスワードを失うと保存済みシークレットは復元できません。",
        localDetail:
          "ルートキーはアプリデータディレクトリ内のファイルとして保存されます。ファイル権限のみで保護され、機密環境には非推奨です。",
      },
      setPassword: "パスワードを設定",
      changePassword: "パスワードを変更",
      setPasswordTitle: "ログインパスワードを設定",
      changePasswordTitle: "ログインパスワードを変更",
      removePasswordTitle: "ログインパスワードを削除",
      setPasswordDescription:
        "起動時に MCPMate のロック解除に必要なパスワードを設定します。",
      changePasswordDescription:
        "MCPMate のロック解除に使用するパスワードを更新します。",
      clearDescription:
        "現在のパスワードを入力して保護を解除します。",
      changePasswordAction: "パスワードを変更",
      removePasswordAction: "パスワードを削除",
      currentPassword: "現在のパスワード",
      newPassword: "新しいパスワード",
      confirmPassword: "パスワード確認",
      passphraseLabel: "マスターパスワード",
      passphrasePlaceholder: "パスワードを入力…",
      passphraseConfirmPlaceholder: "パスワードを再入力…",
      passphraseRequired: "続行するにはマスターパスワードを入力してください。",
      passphraseMismatch: "パスワードが一致しません。",
      currentPassphraseRequired:
        "続行するには現在のマスターパスワードを入力してください。",
      passphraseSetupTitle: "マスターパスワードを設定",
      passphraseSetupDescription:
        "このパスワードはルート暗号化キーをラップします。平文では保存されません。「パスワード」暗号化モードから切り替える場合のみ再度必要です。",
      passphraseSetupContinue: "続行",
      currentPassphraseTitle: "現在のマスターパスワードを入力",
      currentPassphraseDescription:
        "ルートキーは現在のマスターパスワードでラップされています。暗号化モードを切り替える前に入力してキーのロックを解除してください。",
      confirmTitle: "セキュリティモードを切り替えますか？",
      confirmDescription:
        "Secure Store レコードを新しいプロバイダーへローテーションします。MCPMate は既存レコードを先に検証し、失敗した場合は現在のプロバイダーを正とします。",
      confirmPhraseLabel: "続行するには ROTATE SECRETS と入力してください",
      confirmPhrasePlaceholder: "ROTATE SECRETS",
      confirmPhraseDescription:
        "この移行では Secure Store のキーラップメタデータを書き換えます。確認フレーズは手入力してください。",
      confirmCancel: "キャンセル",
      confirmAction: "モードを切り替え",
      switching: "切り替え中…",
      saving: "保存中…",
      passwordSet: "パスワードを設定しました",
      passwordSetError: "パスワードの設定に失敗しました",
      passwordChanged: "パスワードを変更しました",
      passwordChangeError: "パスワードの変更に失敗しました",
      passwordCleared: "パスワードを削除しました",
      passwordClearError: "パスワードの削除に失敗しました",
      passwordRequired: "続行するにはパスワードを入力してください。",
      passwordChangeRequired: "現在のパスワードと新しいパスワードを入力してください。",
      passwordClearRequired:
        "保護を解除するには現在のパスワードを入力してください。",
      protectionScopeUpdated: "保護モードを更新しました",
      protectionScopeUpdateError: "保護モードの更新に失敗しました",
      switchMissingMode: "続行するには暗号化モードを選択してください。",
      encryptionPasswordRotated: "暗号化パスワードを更新しました",
      encryptionPasswordRotateError: "暗号化パスワードの更新に失敗しました",
      switchSuccess: "セキュリティモードを更新しました",
      switchError: "セキュリティモードの切り替えに失敗しました",
      issue: {
        title: "ストアの問題",
      },
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
  },
} as const;
