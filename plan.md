# Pi Console — execution plan for Luna / Codex

## Start here

Continue the existing implementation, not a redesign from scratch. This handoff was created inside the **Pi coding-agent harness**. Luna or another coding agent can execute it with repository read/write access and a command runner. No previous conversation, Pi-specific agent tools or subagents are required. The application being built must still integrate with the **real Pi harness**; changing the development agent does not change the application's backend.

### Copy this instruction into Luna

> Read `plan.md` completely. Execute steps 0–7 in order, one step at a time. Continue the existing implementation; do not rewrite it or stop after producing another plan. For each step, inspect the listed files, fix confirmed problems, add focused regression checks, and run the specified checks. Record files changed, commands/results and the next step in `todo.md`. Never access real Pi credentials or modify real `~/.pi` in tests. Stop the affected step and report exact evidence if it cannot be completed safely. Do not commit or push. Start with step 0 now.

### How to work

- One writer, one step at a time. Do not launch parallel editing agents.
- A review item below is a risk to investigate, **not proof of a bug**. If behavior is already correct, verify it rather than replacing it.
- Before each fix, trace the UI → API → Rust/storage call path and its error path. Add a failing regression for the confirmed bug, then make the smallest fix.
- Use the existing Playwright and Rust test infrastructure; no new test framework.
- If a command fails, record its exact error and fix within that step. If a tool, permission or safe environment is missing, mark the step BLOCKED; do not silently switch runners or use live credentials. Independent safe steps may continue, but the blocked step stays open.
- After each step, reread changed files and inspect `git diff --check` and `git diff --stat`. Do not mark done from a tool's write-success message alone.
- To resume later, read `todo.md` and start at its first unfinished step. Recheck the working tree first.

Repository: `/Users/morizkay/Developer/morizkay/pi-desktop`

Verified at handoff, before creating this file:
- Branch: `main`
- HEAD: `a6257d3f0c18ab2713c2ba49802efebdf95947f4`
- Working tree: clean. Earlier notes about an unborn HEAD/untracked sources are obsolete.
- Stack: Svelte 5 runes, SvelteKit static SPA, Tauri 2/Rust, pnpm.

Read `todo.md`, but do not treat its status as current: it predates the latest settings, workspace pages and terminal implementation. Reconcile it with the code and verification below.

## Goal and guardrails

Finish the charcoal/amber Pi Console inspired by the user's ChatGPT-app reference screenshots: restrained liquid glass, Apple-style easing, chronological conversation, dashboard, workspaces, Tasks and Activity, full-page settings, real terminal and git inspector.

- Stay on Svelte 5; no framework rewrite or speculative abstractions.
- Clear, ADHD-friendly UI and concise communication; no emoji or unnecessary comments.
- Settings stays at `/settings`, not a slide-over.
- Real Pi sessions, models, tools and extension requests only. No fabricated fleet/accounts, test totals, approvals, unsupported pause or claimed task execution.
- The task board is explicitly local browser data; statuses are user labels, not harness execution state.
- Never modify real `~/.pi` during development/tests. Use isolated temporary homes/configuration. No live model prompts or harness package installs in tests.
- Preserve drafts, session identity and pending extension requests across navigation.
- No reset/clean/stage/commit/push unless requested. If later authorized, morizkay is sole commit author; no AI co-author trailers.
- Verify writes on disk. Report tested behavior separately from unverified native behavior.
- Proceed with implementation without another design-confirmation round.

## Implemented on disk

### Console and harness
- `src/routes/+page.svelte`: three-column console, grouped sessions, reply/copy/swipe, fork by stable entry ID, streaming, stop/reconnect, drafts, inspector, model picker, layout controls and footer.
- `src/routes/+layout.svelte`, `src/app.html`, `src/lib/ThemeControl.svelte`: theme variables and Auto/Light/Dark with no-flash setup.
- `src/lib/pi/{api,protocol,messages,markdown,diff}.ts`: Tauri API, response validation, session parsing, sanitized Markdown/highlighting, hunk parsing/quoting.
- `src/lib/ExtensionRequest.svelte`: real extension UI request responses.
- `src-tauri/src/{lib,pi_rpc,git}.rs`: RPC process lifecycle/commands and real git status/diffs.

### Latest settings slice
- `src/routes/settings/+page.svelte`: startup defaults, provider credentials, model toggles/search, custom endpoints/model IDs, package install/remove/check/update and harness update checks.
- `src-tauri/src/preferences.rs`: locked/atomic JSON changes, enabled models, API-key save/remove, custom providers.
- `src-tauri/src/updates.rs`: installed/latest checks with semver; pinned/git/local sources must not be falsely labeled current.
- `src/lib/pi/model-scope.ts`: picker filtering. Current session model remains visible even outside the enabled scope.
- Settings indicate that some changes require restarting Pi Console. OAuth remains managed through Pi login/logout rather than simulated in the UI.

### Latest workspace and terminal slice
- `src/lib/WorkspacePages.svelte`: Overview, Workspaces, Tasks and Activity views selected within the console, not separate routes. This preserves the parent session state.
- `src/lib/pi/tasks.ts`: validated local storage (`pi-console-workspaces-v1`) for task/workspace records.
- Tasks: create/filter/status/delete/export; prepare a real new session with a draft, never auto-send; open linked sessions.
- `src/lib/TerminalPanel.svelte`: xterm, explicit shell start, input/resize/output, close, hidden-panel persistence and workspace mismatch notice.
- `src-tauri/src/terminal.rs`: portable-pty shell in the active workspace, bounded output/backpressure, input queue, ID-scoped commands, resize/close/app-exit cleanup.
- Hiding the panel retains the shell; closing it terminates the managed shell/active job. Verify these lifecycle guarantees, especially descendants and error paths.

## Verification actually completed

Latest implementation passed:
- `pnpm check`: zero errors/warnings.
- `pnpm test`: 7 Playwright tests.
- `cargo check --manifest-path src-tauri/Cargo.toml`.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 10 tests, including real isolated `/bin/sh` PTY echo/resize/exit, atomic config patch, semver parsing/comparison, git fixtures and RPC concurrency.
- LSP reported no diagnostics for `src/routes/+page.svelte`.

Six unused-function warnings remain in `src-tauri/src/pi_config.rs`.

`pnpm build` passed for an earlier slice, **not yet rerun for the latest additions**. The existing 7 browser tests do not comprehensively cover new settings, local tasks or terminal UI. Native Tauri end-to-end and the latest real-harness script run remain unverified. Independent review did not complete.

## Execution plan

Dependencies: `0 baseline → 1 settings → 2 updates → 3 tasks/navigation → 4 terminal → 5 full checks → 6 visual/native checks → 7 handoff`.

Production paths to trace:
- Settings UI → `api.ts` → `preferences.rs` → Pi JSON files → refreshed model picker.
- Update button → `api.ts` → `updates.rs` → installed metadata + registry → verified/unknown/error UI state.
- Task board → `tasks.ts`/localStorage; prepare action → parent console → Pi new session → draft + saved session link. **No prompt send.**
- Terminal panel → ID-scoped Tauri commands → PTY → bounded output → xterm; close/exit → cleanup.
- Pi RPC event → parent session state → conversation/activity/extension request → response channel.

Test paths: Playwright → synthetic Tauri responses/browser storage; Rust → temporary config/git/PTY fixtures. Only the isolated harness smoke may start Pi, without live prompts. Native tests require separately verified isolation.

### 0. Establish the current baseline

**Read:** repository agent instructions, `package.json`, `playwright.config.ts`, `todo.md`, existing `tests/*.spec.ts`, `tests/real_harness.py`, and Rust tests/setup that touch filesystem or processes.

**Do:**
1. Run `pwd`, `git status --short`, `git branch --show-current`, `git rev-parse HEAD`. Treat the handoff snapshot above as historical; do not reset to it.
2. Verify tests use synthetic data/temp directories before running them. Do not read real `auth.json` to inspect configuration.
3. Run `pnpm check`, `pnpm test`, and `cargo test --manifest-path src-tauri/Cargo.toml`.
4. Add a step 0–7 progress section to `todo.md`, keeping its existing requirements.

**Done when:** baseline results and any existing failures are recorded. Missing dependencies/tools are blockers to report, not permission to install Pi packages or use a different agent runner.

### 1. Verify settings, credentials and model scope

**Read:** `src/routes/settings/+page.svelte`, `src/lib/pi/{api,model-scope}.ts`, `src/routes/+page.svelte`, `src-tauri/src/{preferences,pi_config,lib}.rs`, relevant Pi settings/models/auth documentation.

**Do:**
- Validate Pi-compatible locking and atomic writes; malformed JSON must not be overwritten, unrelated fields/model metadata must survive, and permissions must protect secrets.
- Confirm credentials never return to the browser or logs, and OAuth cannot be accidentally replaced.
- Check scope patterns/thinking suffixes, unknown IDs, current model visibility and empty-scope/all-models behavior. Toggling one model must not unexpectedly enable others.
- Add temporary-file Rust checks and synthetic browser coverage for saves, failures and model toggles. Reuse the IPC fixture pattern in `tests/harness.spec.ts`.

**Check:** `pnpm check`, `pnpm test`, `cargo test --manifest-path src-tauri/Cargo.toml`.

**Done when:** the above invariants have executable evidence and settings failures leave recoverable state, without touching real configuration.

### 2. Verify update detection and truthful results

**Read:** `src-tauri/src/updates.rs`, package commands in `src-tauri/src/{pi_config,lib}.rs`, settings update UI and its API helpers.

**Do:** verify installed-version discovery, scoped names, pins, prereleases, unknown/git/local sources, network timeout/error states and installed-version recheck after an update command. Add deterministic fixtures; do not actually install/update harness packages.

**Check:** `pnpm check`, `pnpm test`, `cargo test --manifest-path src-tauri/Cargo.toml`.

**Done when:** unknown/error is never “up to date,” and command completion alone is never proof of a successful version change.

### 3. Verify tasks, drafts and navigation

**Read:** `src/lib/WorkspacePages.svelte`, `src/lib/pi/tasks.ts`, task preparation/draft/navigation/request handling in `src/routes/+page.svelte`, `src/lib/ExtensionRequest.svelte`.

**Do:**
- Cover local create/status/filter/reload/delete/export and malformed/duplicate records/storage failures. Never overwrite invalid saved data silently.
- Cover prepare-session success and partial failure (new session succeeds, refresh fails). Preserve drafts and link only a verified session; never auto-send a prompt.
- Cover draft retention across views and pending extension requests during navigation. Requests must remain visible/actionable, not stranded in a hidden inspector.
- Add browser fixtures asserting these paths, including that preparation makes no `pi_prompt` call.

**Check:** `pnpm check`, `pnpm test`.

**Done when:** local tasks stay honestly labeled, drafts survive navigation/failure, and approvals remain reachable.

### 4. Verify terminal lifecycle and bounded I/O

**Read:** `src/lib/TerminalPanel.svelte`, terminal/keyboard/navigation handling in `src/routes/+page.svelte`, terminal API helpers, `src-tauri/src/{terminal,lib}.rs`.

**Do:**
- Trace start → input/output → resize → hide/show → close/exit. Cover close/start/poll/input races, natural exit/restart and workspace changes.
- Verify output backpressure and large-paste limits; failed/partial input must be disclosed. Terminal keyboard events must not send a chat draft or stop Pi.
- Check foreground/background child cleanup, PID reuse, route teardown and app exit. Do not claim all descendants terminate without proving it.
- Add synthetic terminal browser tests and isolated `/bin/sh` Rust lifecycle tests; never source the user's real shell startup files in tests.

**Check:** `pnpm check`, `pnpm test`, `cargo test --manifest-path src-tauri/Cargo.toml`.

**Done when:** hiding retains the shell, explicit close cleans up managed processes, queues remain bounded and lifecycle/error regressions pass. Record unsupported cases explicitly.

### 5. Run the full regression gate

```sh
pnpm check
pnpm build
pnpm test
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
pnpm test:harness
```

Read `tests/real_harness.py` before execution and verify its isolation. Do not run `pnpm tauri dev` blindly against the user's real Pi configuration; establish a safe temporary-home/native test environment first.

Installed Pi documentation, if still available:
`/opt/homebrew/Cellar/pi-coding-agent/0.84.4/libexec/lib/node_modules/@earendil-works/pi-coding-agent/`
Read `README.md` and relevant `docs/{rpc,settings,models,packages}.md`; follow related documentation as needed. Verify the installed version/path rather than assuming it remains unchanged.

### 6. Native and visual acceptance
- Browser preview at `http://localhost:1420/` renders without Tauri, never displays fake connected data and disables backend-only actions.
- Capture desktop/mobile and dark/light screenshots of console, settings, overview and task board. Check scrolling, overflow, keyboard/focus and panel flip/rotate/hide combinations.
- In an isolated native app, exercise real session loading/fork, model selection, streaming/stop/extension response plumbing where safely testable without live prompts, git hunks and terminal lifecycle.
- Original screenshots were temporary clipboard files and may be gone. Existing styling is the implemented reference interpretation; don't invent a different visual direction.
- If native testing cannot be safely performed, report the precise blocker and leave those checklist items open.

### 7. Close the handoff honestly
- Update `todo.md` to separate implemented, tested, reviewed and native-unverified items. Leave blocked checks unchecked.
- Record exact commands/results and remaining limitations; do not claim full multi-provider/update/native coverage from compilation alone.
- Remove obsolete helpers only after confirming they have no callers; rerun affected checks if removing code.
- Finish with `git diff --check`, inspect changed-file scope, and do not commit/push.

Use this small checkpoint format in `todo.md` after every step:

```text
Step N — DONE / IN PROGRESS / BLOCKED
Changed: file paths and one-line behavior change (or “review only”).
Verified: exact commands, pass/fail and test counts where available.
Remaining: exact limitation/error; no secrets in logs.
Next: step number and first action.
```

Final response: summarize changes, passing checks and remaining blockers. A safe native-testing blocker is acceptable to report; presenting it as completed is not.

## Prior delegation failures

Earlier automation hit a worker timeout, Codex quota, a native Cursor model/provider mismatch and finally `External CLI parser did not produce a terminal state`. Independent review never completed. Subsequent direct work is now present in the repository; do not assume an old worker is still making progress.

If using Pi subagents again, inspect run status before retrying and follow the same-protocol recovery rules. Never silently switch execution modes after a child infrastructure failure. The user's move to Codex here is explicit; this file is the cross-harness handoff, not a request to resurrect failed workers.
