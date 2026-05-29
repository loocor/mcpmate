import { expect, test, type Page } from "@playwright/test";

declare global {
  interface Window {
    __mcpmateTestWebSocketUrls: string[];
  }
}

type OnboardingMockOptions = {
  completed?: boolean;
  clients?: Array<{
    identifier: string;
    display_name: string;
    detected: boolean;
    config_path?: string;
    config_file_parse_effective?: unknown;
    config_mode?: string;
    writable_config?: boolean;
    approval_status?: string;
  }>;
  scanCandidates?: Array<{
    key: string;
    name: string;
    kind: string;
    command?: string;
    args?: string[];
    env?: Record<string, string>;
    source_clients?: string[];
    source_client_ids?: string[];
  }>;
  onClientUpdate?: (payload: unknown) => void;
  onServerImport?: (payload: unknown) => void;
};

type AdminDiscoveryMockOptions = {
  clients?: unknown[];
  servers?: unknown[];
  onRequest?: (url: URL) => void;
};

async function installReadyWebSocket(page: Page): Promise<void> {
  await page.addInitScript(() => {
    class MockWebSocket {
      url: string;
      readyState = 1;
      onopen: ((event: Event) => void) | null = null;
      onmessage: ((event: MessageEvent) => void) | null = null;
      onclose: ((event: CloseEvent) => void) | null = null;
      onerror: ((event: Event) => void) | null = null;

      constructor(url: string) {
        this.url = url;
        queueMicrotask(() => {
          this.onopen?.(new Event("open"));
          this.onmessage?.(
            new MessageEvent("message", {
              data: JSON.stringify({ type: "ready", status: "ok" }),
            }),
          );
        });
      }

      close(): void {
        this.onclose?.(new CloseEvent("close"));
      }

      send(_data: string): void {}
      addEventListener(): void {}
      removeEventListener(): void {}
      dispatchEvent(): boolean {
        return true;
      }
    }

    // @ts-expect-error test override
    window.WebSocket = MockWebSocket;
  });
}

async function installOnboardingApiMocks(
  page: Page,
  options: OnboardingMockOptions = {},
): Promise<void> {
  let completed = options.completed ?? false;
  const clients =
    options.clients ??
    [
      {
        identifier: "claude-desktop",
        display_name: "Claude Desktop",
        detected: true,
        config_path: "/tmp/claude_desktop_config.json",
        config_file_parse_effective: null,
        config_mode: "transparent",
      },
    ];
  const scanCandidates =
    options.scanCandidates ??
    [
      {
        key: "stdio:test-server",
        name: "test-server",
        kind: "stdio",
        command: "node",
        args: ["server.js"],
        env: {},
        source_clients: ["Claude Desktop"],
        source_client_ids: ["claude-desktop"],
      },
    ];

  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    const { pathname } = url;

    const ok = (data: unknown) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(data),
      });

    switch (pathname) {
      case "/api/onboarding/status":
        return ok({
          success: true,
          data: {
            completed,
            servers_count: 0,
            clients_count: 0,
          },
        });

      case "/api/onboarding/complete":
        completed = true;
        return ok({ success: true, data: { ok: true } });

      case "/api/onboarding/reset":
        completed = false;
        return ok({ success: true, data: { ok: true } });

      case "/api/onboarding/runtime-check":
        return ok({
          success: true,
          data: {
            runtimes: [
              { name: "node", available: true, version: "v20.0.0", path: "/usr/bin/node", source: "system" },
              { name: "npx", available: true, version: "10.0.0", path: "/usr/bin/npx", source: "system" },
              { name: "bun", available: true, version: "1.3.0", path: "/usr/bin/bun", source: "system" },
              { name: "bunx", available: true, version: "1.3.0", path: "/usr/bin/bunx", source: "system" },
              { name: "python3", available: true, version: "3.11.0", path: "/usr/bin/python3", source: "system" },
              { name: "uv", available: true, version: "0.8.0", path: "/usr/bin/uv", source: "system" },
              { name: "uvx", available: true, version: "0.8.0", path: "/usr/bin/uvx", source: "system" },
            ],
            has_js_runtime: true,
            has_python_runtime: true,
          },
        });

      case "/api/onboarding/server-scan":
        return ok({
          success: true,
          data: {
            candidates: scanCandidates,
            errors: [],
          },
        });

      case "/api/client/list":
      case "/api/client/detect":
        return ok({
          success: true,
          data: {
            total: clients.length,
            client: clients,
          },
        });

      case "/api/client/update":
        options.onClientUpdate?.(await request.postDataJSON());
        return ok({ success: true, data: { ok: true } });

      case "/api/client/manage/approve":
        return ok({ success: true, data: { ok: true } });

      case "/api/mcp/servers/import":
        options.onServerImport?.(await request.postDataJSON());
        return ok({
          success: true,
          data: {
            imported_count: scanCandidates.length,
            imported_servers: scanCandidates.map((candidate) => candidate.name),
            skipped_count: 0,
            skipped_servers: [],
            failed_count: 0,
            failed_servers: [],
            error_details: null,
          },
        });

      case "/api/mcp/servers":
        return ok({ success: true, servers: [] });

      case "/api/mcp/profile/list":
        return ok({
          success: true,
          data: {
            total: 1,
            timestamp: new Date().toISOString(),
            profile: [
              {
                id: "default-anchor",
                name: "Default",
                profile_type: "default",
                multi_select: true,
                priority: 0,
                is_active: true,
                is_default: true,
                role: "default_anchor",
                allowed_operations: [],
              },
            ],
          },
        });

      case "/api/system/status":
        return ok({ status: "running", uptime: 120, version: "test" });

      case "/api/system/readiness":
        return ok({ type: "ready", status: "ok" });

      case "/api/system/metrics":
        return ok({
          timestamp: new Date().toISOString(),
          cpu_usage_percent: 10,
          cpu_usage: 10,
          system_cpu_usage: 20,
          memory_usage: 100_000_000,
          memory_usage_bytes: 100_000_000,
          system_memory_usage: 200_000_000,
          system_memory_total: 1_000_000_000,
        });

      case "/api/config/suits":
        return ok({ suits: [] });

      case "/api/audit/list":
        return ok({ success: true, data: { events: [], next_cursor: null } });

      default:
        return ok({ success: true });
    }
  });
}

async function installAdminDiscoveryMocks(
  page: Page,
  options: AdminDiscoveryMockOptions = {},
): Promise<void> {
  await page.route("**/discovery/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    options.onRequest?.(url);

    const body = url.pathname.endsWith("/clients")
      ? {
          schemaVersion: "test",
          generatedAt: new Date().toISOString(),
          clients: options.clients ?? [],
          metadata: {},
        }
      : {
          schemaVersion: "test",
          generatedAt: new Date().toISOString(),
          servers: options.servers ?? [],
          metadata: {},
        };

    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(body),
      headers: {
        "access-control-allow-origin": "*",
      },
    });
  });
}

async function installTauriPlatformMock(page: Page, platform: "macos" | "windows" | "linux"): Promise<void> {
  await page.addInitScript((value) => {
    const w = window as unknown as Record<string, unknown>;
    w.__MCPMATE_IS_TAURI__ = true;
    w.__TAURI_INTERNALS__ = {
      invoke: async (command: string) => {
        if (command === "mcp_shell_read_platform") {
          return value;
        }
        if (command === "mcp_shell_read_core_source") {
          return {};
        }
        return null;
      },
    };
  }, platform);
}

async function startOnboardingWizard(page: Page): Promise<void> {
  await page.getByRole("checkbox", { name: /Allow scanning local runtimes/i }).check();
  await page.getByRole("button", { name: /Get Started/i }).click();
}

test("first-time user entering / gets redirected to onboarding", async ({ page }) => {
  await installReadyWebSocket(page);
  await installOnboardingApiMocks(page, { completed: false });

  await page.goto("/");
  await expect(page).toHaveURL(/\/onboarding$/);
  await expect(page.getByText("Welcome to MCPMate")).toBeVisible();
});

test("completed user does not get redirected to onboarding", async ({ page }) => {
  await installReadyWebSocket(page);
  await installOnboardingApiMocks(page, { completed: true });

  await page.goto("/");
  await expect(page).toHaveURL(/\/$/);
  await expect(page.getByText("System Status")).toBeVisible();
});

test("backend readiness gate uses HTTP without opening websocket", async ({ page }) => {
  await page.addInitScript(() => {
    const NativeWebSocket = window.WebSocket;
    window.__mcpmateTestWebSocketUrls = [];

    class RecordingWebSocket extends NativeWebSocket {
      constructor(url: string | URL, protocols?: string | string[]) {
        const urlText = String(url);
        window.__mcpmateTestWebSocketUrls.push(urlText);
        if (urlText.includes("/ws/readiness")) {
          throw new Error(`Unexpected readiness websocket connection to ${urlText}`);
        }
        super(url, protocols);
      }
    }

    // @ts-expect-error test override
    window.WebSocket = RecordingWebSocket;
  });
  await installOnboardingApiMocks(page, { completed: true });

  await page.goto("/");
  await expect(page).toHaveURL(/\/$/);
  await expect(page.getByText("System Status")).toBeVisible();
  await expect
    .poll(() =>
      page.evaluate(() =>
        window.__mcpmateTestWebSocketUrls.some((url) =>
          url.includes("/ws/readiness"),
        ),
      ),
    )
    .toBe(false);
});

test("onboarding wizard completes and stays out after completion", async ({ page }) => {
  await installReadyWebSocket(page);
  await installOnboardingApiMocks(page, { completed: false });

  await page.goto("/onboarding");
  await expect(page.getByText("Welcome to MCPMate")).toBeVisible();

  await startOnboardingWizard(page);
  await expect(page.getByText("Check Your Environment")).toBeVisible();

  await page.getByRole("button", { name: /^Next$/ }).click();
  await expect(page.getByText("Detected MCP Clients")).toBeVisible();

  await page.getByRole("button", { name: /Claude Desktop/ }).click();
  await page.getByRole("button", { name: /^Next$/ }).click();
  await expect(page.getByText("Import Existing Servers")).toBeVisible();

  await page.getByRole("button", { name: /test-server/ }).click();
  await page.getByRole("button", { name: /^Next$/ }).click();

  await expect(page.getByText("Join the Community")).toBeVisible();
  await page.getByRole("button", { name: /Finish Setup/i }).click();

  await expect(page).toHaveURL(/\/$/);
  await page.goto("/");
  await expect(page).toHaveURL(/\/$/);
  await expect(page.getByText("System Status")).toBeVisible();
});

test("onboarding uses client presets when local detection is empty", async ({ page }) => {
  const adminRequests: URL[] = [];
  const clientUpdates: unknown[] = [];

  await installReadyWebSocket(page);
  await installTauriPlatformMock(page, "macos");
  await installAdminDiscoveryMocks(page, {
    clients: [
      {
        identifier: "cursor-desktop",
        displayName: "Cursor",
        description: "AI code editor",
        links: {
          homepage: "https://cursor.com",
          docs: "https://docs.cursor.com",
          support: "https://support.cursor.com",
        },
        icon: {
          url: "https://example.com/cursor.png",
        },
        config: {
          kind: "file",
          file: {
            format: "json",
            paths: {
              macos: "~/Library/Application Support/Cursor/mcp.json",
              windows: "%APPDATA%\\Cursor\\mcp.json",
            },
            container: {
              type: "standard",
              keys: ["mcpServers"],
            },
          },
          transports: {
            stdio: {
              command_field: "command",
              args_field: "args",
              env_field: "env",
            },
          },
        },
      },
    ],
    servers: [],
    onRequest: (url) => adminRequests.push(url),
  });
  await installOnboardingApiMocks(page, {
    completed: false,
    clients: [],
    scanCandidates: [],
    onClientUpdate: (payload) => clientUpdates.push(payload),
  });

  await page.goto("/onboarding");
  await startOnboardingWizard(page);
  await page.getByRole("button", { name: /^Next$/ }).click();

  await expect(page.getByText("Detected MCP Clients")).toBeVisible();
  await expect(page.getByRole("button", { name: /Cursor/ })).toBeVisible();
  await expect(page.getByText("Preset client")).toBeVisible();
  await expect(page.getByText(/client presets can be applied directly/i)).toBeVisible();

  await page.getByRole("button", { name: /Cursor/ }).click();
  await page.getByRole("button", { name: /^Next$/ }).click();
  await page.getByRole("button", { name: /^Next$/ }).click();
  await page.getByRole("button", { name: /Finish Setup/i }).click();

  await expect.poll(() => clientUpdates.length).toBeGreaterThan(0);
  expect(adminRequests.some((url) => url.pathname === "/discovery/clients" && url.searchParams.get("random") === "6")).toBe(
    true,
  );
  expect(clientUpdates).toContainEqual({
    identifier: "cursor-desktop",
    display_name: "Cursor",
    config_file_state: "with_config_file",
    config_path: "~/Library/Application Support/Cursor/mcp.json",
    description: "AI code editor",
    homepage_url: "https://cursor.com",
    docs_url: "https://docs.cursor.com",
    support_url: "https://support.cursor.com",
    logo_url: "https://example.com/cursor.png",
    config_file_parse: {
      format: "json",
      container_type: "standard",
      container_keys: ["mcpServers"],
    },
    clear_config_file_parse: false,
    transports: {
      stdio: {
        command_field: "command",
        args_field: "args",
        env_field: "env",
      },
    },
    clear_transports: false,
  });
});

test("onboarding imports server presets through existing backend import API", async ({ page }) => {
  const adminRequests: URL[] = [];
  const serverImports: unknown[] = [];

  await installReadyWebSocket(page);
  await installAdminDiscoveryMocks(page, {
    clients: [],
    servers: [
      {
        id: "github",
        official: { title: "GitHub" },
        runtime: {
          install_config: {
            type: "stdio",
            command: "npx",
            args: ["-y", "@modelcontextprotocol/server-github"],
            env: { GITHUB_TOKEN: "${GITHUB_TOKEN}" },
          },
        },
      },
    ],
    onRequest: (url) => adminRequests.push(url),
  });
  await installOnboardingApiMocks(page, {
    completed: false,
    clients: [],
    scanCandidates: [],
    onServerImport: (payload) => serverImports.push(payload),
  });

  await page.goto("/onboarding");
  await startOnboardingWizard(page);
  await page.getByRole("button", { name: /^Next$/ }).click();
  await page.getByRole("button", { name: /^Next$/ }).click();

  await expect(page.getByText("Import Existing Servers")).toBeVisible();
  await expect(page.getByRole("button", { name: /GitHub/ })).toBeVisible();
  await expect(page.getByText(/server presets can be imported directly/i)).toBeVisible();

  await page.getByRole("button", { name: /GitHub/ }).click();
  await page.getByRole("button", { name: /^Next$/ }).click();
  await page.getByRole("button", { name: /Finish Setup/i }).click();

  await expect.poll(() => serverImports.length).toBeGreaterThan(0);
  expect(adminRequests.some((url) => url.pathname === "/discovery/servers" && url.searchParams.get("random") === "6")).toBe(
    true,
  );
  expect(serverImports).toContainEqual(
    expect.objectContaining({
      mcpServers: {
        GitHub: {
          type: "stdio",
          registry_server_id: "github",
          command: "npx",
          args: ["-y", "@modelcontextprotocol/server-github"],
          env: { GITHUB_TOKEN: "${GITHUB_TOKEN}" },
        },
      },
    }),
  );
});
