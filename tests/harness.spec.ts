import { test, expect } from "@playwright/test";

// Protocol fixture only: never starts Pi, reads ~/.pi, runs a tool, or sends a model prompt.
test("harness fixture validates fork IDs, approval channel, failed drafts and settled streaming", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("console", (e) => {
    if (e.type() === "error") errors.push(e.text());
  });
  await page.addInitScript(() => {
    const w = window as any;
    w.isTauri = true;
    let next = 0,
      forked = false,
      running = false,
      promptCount = 0;
    let finishFork: (() => void) | undefined;
    const callbacks = new Map<number, (e: any) => void>();
    const listeners = new Map<string, number>();
    w.__calls = [];
    w.__emit = (payload: any) =>
      callbacks.get(listeners.get("pi-rpc-event")!)?.({ payload });
    w.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener: (name: string) => listeners.delete(name),
    };
    w.__TAURI_INTERNALS__ = {
      transformCallback: (callback: any) => {
        callbacks.set(++next, callback);
        return next;
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
              path: "/fixture/session.jsonl",
              id: "session",
              cwd: "/fixture/project",
              title: "Protocol fixture",
              updatedAt: new Date().toISOString(),
            },
          ];
        if (command === "pi_git_status")
          return {
            branch: "fixture-branch",
            root: "/fixture/project",
            files: [{ path: "src/fixture.ts", status: " M" }],
          };
        if (command === "pi_git_diff") return "@@ -1 +1 @@\n-old\n+new\n";
        if (command === "pi_extension_response") {
          finishFork?.();
          return;
        }
        if (command === "pi_prompt") {
          promptCount++;
          if (promptCount === 1)
            return {
              type: "response",
              success: false,
              error: "Fixture prompt rejected",
            };
          running = true;
          w.__emit({ type: "agent_start" });
          return { type: "response", success: true };
        }
        if (command === "pi_abort") {
          running = false;
          w.__emit({ type: "agent_settled" });
          return { type: "response", success: true };
        }
        if (command === "pi_rpc") {
          const type = args.command.type;
          let data: any = {};
          if (type === "get_state")
            data = {
              sessionId: forked ? "forked" : "session",
              sessionFile: forked
                ? "/fixture/fork.jsonl"
                : "/fixture/session.jsonl",
              model: { provider: "fixture", id: "model" },
              thinkingLevel: "low",
              isStreaming: running,
            };
          if (type === "get_entries")
            data = {
              leafId: forked ? "one" : "two",
              entries: [
                {
                  type: "message",
                  id: "one",
                  parentId: null,
                  message: {
                    role: "user",
                    content: "Duplicate prompt",
                    timestamp: 1000,
                  },
                },
                {
                  type: "message",
                  id: "two",
                  parentId: "one",
                  message: {
                    role: "user",
                    content: "Duplicate prompt",
                    timestamp: 2000,
                  },
                },
              ],
            };
          if (type === "get_messages") data = { messages: [] };
          if (type === "get_available_models")
            data = {
              models: [
                { provider: "fixture", id: "model", name: "Fixture model" },
              ],
            };
          if (type === "get_fork_messages")
            data = {
              messages: [
                { entryId: "one", text: "Duplicate prompt" },
                { entryId: "two", text: "Duplicate prompt" },
              ],
            };
          if (type === "fork") {
            w.__emit({
              type: "extension_ui_request",
              id: "approval",
              method: "confirm",
              title: "Fixture fork confirmation",
              message: "Approve this fixture fork?",
            });
            await new Promise<void>((resolve) => {
              finishFork = resolve;
            });
            forked = true;
            data = { text: "Duplicate prompt", cancelled: false };
          }
          return { type: "response", success: true, command: type, data };
        }
        throw new Error(`Unexpected fixture command ${command}`);
      },
    };
  });
  await page.goto("/");
  await expect(page.locator(".message")).toHaveCount(2);
  await expect(page.getByLabel("Model", { exact: true })).toHaveValue(
    "fixture/model",
  );
  await page.locator(".message").last().hover();
  await page.getByRole("button", { name: "Fork", exact: true }).last().click();
  await expect(
    page.getByRole("heading", { name: "Fixture fork confirmation" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Confirm", exact: true }).click();
  await expect(page.getByRole("textbox", { name: "Message Pi" })).toHaveValue(
    "Duplicate prompt",
  );
  const fork = await page.evaluate(() =>
    (window as any).__calls.find((c: any) => c.args.command?.type === "fork"),
  );
  expect(fork.args.command.entryId).toBe("two");
  const approval = await page.evaluate(() =>
    (window as any).__calls.find(
      (c: any) => c.command === "pi_extension_response",
    ),
  );
  expect(approval.args.response).toEqual({
    type: "extension_ui_response",
    id: "approval",
    confirmed: true,
  });
  await page.getByRole("button", { name: "Send", exact: false }).click();
  await expect(page.getByRole("alert")).toContainText(
    "Fixture prompt rejected",
  );
  await expect(page.getByRole("textbox", { name: "Message Pi" })).toHaveValue(
    "Duplicate prompt",
  );
  await page.getByRole("button", { name: "Send", exact: false }).click();
  await expect(page.getByRole("textbox", { name: "Message Pi" })).toHaveValue(
    "",
  );
  await page.evaluate(() => {
    const w = window as any;
    w.__emit({
      type: "message_start",
      message: { role: "assistant", content: [], timestamp: 3000 },
    });
    w.__emit({
      type: "message_update",
      assistantMessageEvent: {
        type: "text_delta",
        contentIndex: 0,
        delta: "**Streaming fixture**",
      },
    });
    w.__emit({ type: "agent_end", willRetry: true });
  });
  await expect(page.locator(".markdown strong")).toHaveText(
    "Streaming fixture",
  );
  await expect(
    page.getByRole("button", { name: "Stop", exact: false }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Stop", exact: false }).click();
  await expect(
    page.getByRole("button", { name: "Stop", exact: false }),
  ).toHaveCount(0);
  await page
    .getByRole("button", { name: "src/fixture.ts", exact: false })
    .click();
  await page.getByRole("button", { name: "Discuss", exact: false }).click();
  await expect(page.locator(".reply-bar")).toContainText(
    "src/fixture.ts (working tree)",
  );
  await page
    .getByRole("textbox", { name: "Message Pi" })
    .fill("Please review this hunk");
  await page
    .getByRole("textbox", { name: "Message Pi" })
    .press("Control+Enter");
  const prompt = await page.evaluate(() =>
    (window as any).__calls
      .filter((c: any) => c.command === "pi_prompt")
      .at(-1),
  );
  expect(prompt.args.message).toContain("> @@ -1 +1 @@");
  expect(prompt.args.message).toContain("Please review this hunk");

  await page.evaluate(() => {
    const w = window as any;
    w.__emit({
      type: "extension_ui_request",
      id: "pending-navigation",
      method: "confirm",
      title: "Pending navigation approval",
      message: "This must stay actionable outside the session inspector.",
    });
  });
  await expect(
    page.getByRole("heading", { name: "Pending navigation approval" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Tasks", exact: false }).click();
  const pending = page.locator('[aria-label="Pending extension requests"]');
  await expect(pending).toBeVisible();
  await pending.getByRole("button", { name: "Confirm", exact: true }).click();
  const pendingResponse = await page.evaluate(() =>
    (window as any).__calls.find(
      (c: any) =>
        c.command === "pi_extension_response" &&
        c.args.response.id === "pending-navigation",
    ),
  );
  expect(pendingResponse.args.response.confirmed).toBe(true);

  await page.evaluate(() => {
    (window as any).__emit({
      type: "extension_ui_request",
      id: "timed-out",
      method: "confirm",
      title: "Timed request",
      timeout: 20,
    });
  });
  await expect(page.getByRole("heading", { name: "Timed request" })).toBeVisible();
  await expect
    .poll(
      async () =>
        page.evaluate(
          () =>
            (window as any).__calls.find(
              (c: any) =>
                c.command === "pi_extension_response" &&
                c.args.response.id === "timed-out",
            )?.args.response,
        ),
      { timeout: 1000 },
    )
    .toMatchObject({ id: "timed-out", cancelled: true });
  expect(errors).toEqual([]);
});
