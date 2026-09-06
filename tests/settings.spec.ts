import { test, expect } from "@playwright/test";

test("settings keeps credentials private and removes wildcard scope when disabling a model", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const w = window as any;
    w.isTauri = true;
    let enabledModels = ["anthropic/claude-*", "unknown/model"];
    w.__settingsCalls = [];
    w.__TAURI_INTERNALS__ = {
      invoke: async (command: string, args: any = {}) => {
        w.__settingsCalls.push({ command, args });
        if (command === "pi_read_settings")
          return { packages: ["npm:demo"], enabledModels };
        if (command === "pi_list_auth_providers")
          return [
            { id: "anthropic", authType: "oauth" },
            { id: "openai", authType: "api_key" },
          ];
        if (command === "pi_list_provider_ids") return ["anthropic", "openai"];
        if (command === "pi_list_models_store")
          return args.provider === "anthropic"
            ? [
                { id: "claude-sonnet", name: "Sonnet" },
                { id: "claude-opus", name: "Opus" },
              ]
            : [{ id: "gpt-4o", name: "GPT-4o" }];
        if (command === "pi_save_enabled_models") {
          if (!args.models.length) throw new Error("Configuration is locked");
          enabledModels = args.models;
          return;
        }
        if (command === "pi_set_api_key") return;
        if (command === "pi_check_update")
          return {
            source: args.source || "Pi harness",
            installed: "1.0.0",
            latest: "1.1.0",
            available: true,
            note: "New release verified on the public npm registry.",
          };
        if (command === "pi_package_update")
          return { success: false, stdout: "", stderr: "simulated update failed" };
        if (command === "pi_custom_providers") return [];
        throw new Error(`Unexpected settings fixture command ${command}`);
      },
    };
  });
  await page.goto("/settings");
  await expect(page.getByRole("button", { name: "All models" })).toBeEnabled();

  const models = page.getByRole("checkbox");
  await expect(models.first()).toBeChecked();
  await models.first().uncheck();
  const savedScope = await page.evaluate(() =>
    (window as any).__settingsCalls
      .filter((call: any) => call.command === "pi_save_enabled_models")
      .at(-1),
  );
  expect(savedScope.args.models).not.toContain("anthropic/claude-*");
  expect(savedScope.args.models).not.toContain("anthropic/claude-sonnet");
  expect(savedScope.args.models).toContain("anthropic/claude-opus");
  expect(savedScope.args.models).toContain("unknown/model");

  const credentials = page
    .locator("section")
    .filter({ has: page.getByRole("heading", { name: "Provider credentials" }) });
  await credentials.getByLabel("Provider ID", { exact: true }).last().fill("openai");
  await credentials.getByLabel("API key", { exact: true }).fill("sk-test-secret");
  await credentials.getByRole("button", { name: "Save API key", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("Key saved securely");
  expect(await page.locator("body").textContent()).not.toContain("sk-test-secret");
  const keyCall = await page.evaluate(() =>
    (window as any).__settingsCalls.find(
      (call: any) => call.command === "pi_set_api_key",
    ),
  );
  expect(keyCall.args.key).toBe("sk-test-secret");

  const packages = page
    .locator("section")
    .filter({ has: page.getByRole("heading", { name: "Extensions & packages" }) });
  await packages.getByRole("button", { name: "Check", exact: true }).click();
  await packages.getByRole("button", { name: "Update", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("simulated update failed");

  await page.getByRole("button", { name: "All models" }).click();
  await expect(page.getByRole("alert")).toContainText("Configuration is locked");
});
