import { test, expect } from "@playwright/test";

test("local task board persists, filters, exports and prepares without prompting", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const w = window as any;
    w.isTauri = true;
    let sessionId = "session";
    let sessionPath = "/fixture/session.jsonl";
    let sessionNumber = 0;
    let failRefresh = false;
    const callbacks = new Map<number, (event: any) => void>();
    const listeners = new Map<string, number>();
    w.__calls = [];
    w.__setFailRefresh = () => (failRefresh = true);
    w.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener: (name: string) => listeners.delete(name),
    };
    w.__TAURI_INTERNALS__ = {
      transformCallback: (callback: any) => {
        const id = callbacks.size + 1;
        callbacks.set(id, callback);
        return id;
      },
      invoke: async (command: string, args: any = {}) => {
        w.__calls.push({ command, args });
        if (command === "plugin:event|listen") {
          listeners.set(args.event, args.handler);
          return args.handler;
        }
        if (command === "plugin:event|unlisten") return;
        if (command === "pi_get_cwd") return "/fixture/project";
        if (command === "pi_read_settings") return { packages: [], enabledModels: [] };
        if (command === "pi_list_sessions")
          return [
            {
              path: sessionPath,
              id: sessionId,
              cwd: "/fixture/project",
              title: "Fixture session",
              updatedAt: "2026-01-01T00:00:00.000Z",
            },
          ];
        if (command === "pi_git_status")
          return { branch: "fixture", root: "/fixture/project", files: [] };
        if (command === "pi_rpc") {
          const type = args.command.type;
          if (failRefresh && type === "get_state") throw new Error("partial refresh failed");
          let data: any = {};
          if (type === "get_state")
            data = {
              sessionId,
              sessionFile: sessionPath,
              model: { provider: "fixture", id: "model" },
              thinkingLevel: "low",
              isStreaming: false,
            };
          if (type === "get_entries") data = { leafId: "root", entries: [] };
          if (type === "get_messages") data = { messages: [] };
          if (type === "get_available_models")
            data = { models: [{ provider: "fixture", id: "model", name: "Fixture model" }] };
          if (type === "get_session_stats") data = {};
          return { type: "response", success: true, data };
        }
        if (command === "pi_new_session") {
          sessionNumber += 1;
          sessionId = `session-${sessionNumber}`;
          sessionPath = `/fixture/session-${sessionNumber}.jsonl`;
          return { type: "response", success: true, data: { cancelled: false } };
        }
        if (command === "pi_prompt") return { type: "response", success: true };
        if (command === "pi_extension_response") return;
        if (command === "terminal_start") return { id: 41, cwd: "/fixture/project" };
        if (command === "terminal_poll") return { bytes: [], exited: false };
        if (command === "terminal_write" || command === "terminal_resize" || command === "terminal_close") return;
        throw new Error(`Unexpected workspace fixture command ${command}`);
      },
    };
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Toggle terminal", exact: true }).click();
  await page.getByRole("button", { name: "Start shell", exact: true }).click();
  await expect(page.getByText("Running", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Toggle terminal", exact: true }).click();
  expect(
    await page.evaluate(
      () => (window as any).__calls.filter((c: any) => c.command === "terminal_close").length,
    ),
  ).toBe(0);
  await page.getByRole("button", { name: "Toggle terminal", exact: true }).click();
  page.once("dialog", (dialog) => dialog.accept());
  await page.getByRole("button", { name: "Close terminal", exact: true }).click();
  await expect
    .poll(() =>
      page.evaluate(
        () => (window as any).__calls.filter((c: any) => c.command === "terminal_close").length,
      ),
    )
    .toBe(1);
  await page.getByRole("button", { name: "Tasks", exact: false }).click();
  await expect(page.getByRole("heading", { name: "Tasks and activity" })).toBeVisible();

  await page.getByLabel("Task title", { exact: true }).fill("Build feature");
  await page.getByLabel("Task workspace", { exact: true }).fill("/fixture/project");
  await page.getByRole("button", { name: "Add task", exact: true }).click();
  await expect(page.getByText("Build feature", { exact: true })).toBeVisible();
  await page
    .getByRole("combobox", { name: "Status for Build feature" })
    .selectOption("Completed");
  await page.getByLabel("Search tasks", { exact: true }).fill("feature");
  await expect(page.getByText("Build feature", { exact: true })).toBeVisible();
  const downloadPromise = page.waitForEvent("download");
  await page.getByRole("button", { name: "Export", exact: true }).click();
  expect((await downloadPromise).suggestedFilename()).toBe("pi-console-tasks.json");

  await page.getByLabel("Search tasks", { exact: true }).fill("");
  await page.reload();
  await page.getByRole("button", { name: "Tasks", exact: false }).click();
  await expect(page.getByText("Build feature", { exact: true })).toBeVisible();
  await page
    .getByRole("article")
    .filter({ hasText: "Build feature" })
    .getByRole("button", { name: "Prepare session", exact: true })
    .click();
  await expect(
    page
      .getByRole("article")
      .filter({ hasText: "Build feature" })
      .getByRole("button", { name: "Open session", exact: false }),
  ).toBeVisible();

  await page.getByLabel("Task title", { exact: true }).fill("Prepare failure");
  await page.getByLabel("Task workspace", { exact: true }).fill("/fixture/project");
  await page.getByRole("button", { name: "Add task", exact: true }).click();
  await page.evaluate(() => (window as any).__setFailRefresh());
  await page
    .getByRole("article")
    .filter({ hasText: "Prepare failure" })
    .getByRole("button", { name: "Prepare session", exact: true })
    .click();
  await expect(page.getByRole("alert")).toContainText("partial refresh failed");
  await expect(
    page
      .getByRole("article")
      .filter({ hasText: "Prepare failure" })
      .getByRole("button", { name: "Prepare session", exact: true }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => (window as any).__calls.filter((c: any) => c.command === "pi_prompt").length,
    ),
  ).toBe(0);

  await page.evaluate(() => {
    const original = Storage.prototype.setItem;
    Storage.prototype.setItem = function (key: string, value: string) {
      if (key === "pi-console-workspaces-v1") throw new Error("storage disabled");
      return original.call(this, key, value);
    };
  });
  await page.getByLabel("Task title", { exact: true }).fill("Storage warning");
  await page.getByLabel("Task workspace", { exact: true }).fill("/fixture/project");
  await page.getByRole("button", { name: "Add task", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("only in memory");

  page.once("dialog", (dialog) => dialog.accept());
  await page
    .getByRole("article")
    .filter({ hasText: "Build feature" })
    .getByRole("button", { name: "Delete Build feature", exact: true })
    .click();
  await expect(page.getByText("Build feature", { exact: true })).toHaveCount(0);
});
