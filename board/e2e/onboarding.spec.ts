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
              { name: "node", available: true, version: "v20.0.0", path: "/usr/bin/node" },
              { name: "npx", available: true, version: "10.0.0", path: "/usr/bin/npx" },
              { name: "bun", available: false },
              { name: "bunx", available: false },
              { name: "python3", available: true, version: "3.11.0", path: "/usr/bin/python3" },
              { name: "uv", available: false },
              { name: "uvx", available: false },
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
        return ok({
          success: true,
          data: {
            total: clients.length,
            client: clients,
          },
        });

      case "/api/client/update":
      case "/api/client/manage/approve":
        return ok({ success: true, data: { ok: true } });

      case "/api/mcp/servers/import":
        return ok({ success: true, imported: scanCandidates.length });

      case "/api/mcp/servers":
        return ok({ success: true, servers: [] });

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

  await page.getByRole("button", { name: /English/i }).click();
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
