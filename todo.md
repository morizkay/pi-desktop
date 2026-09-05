# Pi Console — delivery checklist

## Recovery / verification
- Implementation files survived the worker timeout; independent review is pending.
- Parent reran `pnpm check`, `pnpm build`, and `pnpm test`: passed (7 tests).
- Parent reran `cargo check` and `cargo test`: passed (6 tests); six unused-function warnings remain in `pi_config.rs`.
- These checks do not yet verify the complete native Tauri desktop flow. Items below remain unchecked until reviewed.

## Implemented delivery slice — awaiting review
- [ ] Reference-inspired charcoal/amber three-column shell, restrained glass, chronological separators, responsive typography/spacing
- [ ] Full-page settings, Auto/Light/Dark theme with live system preference
- [ ] Real sessions, search, workspace selection, date grouping, new session, fork using stable entry IDs
- [ ] Reply/copy keyboard controls and touch swipe-to-reply
- [ ] Sanitized Markdown and syntax highlighting with runnable XSS regression
- [ ] Real model/state/streaming, tool activity/log inspector, extension UI dialogs
- [ ] Safe drafts, command/control-enter, stop, reconnect/errors, near-bottom scrolling
- [ ] Git branch/status footer, hunk-based staged/worktree diffs and inline quoted hunk reply
- [ ] Persisted layout controls: hide/show navigation/inspector, flip, rotate
- [ ] Honest disconnected browser preview; no fabricated sessions or unsupported pause
- [ ] Protocol error/cancellation validation, listener cleanup/races, cwd/session identity, nonblocking RPC approvals/stop
- [ ] Svelte check/build, Rust check/tests, runnable frontend regressions and real browser smoke/screenshot

## Larger requested work — not completed
- [ ] Settings: model on/off toggles (scoped models)
- [ ] Settings: complete multi-provider configuration and API key lifecycle (existing single-key write UI is not complete management)
- [ ] Real extension version updates: compare installed/latest, notifications and verified update results
- [ ] Pi harness installed/latest version comparison and update notification
- [ ] Built-in PTY terminal (xterm + Tauri PTY), lifecycle and resize

## Constraints / corrections
- The earlier done claims for fork/swipe and full harness integration were unverified; rechecked in this slice.
- Chronological messages replace the old candy-bubble direction to match the supplied primary reference.
- No fictional accounts/plans/test totals, fake sessions, unsupported pause, or shell-based git endpoint.
- Preserve existing untracked files; never reset/clean/stage/commit/push. Tests must not change real ~/.pi configuration or send live prompts/install harness packages.
