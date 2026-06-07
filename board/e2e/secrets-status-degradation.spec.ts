import { expect, test } from "@playwright/test";

/**
 * E2E: Secrets Store Status Degradation (Step 4 + Step 9)
 *
 * Covers: ready/unavailable/error states, alert banners, disabled buttons,
 * SecretPickerButton hiding, and status transitions.
 */

type SecretStoreStatusData = {
  status: string;
  provider?: {
    provider_id: string;
    provider_kind: string;
    provider_mode: string;
    security_level: string;
  };
  issue?: {
    reason_code: string;
    message: string;
  };
};

function statusReady(): SecretStoreStatusData {
  return {
    status: "ready",
    provider: {
      provider_id: "test-provider",
      provider_kind: "os_keychain",
      provider_mode: "auto",
      security_level: "hardware_backed",
    },
  };
}

function statusUnavailable(
  reasonCode = "provider_unavailable",
  message = "OS keychain not accessible",
): SecretStoreStatusData {
  return {
    status: "unavailable",
    issue: { reason_code: reasonCode, message },
  };
}

function ok(data: unknown) {
  return {
    status: 200,
    contentType: "application/json",
    body: JSON.stringify({ success: true, data }),
  };
}

async function installSecretsPageMocks(
  page: import("@playwright/test").Page,
  options: {
    storeStatus?: SecretStoreStatusData;
    secrets?: Array<{
      alias: string;
      placeholder: string;
      kind: string;
      label?: string;
      provider_id: string;
      provider_kind: string;
      version: number;
      used_by_count: number;
    }>;
  } = {},
) {
  const storeStatus = options.storeStatus ?? statusReady();
  const secrets = options.secrets ?? [];

  await page.route("**/api/**", async (route) => {
    const url = new URL(route.request().url());
    const { pathname } = url;

    switch (pathname) {
      case "/api/secrets/status":
        return route.fulfill(ok(storeStatus));
      case "/api/secrets/list":
        return route.fulfill(ok({ secrets }));
      case "/api/secrets/usages":
        return route.fulfill(ok({ usages: [] }));
      default:
        return route.fulfill(ok({}));
    }
  });
}

test.describe("Secrets store status degradation", () => {
  test("store ready → no alert, Add button enabled", async ({ page }) => {
    await installSecretsPageMocks(page, {
      storeStatus: statusReady(),
      secrets: [
        {
          alias: "github-token",
          placeholder: "[[secret:github-token]]",
          kind: "token",
          label: "GitHub PAT",
          provider_id: "dev",
          provider_kind: "development",
          version: 1,
          used_by_count: 2,
        },
      ],
    });

    await page.goto("/secrets");

    await expect(page.getByRole("alert")).toHaveCount(0);
    const addButton = page.getByRole("button", { name: /add secret/i });
    await expect(addButton).toBeEnabled();
    await expect(page.getByText("github-token")).toBeVisible();
  });

  test("store unavailable → alert banner, Add button disabled", async ({
    page,
  }) => {
    await installSecretsPageMocks(page, {
      storeStatus: statusUnavailable(
        "provider_unavailable",
        "OS keychain not accessible for testing",
      ),
      secrets: [
        {
          alias: "existing-token",
          placeholder: "[[secret:existing-token]]",
          kind: "token",
          provider_id: "dev",
          provider_kind: "development",
          version: 1,
          used_by_count: 0,
        },
      ],
    });

    await page.goto("/secrets");

    const alert = page.getByRole("alert");
    await expect(alert).toBeVisible();
    await expect(alert).toContainText("Secure store unavailable");
    await expect(alert).toContainText("OS keychain not accessible for testing");

    const addButton = page.getByRole("button", { name: /add secret/i });
    await expect(addButton).toBeDisabled();

    await expect(page.getByText("existing-token")).toBeVisible();
  });

  test("store unavailable + empty list → empty-state CTA disabled", async ({
    page,
  }) => {
    await installSecretsPageMocks(page, {
      storeStatus: statusUnavailable(),
      secrets: [],
    });

    await page.goto("/secrets");

    await expect(page.getByRole("alert")).toBeVisible();
    await expect(page.getByText(/no secrets stored/i)).toBeVisible();

    const ctaButton = page.getByRole("button", { name: /add first secret/i });
    await expect(ctaButton).toBeDisabled();
  });

  test("store unavailable → edit and delete buttons disabled", async ({
    page,
  }) => {
    await installSecretsPageMocks(page, {
      storeStatus: statusUnavailable(),
      secrets: [
        {
          alias: "my-token",
          placeholder: "[[secret:my-token]]",
          kind: "token",
          provider_id: "dev",
          provider_kind: "development",
          version: 1,
          used_by_count: 0,
        },
      ],
    });

    await page.goto("/secrets");

    await expect(page.getByRole("alert")).toBeVisible();

    const editButton = page.getByRole("button", { name: /edit secret/i });
    await expect(editButton).toBeDisabled();

    const deleteButton = page.getByRole("button", { name: /delete secret/i });
    await expect(deleteButton).toBeDisabled();
  });

  test("status API error → error alert shown, Add button disabled", async ({
    page,
  }) => {
    await page.route("**/api/**", async (route) => {
      const url = new URL(route.request().url());
      const { pathname } = url;

      if (pathname === "/api/secrets/status") {
        return route.fulfill({
          status: 500,
          contentType: "application/json",
          body: JSON.stringify({ success: false, error: "internal error" }),
        });
      }
      if (pathname === "/api/secrets/list") {
        return route.fulfill(ok({ secrets: [] }));
      }
      return route.fulfill(ok({}));
    });

    await page.goto("/secrets");

    const alert = page.getByRole("alert").first();
    await expect(alert).toBeVisible({ timeout: 10_000 });
    await expect(alert).toContainText(/store status|failed|error/i);

    const addButton = page.getByRole("button", { name: /add secret/i });
    await expect(addButton).toBeDisabled();
  });

  test("SecretPickerButton hidden when store unavailable", async ({
    page,
  }) => {
    await installSecretsPageMocks(page, {
      storeStatus: statusUnavailable(),
    });

    await page.goto("/install/manual");

    const pickerButton = page.getByRole("button", { name: /use secret/i });
    await expect(pickerButton).toHaveCount(0);
  });

  test("status transitions: ready → unavailable → ready", async ({ page }) => {
    let storeStatus: SecretStoreStatusData = statusReady();

    await page.route("**/api/**", async (route) => {
      const url = new URL(route.request().url());
      const { pathname } = url;

      if (pathname === "/api/secrets/status") {
        return route.fulfill(ok(storeStatus));
      }
      if (pathname === "/api/secrets/list") {
        return route.fulfill(ok({ secrets: [] }));
      }
      return route.fulfill(ok({}));
    });

    await page.goto("/secrets");
    await expect(page.getByRole("alert")).toHaveCount(0);
    await expect(
      page.getByRole("button", { name: /add secret/i }),
    ).toBeEnabled();

    storeStatus = statusUnavailable("provider_unavailable", "Store went down");
    await page.getByRole("button", { name: /refresh/i }).click();
    await expect(page.getByRole("alert")).toBeVisible({ timeout: 5_000 });
    await expect(
      page.getByRole("button", { name: /add secret/i }),
    ).toBeDisabled();

    storeStatus = statusReady();
    await page.getByRole("button", { name: /refresh/i }).click();
    await expect(page.getByRole("alert")).toHaveCount(0, { timeout: 5_000 });
    await expect(
      page.getByRole("button", { name: /add secret/i }),
    ).toBeEnabled();
  });
});
