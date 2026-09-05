import { test, expect } from "@playwright/test";

test("disconnected localhost renders, persists theme/layout/draft, and navigates cleanly", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("console", (e) => {
    if (e.type() === "error") errors.push(e.text());
  });
  await page.goto("/");
  await expect(
    page.getByRole("heading", {
      name: "A little focus. A lot of possibility.",
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Send", exact: false }),
  ).toBeDisabled();
  await expect(page.locator(".session-list button")).toHaveCount(0);
  await page
    .getByRole("textbox", { name: "Message Pi" })
    .fill("A safe unsent draft");
  await page.getByLabel("Theme", { exact: true }).selectOption("light");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await page.getByLabel("Theme", { exact: true }).selectOption("dark");
  await page
    .getByRole("button", { name: "Layout controls", exact: true })
    .click();
  await page.getByRole("button", { name: "Flip panels", exact: true }).click();
  await expect(page.locator(".console")).toHaveClass(/flipped/);
  await page.reload();
  await expect(page.locator(".console")).toHaveClass(/flipped/);
  await expect(page.getByRole("textbox", { name: "Message Pi" })).toHaveValue(
    "A safe unsent draft",
  );
  await page
    .getByRole("button", { name: "Layout controls", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Rotate inspector", exact: true })
    .click();
  await expect(page.locator(".console")).toHaveClass(/rotated/);
  await page.getByRole("button", { name: "Reset layout", exact: true }).click();
  await page
    .getByRole("button", { name: "Hide inspector", exact: true })
    .click();
  await expect(
    page.getByRole("complementary", { name: "Inspector" }),
  ).toHaveCount(0);
  await page
    .getByRole("button", { name: "Show inspector", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Layout controls", exact: true })
    .click();
  await page.screenshot({
    path: "test-results/console-dark.png",
    fullPage: true,
  });
  await page.getByRole("link", { name: "Settings", exact: false }).click();
  await expect(
    page.getByRole("heading", { name: "Settings", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Save API key", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByRole("button", { name: "Install", exact: true }),
  ).toBeDisabled();
  await page.screenshot({
    path: "test-results/settings-dark.png",
    fullPage: true,
  });
  await page.getByRole("link", { name: "Pi Console", exact: false }).click();
  await expect(page.getByRole("textbox", { name: "Message Pi" })).toHaveValue(
    "A safe unsent draft",
  );
  expect(errors).toEqual([]);
});

test("mobile layout has no horizontal overflow and panels remain controllable", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Layout controls", exact: true }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page
    .getByRole("button", { name: "Layout controls", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Show navigation", exact: true })
    .click();
  await expect(
    page.getByRole("complementary", { name: "Navigation" }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Hide navigation", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Show inspector", exact: true })
    .click();
  await expect(
    page.getByRole("complementary", { name: "Inspector" }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Hide inspector", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Layout controls", exact: true })
    .click();
  await page.screenshot({
    path: "test-results/console-mobile.png",
    fullPage: true,
  });
  expect(errors).toEqual([]);
});

test("markdown sanitization blocks runnable XSS while preserving highlighting", async ({
  page,
}) => {
  await page.goto("/");
  const result = await page.evaluate(async () => {
    const modulePath = "/src/lib/pi/markdown.ts";
    const { renderMarkdown } = await import(/* @vite-ignore */ modulePath);
    (window as any).__xss = 0;
    const html = renderMarkdown(
      '<img src=x onerror="window.__xss=1"><svg onload="window.__xss=2"></svg><script>window.__xss=3</script>\n[bad](javascript:window.__xss=4)\n<iframe srcdoc="<script>window.__xss=5</script>"></iframe>\n\n```typescript\nconst safe = "hello";\n```',
    );
    const el = document.createElement("div");
    el.innerHTML = html;
    document.body.appendChild(el);
    const bad = el.querySelector("a");
    bad?.click();
    await new Promise((r) => setTimeout(r, 100));
    const result = {
      xss: (window as any).__xss,
      dangerous: el.querySelectorAll(
        'script,img,svg,iframe,[onerror],[onload],[href^="javascript:"]',
      ).length,
      highlighted: !!el.querySelector(".hljs-keyword"),
      text: el.textContent,
    };
    el.remove();
    return result;
  });
  expect(result.xss).toBe(0);
  expect(result.dangerous).toBe(0);
  expect(result.highlighted).toBe(true);
  expect(result.text).toContain("const safe");
});
