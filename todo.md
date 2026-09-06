# Pi Console — delivery checklist

## Step-by-step progress

Step 7 — DONE
Changed: reconciled this checklist into Implemented, Tested, Reviewed and Native / unverified sections; left unsupported native/live-provider items unchecked.
Verified: final `git diff --check` passed; final scope/status review found only the intended modified source/tests, `todo.md`, and user-provided untracked `plan.md`; no staging, commit, push, reset or clean was performed. No secrets were read or logged.
Remaining: native Tauri E2E, live provider/registry behavior, complete multi-provider credential lifecycle, Windows cleanup and daemonized/reparented descendants remain unverified as explicitly listed below.
Next: none; handoff is complete with the open native/live limitations preserved.

Step 6 — DONE WITH NATIVE LIMITATION
Changed: verification only; captured temporary browser acceptance screenshots under `/private/tmp/pi-console-visual-WiEyls/` for dark/light desktop and mobile Overview, Tasks and Settings states.
Verified: the preview at `http://127.0.0.1:1420/` rendered disconnected/backend-disabled content with no fabricated sessions or credentials; all 12 requested screenshots captured; desktop and mobile overflow checks returned true for Overview, Tasks and Settings in both themes. Existing browser coverage also verified keyboard/layout persistence, focusable controls, draft retention, flip/rotate and hide/show navigation/inspector, plus mobile panel control with no console errors.
Remaining: full native Tauri E2E is intentionally unverified. The repository has no isolated native desktop fixture for real session/model/streaming/git/terminal plumbing, and `pnpm tauri dev` was not launched against any user configuration. Rust command/PTY tests and disconnected browser coverage remain the safe verified boundary; native checklist items stay open.
Next: Step 7 — reconcile the handoff checklist, record exact implemented/tested/reviewed/native-unverified boundaries, run final diff/status safety checks, and do not commit or push.

Step 5 — DONE
Changed: verification only; no production code changes were needed for the full regression gate.
Verified: `pnpm check` passed with 0 errors and 0 warnings; `pnpm build` completed successfully; `pnpm test` passed 10/10; `cargo check --manifest-path src-tauri/Cargo.toml` passed with 6 existing dead-code warnings in `src-tauri/src/pi_config.rs`; `cargo test --manifest-path src-tauri/Cargo.toml` passed 11/11; `pnpm test:harness` passed against installed Pi `0.84.4` (`/opt/homebrew/bin/pi`). The real harness inherited only PATH/TMPDIR/LANG, used temporary HOME/agent/session paths, offline mode, no approvals/extensions/skills/templates/themes/context files, and sent no prompts or package installs.
Remaining: native Tauri desktop behavior and visual/native acceptance remain; the six existing Rust warnings remain; live provider/network behavior remains intentionally untested.
Next: Step 6 — capture browser desktop/mobile and dark/light visuals, verify disconnected/backend-disabled behavior and responsive interaction, and attempt only safe isolated native checks.

Step 4 — DONE
Changed: `src/lib/TerminalPanel.svelte` lifecycle was reviewed against the Tauri terminal commands; `tests/workspace.spec.ts` now covers start, hide/show retention and ID-scoped close; `src-tauri/src/terminal.rs` now cleans up exact-shell descendants before process-group close and includes an isolated background-child regression test.
Verified: `pnpm check` passed with 0 errors and 0 warnings; `pnpm test` passed 10/10, including the terminal browser fixture; `cargo test --manifest-path src-tauri/Cargo.toml` passed 11/11, including PTY echo/resize/exit and background-child cleanup. Input chunks remain bounded at 1024 bytes in the panel and 16 KiB per native write; native output is capped at 256 KiB with backpressure. The focused Rust process-cleanup test required host access because the sandbox blocks `/usr/bin/pgrep`.
Remaining: native Tauri UI lifecycle and Windows descendant cleanup remain unverified; `/usr/bin/pgrep` cleanup is Unix/macOS-specific and best-effort if a process daemonizes/reparents before close; no live prompt was sent and no secrets were read or logged.
Next: Step 5 — run the full build, Rust check/tests and isolated real harness with no live prompts or package installs.

Step 3 — DONE
Changed: `src/lib/pi/tasks.ts` rejects duplicate task IDs; `src/routes/+page.svelte` verifies the refreshed session identity before linking a prepared task and renders pending extension requests outside the session inspector; `src/lib/ExtensionRequest.svelte` sends a cancellation response on timeout; added task/protocol/browser coverage in `tests/workspace.spec.ts`, `tests/protocol.spec.ts` and `tests/harness.spec.ts`.
Verified: `pnpm check` passed with 0 errors and 0 warnings; `pnpm test` passed 10/10, covering local task create/status/filter/delete/export/reload, storage failure disclosure, prepare success/partial failure, no auto-send, duplicate records, navigation-visible approvals and timeout cancellation.
Remaining: native localStorage/Tauri task behavior and native extension plumbing remain unverified; no prompt was sent and no secrets were read or logged.
Next: Step 4 — inspect terminal start/input/output/resize/hide/show/close/exit races, bounded I/O and descendant cleanup.

Step 2 — DONE
Changed: `src/routes/settings/+page.svelte` now checks package-command success, rechecks the installed version, rejects unchanged/unknown post-update state, and only then reports a verified update; `tests/settings.spec.ts` covers a simulated failed update.
Verified: reviewed `updates.rs`, Pi package/update docs and installed `pi --version` (`0.84.4`) without running install/update commands; pinned npm and local/git/unknown sources remain explicitly unsupported rather than “current”; `pnpm check` passed with 0 errors and 0 warnings; `pnpm test` passed 8/8; `cargo test --manifest-path src-tauri/Cargo.toml` passed 10/10 with 6 existing dead-code warnings in `pi_config.rs`.
Remaining: live registry timeout/error behavior and native update commands remain unverified by design; no package was installed or updated and no secrets were read or logged.
Next: Step 3 — inspect local tasks, draft retention, navigation and extension-request reachability.

Step 1 — DONE
Changed: `src/routes/settings/+page.svelte` removes wildcard patterns when a matched model is disabled; added `tests/settings.spec.ts` for scoped-model saves, locked-write errors, OAuth/API-key display behavior and secret non-disclosure; extended `src-tauri/src/preferences.rs` temporary-file coverage to assert private file permissions.
Verified: reviewed Pi 0.84.4 settings/models/providers/RPC/package/security docs; traced settings UI → `api.ts` → Tauri commands → atomic locked JSON writes; `pnpm check` passed with 0 errors and 0 warnings; `pnpm test` passed 8/8; `cargo test --manifest-path src-tauri/Cargo.toml` passed 10/10 with 6 existing dead-code warnings in `pi_config.rs`.
Remaining: native settings behavior and broader provider/update coverage remain planned for later steps; no secrets were read or logged.
Next: Step 2 — inspect update detection, package/harness version sources and truthful unknown/error states.

Step 0 — DONE
Changed: review only; confirmed repository state, isolated browser/Rust fixtures, and the real-harness temporary-home/no-prompt constraints.
Verified: `pwd` = `/Users/morizkay/Developer/morizkay/pi-desktop`; branch `main`; HEAD `a6257d3f0c18ab2713c2ba49802efebdf95947f4`; `git status --short` showed only untracked user-provided `plan.md`; `pnpm check` passed with 0 errors and 0 warnings; `pnpm test` initially hit sandbox `listen EPERM` on `127.0.0.1:1420`, then passed 7/7 with the approved outside-sandbox run; `cargo test --manifest-path src-tauri/Cargo.toml` passed 10/10 with 6 existing dead-code warnings in `pi_config.rs`.
Remaining: no Step 0 failures; the six Rust warnings remain; real harness and native checks are later planned work; no secrets were read or logged.
Next: Step 1 — inspect settings, credential lifecycle, and model-scope paths before adding focused regression coverage.

## Handoff status

### Implemented
- [x] Settings model scope persistence, wildcard cleanup, credential write/remove boundaries and verified package-update reporting.
- [x] Local task persistence/export/filter/status/delete, duplicate-record rejection, session identity checks and navigation-visible extension approvals.
- [x] Terminal panel/native PTY lifecycle, bounded input/output, resize, explicit close, descendant cleanup and app-exit cleanup path.
- [x] Disconnected charcoal/amber console, settings, overview/task/activity views, responsive layout controls, draft persistence and sanitized Markdown.

### Tested
- [x] `pnpm check`, `pnpm build`, `pnpm test` (10/10), `cargo check --manifest-path src-tauri/Cargo.toml`, `cargo test --manifest-path src-tauri/Cargo.toml` (11/11), and `pnpm test:harness` (PASS).
- [x] Browser regressions cover disconnected preview, settings secrecy/scope behavior, tasks, protocol errors/cancellation, approvals, mobile overflow/panel controls, layout persistence and XSS sanitization.
- [x] Real Pi 0.84.4 RPC smoke used temporary HOME/config/session directories, offline mode and no prompts/package installs.
- [x] Dark/light desktop/mobile screenshots captured in `/private/tmp/pi-console-visual-WiEyls/`; all requested overflow checks passed.

### Reviewed
- [x] Tauri preferences atomic writes and private permissions; terminal input/output bounds and exact-ID operations; update source/version semantics; Pi 0.84.4 RPC/settings/models/packages documentation; real-harness isolation.
- [x] Final scope review: only the intended source/tests plus user-provided `plan.md` and progress `todo.md` are present; no staging, commit, push, reset or clean was performed.

### Native / unverified — intentionally open
- [ ] Full Tauri desktop E2E: real session loading/fork, model selection, streaming/stop, extension UI, git hunks and terminal UI.
- [ ] Live provider/registry behavior, native package/update commands, Windows descendant cleanup and daemonized/reparented terminal descendants.
- [ ] Complete multi-provider credential lifecycle and any claims requiring live user configuration.

## Constraints / corrections
- The earlier done claims for fork/swipe and full harness integration were unverified; rechecked in this slice.
- Chronological messages replace the old candy-bubble direction to match the supplied primary reference.
- No fictional accounts/plans/test totals, fake sessions, unsupported pause, or shell-based git endpoint.
- Preserve existing untracked files; never reset/clean/stage/commit/push. Tests must not change real ~/.pi configuration or send live prompts/install harness packages.
