<script lang="ts">
  import { onMount, tick } from "svelte";
  import { beforeNavigate } from "$app/navigation";
  import { listen } from "@tauri-apps/api/event";
  import ThemeControl from "$lib/ThemeControl.svelte";
  import WorkspacePages from "$lib/WorkspacePages.svelte";
  import TerminalPanel from "$lib/TerminalPanel.svelte";
  import { inModelScope } from "$lib/pi/model-scope";
  import ExtensionRequest from "$lib/ExtensionRequest.svelte";
  import {
    hasDesktop,
    piAbort,
    piGetCwd,
    piReadSettings,
    piListSessions,
    piNewSession,
    piSwitchSession,
    piPrompt,
    piFork,
    piRpc,
    piGitStatus,
    piGitDiff,
    type PiSessionSummary,
    type GitStatus,
  } from "$lib/pi/api";
  import {
    rpcData,
    record,
    parseModels,
    parseForkMessages,
    type PiModel,
    type JsonRecord,
  } from "$lib/pi/protocol";
  import {
    parseActiveEntries,
    chatMessage,
    messageTextFromPi,
    type ChatMessage,
  } from "$lib/pi/messages";
  import { renderMarkdown } from "$lib/pi/markdown";
  import { parseHunks, quoteHunk } from "$lib/pi/diff";

  let desktop = $state(false),
    connected = $state(false),
    connecting = $state(false),
    busy = $state(false),
    running = $state(false);
  let sessions = $state<PiSessionSummary[]>([]),
    messages = $state<ChatMessage[]>([]),
    models = $state<PiModel[]>([]);
  let cwd = $state(""),
    sessionId = $state(""),
    sessionPath = $state(""),
    sessionName = $state("");
  let modelKey = $state(""),
    thinking = $state(""),
    error = $state(""),
    notice = $state("");
  let draft = $state(""),
    quote = $state(""),
    streaming = $state(""),
    search = $state(""),
    workspaceFilter = $state("");
  let navTab = $state("sessions"),
    inspectorTab = $state("changes"),
    showNav = $state(true),
    showInspector = $state(true),
    showTerminal = $state(false),
    terminalActive = $state(false),
    flipped = $state(false),
    rotated = $state(false),
    layoutOpen = $state(false);
  let workspaceInput = $state(""),
    workspaceForm = $state(false),
    nearBottom = $state(true);
  let repo = $state<GitStatus | null>(null),
    gitError = $state(""),
    selectedFile = $state(""),
    staged = $state(false),
    patch = $state(""),
    diffError = $state("");
  let tools = $state<
      Array<{ id: string; name: string; status: string; detail: string }>
    >([]),
    logs = $state<string[]>([]);
  let requests = $state<JsonRecord[]>([]),
    extensionStatus = $state<Record<string, string>>({}),
    widgets = $state<Record<string, string>>({});
  let stats = $state<JsonRecord>({});
  let messagesEl: HTMLDivElement | undefined = $state(),
    composerEl: HTMLTextAreaElement | undefined = $state();
  let disposed = false,
    hydrating = false,
    buffered: unknown[] = [],
    refreshAgain = false,
    epoch = 0,
    diffEpoch = 0,
    eventId = 0;
  let initialized = $state(false);
  const workspaces = $derived([
    ...new Set([cwd, ...sessions.map((s) => s.cwd)].filter(Boolean)),
  ]);
  const filtered = $derived(
    sessions.filter(
      (s) =>
        (!workspaceFilter || s.cwd === workspaceFilter) &&
        `${s.title} ${s.cwd}`.toLowerCase().includes(search.toLowerCase()),
    ),
  );
  const hunks = $derived(parseHunks(patch));
  const stateLabel = $derived(
    !connected
      ? connecting
        ? "Connecting"
        : "Disconnected"
      : requests.length
        ? "Awaiting response"
        : running
          ? "Working"
          : "Ready",
  );
  const locked = $derived(!connected || busy || running);
  const project = $derived(
    cwd.split("/").filter(Boolean).pop() || "No workspace",
  );
  const draftKey = () => `pi-draft:${sessionId || "preview"}`;
  function saveDraft() {
    try {
      sessionStorage.setItem(draftKey(), JSON.stringify({ draft, quote }));
    } catch {
      notice =
        "Draft storage unavailable. Keep this window open to preserve your draft.";
    }
  }
  function restoreDraft() {
    try {
      const saved = JSON.parse(sessionStorage.getItem(draftKey()) || "{}");
      draft = typeof saved.draft === "string" ? saved.draft : "";
      quote = typeof saved.quote === "string" ? saved.quote : "";
    } catch {
      draft = "";
      quote = "";
    }
  }
  $effect(() => {
    draft;
    quote;
    if (initialized) saveDraft();
  });
  $effect(() => {
    const layout = { showNav, showInspector, showTerminal, flipped, rotated };
    if (initialized)
      try {
        localStorage.setItem("pi-layout", JSON.stringify(layout));
      } catch {
        /* Layout is still usable without storage. */
      }
  });
  $effect(() => {
    messages.length;
    streaming;
    if (nearBottom)
      void tick().then(() => {
        if (messagesEl && !disposed)
          messagesEl.scrollTop = messagesEl.scrollHeight;
      });
  });
  beforeNavigate(({ cancel }) => {
    saveDraft();
    if (busy || running || requests.length) {
      cancel();
      notice =
        "Stop the agent and resolve extension requests before leaving this session.";
    } else if (terminalActive && !confirm('Leaving this page closes the terminal and its active job. Continue?')) cancel();
  });
  function groupSessions() {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const groups = new Map<string, PiSessionSummary[]>();
    for (const s of filtered) {
      const age = today.getTime() - Date.parse(s.updatedAt);
      const label =
        age <= 0
          ? "Today"
          : age < 86400000
            ? "Yesterday"
            : age < 7 * 86400000
              ? "Previous 7 days"
              : "Older";
      groups.set(label, [...(groups.get(label) || []), s]);
    }
    return [...groups];
  }
  async function safely(action: () => Promise<unknown>) {
    error = "";
    try {
      await action();
    } catch (e) {
      if (!disposed) error = e instanceof Error ? e.message : String(e);
    }
  }
  function addReply(text: string) {
    navTab = 'sessions';
    quote = quote ? `${quote}\n\n${text}` : text;
    composerEl?.focus();
  }
  async function copy(text: string) {
    await safely(async () => {
      await navigator.clipboard.writeText(text);
      notice = "Copied to clipboard.";
    });
  }
  function swipeReply(node: HTMLElement, message: ChatMessage) {
    let start: { x: number; y: number } | undefined;
    const down = (e: PointerEvent) => {
      if (
        e.pointerType === "touch" &&
        !(e.target as HTMLElement).closest("button,a,pre")
      )
        start = { x: e.clientX, y: e.clientY };
    };
    const up = (e: PointerEvent) => {
      if (
        start &&
        e.clientX - start.x > 80 &&
        Math.abs(e.clientY - start.y) < 35
      )
        addReply(message.text);
      start = undefined;
    };
    const cancel = () => {
      start = undefined;
    };
    node.addEventListener("pointerdown", down);
    node.addEventListener("pointerup", up);
    node.addEventListener("pointercancel", cancel);
    return {
      destroy() {
        node.removeEventListener("pointerdown", down);
        node.removeEventListener("pointerup", up);
        node.removeEventListener("pointercancel", cancel);
      },
    };
  }
  async function refreshGit() {
    const current = ++diffEpoch;
    try {
      const result = await piGitStatus();
      if (!disposed && current === diffEpoch) {
        repo = result;
        gitError = "";
        if (
          selectedFile &&
          !result.files.some((f) => f.path === selectedFile)
        ) {
          selectedFile = "";
          patch = "";
        }
      }
    } catch (e) {
      if (!disposed && current === diffEpoch) {
        repo = null;
        gitError = String(e);
        patch = "";
      }
    }
  }
  async function openDiff(path = selectedFile, cached = staged) {
    selectedFile = path;
    staged = cached;
    patch = "";
    diffError = "";
    const current = ++diffEpoch;
    if (!path) return;
    try {
      const result = await piGitDiff(path, cached);
      if (current === diffEpoch && !disposed) patch = result;
    } catch (e) {
      if (current === diffEpoch && !disposed) diffError = String(e);
    }
  }
  async function refresh() {
    if (hydrating) {
      refreshAgain = true;
      return;
    }
    hydrating = true;
    const current = epoch;
    try {
      const state = rpcData(await piRpc({ type: "get_state" }));
      const entries = await piRpc({ type: "get_entries" });
      const history = rpcData(await piRpc({ type: "get_messages" }));
      const available = parseModels(
        await piRpc({ type: "get_available_models" }),
      );
      const preferences = await piReadSettings();
      const list = await piListSessions();
      const dir = await piGetCwd();
      const usage = rpcData(await piRpc({ type: "get_session_stats" }));
      if (disposed || current !== epoch) return;
      const nextId = String(state.sessionId || "");
      if (sessionId !== nextId) {
        const carryDraft = !sessionId || !connected;
        const previousDraft = draft,
          previousQuote = quote;
        saveDraft();
        sessionId = nextId;
        restoreDraft();
        if (carryDraft && (previousDraft || previousQuote)) {
          draft = draft ? `${draft}\n\n${previousDraft}` : previousDraft;
          quote = quote ? `${quote}\n\n${previousQuote}` : previousQuote;
        }
        tools = [];
        logs = [];
        selectedFile = "";
        patch = "";
        widgets = {};
        extensionStatus = {};
      }
      sessionPath = String(state.sessionFile || "");
      sessionName = String(state.sessionName || "");
      cwd = dir;
      running = state.isStreaming === true || state.isCompacting === true;
      thinking = String(state.thinkingLevel || "");
      const model = record(state.model);
      modelKey =
        model.provider && model.id ? `${model.provider}/${model.id}` : "";
      models = available.filter(m => `${m.provider}/${m.id}` === modelKey || inModelScope(m.provider, m.id, preferences.enabledModels));
      sessions = list;
      stats = usage;
      messages = parseActiveEntries(entries);
      connected = true;
      if (Array.isArray(history.messages))
        tools = history.messages
          .flatMap((m, i) => {
            const msg = record(m);
            return msg.role === "toolResult"
              ? [
                  {
                    id: String(msg.toolCallId || i),
                    name: String(msg.toolName || "tool"),
                    status: msg.isError ? "Error" : "Complete",
                    detail: messageTextFromPi(msg),
                  },
                ]
              : [];
          })
          .slice(-60);
      await refreshGit();
    } finally {
      hydrating = false;
      const queue = buffered;
      buffered = [];
      if (!disposed && current === epoch)
        for (const event of queue) handleEvent(event);
      if (refreshAgain && !disposed) {
        refreshAgain = false;
        void safely(refresh);
      }
    }
  }
  async function connect() {
    if (!desktop || connecting) return;
    connecting = true;
    await safely(async () => {
      await refresh();
      notice = "";
    });
    connecting = false;
  }
  async function changeSession(action: () => Promise<unknown>, fork = false) {
    if (locked) return;
    busy = true;
    saveDraft();
    await safely(async () => {
      const result = rpcData(await action());
      streaming = "";
      nearBottom = true;
      await refresh();
      if (fork && typeof result.text === "string") {
        draft = draft ? `${draft}\n\n${result.text}` : result.text;
        composerEl?.focus();
      }
      workspaceForm = false;
      navTab = 'sessions';
    });
    busy = false;
  }
  async function prepareTask(dir: string, title: string): Promise<string> {
    if (locked) throw new Error('Connect and stop the agent before preparing a task.');
    busy = true; saveDraft();
    try {
      await piNewSession(dir); streaming = ''; await refresh();
      draft = draft ? `${draft}\n\n${title}` : title;
      saveDraft();
      return sessionPath;
    } finally { busy = false; }
  }
  async function forkMessage(message: ChatMessage) {
    if (!message.entryId) return;
    await changeSession(async () => {
      const choices = parseForkMessages(
        await piRpc({ type: "get_fork_messages" }),
      );
      if (!choices.some((c) => c.entryId === message.entryId))
        throw new Error(
          "This message is no longer on the active branch. Refresh the session.",
        );
      return piFork(message.entryId!);
    }, true);
  }
  async function send() {
    if (locked || !draft.trim()) return;
    const submitted = draft,
      submittedQuote = quote;
    const text = `${
      quote
        ? quote
            .split("\n")
            .map((line) => `> ${line}`)
            .join("\n") + "\n\n"
        : ""
    }${draft.trim()}`;
    busy = true;
    await safely(async () => {
      await piPrompt(text);
      if (draft === submitted && quote === submittedQuote) {
        draft = "";
        quote = "";
      }
      // Acceptance is not completion. agent_settled owns the running state.
      await refresh();
    });
    busy = false;
  }
  async function stop() {
    await safely(async () => {
      const queued = rpcData(await piRpc({ type: "clear_queue" }));
      const restored = [
        ...(Array.isArray(queued.steering) ? queued.steering : []),
        ...(Array.isArray(queued.followUp) ? queued.followUp : []),
      ]
        .filter((x) => typeof x === "string")
        .join("\n\n");
      if (restored) draft = draft ? `${draft}\n\n${restored}` : restored;
      await piAbort();
    });
  }
  function handleEvent(payload: unknown) {
    const event = record(payload),
      type = String(event.type || "");
    if (type === "extension_ui_request") {
      if (
        ["confirm", "select", "input", "editor"].includes(String(event.method))
      ) {
        if (!requests.some((r) => r.id === event.id))
          requests = [...requests, event];
        showInspector = true;
        navTab = 'sessions';
      } else if (event.method === "notify")
        notice = String(event.message || "");
      else if (event.method === "setStatus")
        extensionStatus = {
          ...extensionStatus,
          [String(event.statusKey)]: String(event.statusText || ""),
        };
      else if (event.method === "setWidget")
        widgets = {
          ...widgets,
          [String(event.widgetKey)]: Array.isArray(event.widgetLines)
            ? event.widgetLines.join("\n")
            : "",
        };
      else if (
        event.method === "set_editor_text" &&
        typeof event.text === "string"
      ) {
        draft = draft ? `${draft}\n\n${event.text}` : event.text;
      } else if (event.method === "setTitle")
        document.title = String(event.title || "Pi Console");
      return;
    }
    if (hydrating) {
      buffered.push(payload);
      return;
    }
    if (type !== "message_update")
      logs = [...logs.slice(-99), JSON.stringify(event, null, 2)];
    if (type === "agent_start") running = true;
    if (type === "agent_settled") {
      running = false;
      streaming = "";
      void safely(refresh);
    }
    if (type === "auto_retry_start") {
      running = true;
      notice = `Retry ${event.attempt}/${event.maxAttempts}: ${event.errorMessage || ""}`;
    }
    if (type === "extension_error" || event.finalError || event.errorMessage)
      error = String(event.error || event.finalError || event.errorMessage);
    if (type === "message_start" && record(event.message).role === "assistant")
      streaming = messageTextFromPi(event.message);
    if (type === "message_update") {
      const delta = record(event.assistantMessageEvent);
      if (delta.type === "text_delta" && typeof delta.delta === "string")
        streaming += delta.delta;
    }
    if (type === "message_end") {
      const message = chatMessage(event.message, `live-${++eventId}`);
      if (message) {
        if (message.role === "assistant") streaming = "";
        const existing = messages.findIndex(
          (m) =>
            m.role === message.role &&
            m.timestamp !== undefined &&
            m.timestamp === message.timestamp,
        );
        if (existing < 0) messages = [...messages, message];
        else
          messages = messages.map((m, i) =>
            i === existing ? { ...message, id: m.id, entryId: m.entryId } : m,
          );
        if (message.error) error = message.error;
      }
    }
    if (type.startsWith("tool_execution_")) {
      const id = String(event.toolCallId),
        existing = tools.find((t) => t.id === id);
      const detail = event.result || event.partialResult || event.args;
      const tool = {
        id,
        name: String(event.toolName || existing?.name || "tool"),
        status: type.endsWith("_end")
          ? event.isError
            ? "Error"
            : "Complete"
          : "Running",
        detail:
          typeof detail === "object"
            ? JSON.stringify(detail, null, 2)
            : existing?.detail || "",
      };
      tools = [...tools.filter((t) => t.id !== id), tool].slice(-60);
    }
  }
  function keyboard(event: KeyboardEvent) {
    if ((event.target as HTMLElement)?.closest?.('.terminal-panel')) return;
    if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
      event.preventDefault();
      void send();
    }
    if (event.key === "Escape" && running) {
      event.preventDefault();
      void stop();
    }
    if (
      (event.metaKey || event.ctrlKey) &&
      event.shiftKey &&
      event.key.toLowerCase() === "c"
    ) {
      event.preventDefault();
      const last = [...messages].reverse().find((m) => m.role === "assistant");
      if (last) void copy(last.text);
    }
    if (
      (event.metaKey || event.ctrlKey) &&
      event.shiftKey &&
      event.key.toLowerCase() === "r"
    ) {
      event.preventDefault();
      const last = messages.at(-1);
      if (last) addReply(last.text);
    }
  }
  onMount(() => {
    desktop = hasDesktop();
    try {
      const l = JSON.parse(localStorage.getItem("pi-layout") || "{}");
      if (typeof l.showNav === "boolean") showNav = l.showNav;
      else showNav = innerWidth > 760;
      if (typeof l.showInspector === "boolean") showInspector = l.showInspector;
      else showInspector = innerWidth > 1100;
      showTerminal = l.showTerminal === true;
      flipped = l.flipped === true;
      rotated = l.rotated === true;
    } catch {
      /* Use defaults. */
    }
    restoreDraft();
    initialized = true;
    const cleanup: Array<() => void> = [];
    const register = async (name: string, fn: (payload: unknown) => void) => {
      const unlisten = await listen(name, (e) => {
        if (!disposed) fn(e.payload);
      });
      const off = () => {
        Promise.resolve(unlisten()).catch(() => {
          /* The backend may already be gone. */
        });
      };
      if (disposed) off();
      else cleanup.push(off);
    };
    if (desktop)
      void safely(async () => {
        // Install every listener before the first invoke can spawn the harness.
        await register("pi-rpc-event", handleEvent);
        await register("pi-rpc-disconnected", () => {
          connected = false;
          running = false;
          requests = [];
          error =
            "Pi disconnected. Your draft is preserved. Reconnect before retrying.";
          epoch++;
          buffered = [];
        });
        if (!disposed) await connect();
      });
    return () => {
      saveDraft();
      disposed = true;
      epoch++;
      diffEpoch++;
      for (const off of cleanup) off();
    };
  });
</script>

<svelte:head
  ><title>Pi Console</title><meta
    name="description"
    content="A focused desktop workspace for the Pi coding harness"
  /></svelte:head
>
<svelte:window onkeydown={keyboard} onbeforeunload={saveDraft} />
<div
  class="console"
  class:flipped
  class:rotated
  class:no-nav={!showNav}
  class:no-inspector={!showInspector || navTab !== 'sessions'}
>
  {#if showNav}
    <aside class="sidebar" aria-label="Navigation">
      <a class="brand" href="/"><span class="pi-mark">Pi</span> Pi Console</a>
      <nav>
        <button class:active={navTab === 'overview'} onclick={() => navTab = 'overview'}><span>⌂</span> Overview</button>
        <button class:active={navTab === 'tasks'} onclick={() => navTab = 'tasks'}><span>☷</span> Tasks</button>
        <button
          class:active={navTab === "sessions"}
          onclick={() => (navTab = "sessions")}><span>▤</span> Sessions</button
        ><button
          class:active={navTab === "workspaces"}
          onclick={() => (navTab = "workspaces")}
          ><span>▱</span> Workspaces</button
        ><button
          class:active={navTab === 'activity'}
          onclick={() => navTab = 'activity'}><span>⌁</span> Activity</button
        ><a href="/settings"><span>⚙</span> Settings</a>
      </nav>
      <div class="nav-section">
        <span class="eyebrow"
          >{navTab === "sessions" ? "Your sessions" : "Workspaces"}</span
        ><button
          class="icon"
          aria-label="New session"
          title="New session"
          disabled={locked}
          onclick={() => changeSession(() => piNewSession(cwd || undefined))}
          >＋</button
        >
      </div>
      <input
        class="search"
        aria-label="Search sessions"
        placeholder="⌕  Search sessions…"
        bind:value={search}
      />
      {#if navTab === "workspaces"}
        <div class="workspace-list">
          {#each workspaces as path}<button
              class:active={workspaceFilter === path}
              title={path}
              onclick={() => {
                workspaceFilter = path;
                navTab = "sessions";
              }}
              ><span>▱</span><span class="truncate"
                >{path.split("/").pop()}</span
              ><small>{sessions.filter((s) => s.cwd === path).length}</small
              ></button
            >{:else}<p class="muted small">
              Connected workspaces will appear here.
            </p>{/each}
        </div>
        <button
          disabled={locked}
          onclick={() => (workspaceForm = !workspaceForm)}
          >＋ Open workspace</button
        >
      {:else}
        {#if workspaceFilter}<button
            class="filter"
            onclick={() => (workspaceFilter = "")}
            >{workspaceFilter.split("/").pop()} · Clear filter ×</button
          >{/if}
        <div class="session-list">
          {#each groupSessions() as [label, items]}<details open>
              <summary>{label}<span>{items.length}</span></summary
              >{#each items as session}<button
                  class:active={session.path === sessionPath}
                  disabled={locked}
                  title={`${session.title}\n${session.cwd}`}
                  onclick={() =>
                    changeSession(() => piSwitchSession(session.path))}
                  ><span class="truncate"
                    >{session.title || "Untitled session"}</span
                  ><small class="truncate">{session.cwd.split("/").pop()}</small
                  ></button
                >{/each}
            </details>{:else}<p class="nav-empty">
              {search
                ? "No matching sessions."
                : connected
                  ? "No saved sessions yet."
                  : "Your sessions, in one place."}<span
                >{connected
                  ? "Start a new session to begin."
                  : "Connect the desktop harness to see your history."}</span
              >
            </p>{/each}
        </div>
      {/if}
      <div class="sidebar-bottom">
        <span class="connection-dot" class:online={connected}></span><span
          >{desktop ? "Local harness" : "Browser preview"}</span
        ><ThemeControl />
      </div>
    </aside>
  {/if}

  <main class="main">
    <header class="topbar" data-tauri-drag-region>
      <div class="project">
        <strong>{project}</strong><span class="truncate" title={cwd}
          >{cwd || "Connect your local Pi harness"}</span
        >
      </div>
      <span class="branch" title={repo?.root}>⑂ {repo?.branch || "—"}</span>
      <div class="agent-state">
        <span class="muted">Agent</span><span class:working={running}
          ><i class="connection-dot" class:online={connected}
          ></i>{stateLabel}</span
        >
      </div>
      {#if running}<button class="stop" onclick={stop}>□ Stop</button>{/if}
      <button
        class="layout-button"
        aria-label="Layout controls"
        aria-expanded={layoutOpen}
        onclick={() => (layoutOpen = !layoutOpen)}>⊞ <span>Layout</span></button
      >
      {#if layoutOpen}<div class="layout-menu">
          <strong>Arrange your workspace</strong><button
            aria-pressed={showNav}
            onclick={() => (showNav = !showNav)}
            >{showNav ? "Hide" : "Show"} navigation</button
          ><button
            aria-pressed={showInspector}
            onclick={() => (showInspector = !showInspector)}
            >{showInspector ? "Hide" : "Show"} inspector</button
          ><button aria-pressed={showTerminal} onclick={() => showTerminal = !showTerminal}>{showTerminal ? 'Hide' : 'Show'} terminal</button
          ><button aria-pressed={flipped} onclick={() => (flipped = !flipped)}
            >Flip panels</button
          ><button aria-pressed={rotated} onclick={() => (rotated = !rotated)}
            >Rotate inspector</button
          ><button
            onclick={() => {
              showNav = innerWidth > 760;
              showInspector = innerWidth > 1100;
              flipped = false;
              rotated = false;
            }}>Reset layout</button
          >
        </div>{/if}
    </header>
    {#if !connected}<div class="connection-banner">
        <span
          ><strong>{desktop ? "Connect to Pi" : "Disconnected preview"}</strong>
          · {desktop
            ? "Your local harness is not connected."
            : "Open the desktop app to access sessions, models and your workspace."}</span
        ><button disabled={!desktop || connecting} onclick={connect}
          >{connecting ? "Connecting…" : "Reconnect"}</button
        >
      </div>{/if}
    {#if error}<div class="alert" role="alert">
        <span>{error}</span><button
          aria-label="Dismiss error"
          onclick={() => (error = "")}>×</button
        >
      </div>{/if}
    {#if notice}<div class="notice" role="status">
        <span>{notice}</span><button
          aria-label="Dismiss notification"
          onclick={() => (notice = "")}>×</button
        >
      </div>{/if}
    {#if workspaceForm}<form
        class="workspace-form"
        onsubmit={(e) => {
          e.preventDefault();
          void changeSession(() => piNewSession(workspaceInput));
        }}
      >
        <label
          >Workspace directory<input
            aria-label="Workspace directory"
            bind:value={workspaceInput}
            placeholder="/absolute/path/to/project"
            required
          /></label
        >
        <p class="muted small">
          Starts a new harness session in this directory. Existing history and
          drafts are preserved.
        </p>
        <button class="primary" disabled={locked || !workspaceInput.trim()}
          >Open workspace</button
        ><button type="button" onclick={() => (workspaceForm = false)}
          >Cancel</button
        >
      </form>{/if}
    {#if navTab !== 'sessions'}
      <WorkspacePages view={navTab} {sessions} {cwd} {sessionId} {sessionPath} {sessionName} {stateLabel} {connected} {locked} {repo} {tools} {logs}
        onview={(view) => navTab = view}
        onopen={(path) => void changeSession(() => piSwitchSession(path))}
        onnew={(dir) => void changeSession(() => piNewSession(dir))}
        onprepare={prepareTask} />
    {:else}
    <div
      class="messages"
      bind:this={messagesEl}
      onscroll={() => {
        if (messagesEl)
          nearBottom =
            messagesEl.scrollHeight -
              messagesEl.scrollTop -
              messagesEl.clientHeight <
            100;
      }}
    >
      {#if messages.length === 0 && !streaming}<div class="welcome">
          <span class="pi-mark large">Pi</span><span class="eyebrow"
            >Your local coding workspace</span
          >
          <h1>A little focus.<br />A lot of possibility.</h1>
          <p>
            {connected
              ? "Start with an idea, a question, or a change you want to make. Pi works right here, in your project."
              : "Your conversations, code changes and agent activity — together in one quiet workspace."}
          </p>
          <div class="welcome-actions">
            <button
              disabled={locked}
              onclick={() =>
                changeSession(() => piNewSession(cwd || undefined))}
              >＋ New session</button
            ><a href="/settings">Configure Pi <span>↗</span></a>
          </div>
          <div class="welcome-note">
            <span>01 <b>Choose a workspace</b></span><span
              >02 <b>Make something useful</b></span
            ><span>03 <b>Review every change</b></span>
          </div>
        </div>{/if}
      {#each messages as message (message.id)}
        <article class="message" use:swipeReply={message}>
          <div class="avatar" class:pi={message.role === "assistant"}>
            {message.role === "assistant"
              ? "Pi"
              : message.role === "user"
                ? "Y"
                : "·"}
          </div>
          <div class="message-content">
            <div class="message-meta">
              <strong
                >{message.role === "user"
                  ? "You"
                  : message.role === "assistant"
                    ? "Pi"
                    : "System"}</strong
              >{#if message.timestamp}<time
                  datetime={new Date(message.timestamp).toISOString()}
                  >{new Date(message.timestamp).toLocaleTimeString([], {
                    hour: "2-digit",
                    minute: "2-digit",
                  })}</time
                >{/if}
              <div class="message-actions">
                <button
                  title="Reply (⌘/Ctrl Shift R for latest)"
                  onclick={() => addReply(message.text)}>↩ Reply</button
                ><button
                  title="Copy (⌘/Ctrl Shift C for last Pi response)"
                  onclick={() => copy(message.text)}>Copy</button
                >{#if message.role === "user"}<button
                    disabled={locked || !message.entryId}
                    onclick={() => forkMessage(message)}>Fork</button
                  >{/if}
              </div>
            </div>
            <div class="markdown">{@html renderMarkdown(message.text)}</div>
            {#if message.error}<p class="text-error">{message.error}</p>{/if}
          </div>
        </article>
      {/each}
      {#if streaming}<article class="message">
          <div class="avatar pi">Pi</div>
          <div class="message-content">
            <div class="message-meta">
              <strong>Pi</strong><span class="working">Writing…</span>
            </div>
            <div class="markdown">{@html renderMarkdown(streaming)}</div>
          </div>
        </article>{/if}
    </div>
    {#if !nearBottom}<button
        class="jump"
        onclick={() => {
          nearBottom = true;
          if (messagesEl) messagesEl.scrollTop = messagesEl.scrollHeight;
        }}>↓ Latest messages</button
      >{/if}
    <div class="composer-wrap">
      {#each Object.entries(widgets).filter(([, text]) => text) as [key, text]}<pre
          class="widget"
          aria-label={key}>{text}</pre>{/each}
      {#if quote}<div class="reply-bar">
          <span><strong>Reply context</strong> · {quote.slice(0, 120)}</span
          ><button
            aria-label="Remove reply context"
            onclick={() => (quote = "")}>×</button
          >
        </div>{/if}
      <form
        class="composer"
        onsubmit={(e) => {
          e.preventDefault();
          void send();
        }}
      >
        <textarea
          bind:this={composerEl}
          aria-label="Message Pi"
          bind:value={draft}
          oninput={(e) => {
            e.currentTarget.style.height = "auto";
            e.currentTarget.style.height = `${Math.min(e.currentTarget.scrollHeight, 220)}px`;
          }}
          rows="3"
          placeholder={connected
            ? "Describe the next change you want Pi to make…"
            : "Draft your next idea… connect the desktop app to send."}
        ></textarea>
        <div class="composer-footer">
          <select
            aria-label="Model"
            value={modelKey}
            disabled={locked}
            onchange={(e) => {
              const next = e.currentTarget.value;
              void safely(async () => {
                const model = models.find(
                  (m) => `${m.provider}/${m.id}` === next,
                );
                if (model) {
                  busy = true;
                  try {
                    await piRpc({
                      type: "set_model",
                      provider: model.provider,
                      modelId: model.id,
                    });
                    await refresh();
                  } finally {
                    busy = false;
                  }
                }
              });
            }}
            ><option value=""
              >{connected ? "Choose model" : "No model connected"}</option
            >{#each models as model}<option
                value={`${model.provider}/${model.id}`}
                >{model.provider} / {model.id}</option
              >{/each}</select
          ><span class="shortcut">⌘ / Ctrl ↵ to send</span><button
            class="primary"
            type="submit"
            disabled={locked || !draft.trim()}>Send <span>↗</span></button
          >
        </div>
      </form>
      <div class="composer-hint">
        <span
          >{connected
            ? "Pi can read, edit and run tools in your workspace."
            : "Preview only. No prompts are sent."}</span
        ><span>Drafts stay in this browser session.</span>
      </div>
    </div>
    {/if}
    <TerminalPanel {cwd} visible={showTerminal} onrunning={(active) => terminalActive = active} onhide={() => showTerminal = false} />
  </main>

  {#if showInspector && navTab === 'sessions'}
    <aside class="inspector" aria-label="Inspector">
      <div class="inspector-tabs">
        <button
          class:active={inspectorTab === "changes"}
          onclick={() => (inspectorTab = "changes")}
          >Changes {#if repo}<span>{repo.files.length}</span>{/if}</button
        ><button
          class:active={inspectorTab === "activity"}
          onclick={() => (inspectorTab = "activity")}>Activity</button
        ><button
          class="icon"
          aria-label="Refresh inspector"
          disabled={!connected}
          onclick={() =>
            safely(async () => {
              await refreshGit();
              if (selectedFile) await openDiff();
            })}>↻</button
        >
      </div>
      <div class="inspector-content">
        {#each requests as request (String(request.id))}<ExtensionRequest
            {request}
            ondone={(id) => (requests = requests.filter((r) => r.id !== id))}
          />{/each}
        {#if inspectorTab === "changes"}
          <section class="changes">
            <div class="section-heading">
              <span class="eyebrow">Working tree</span>{#if repo}<span
                  class="muted small">{repo.branch}</span
                >{/if}
            </div>
            {#if !repo}<div class="inspector-empty">
                <span>⑂</span>
                <h3>{connected ? "Git unavailable" : "Changes live here"}</h3>
                <p>
                  {connected
                    ? gitError
                    : "File changes and hunk diffs will appear when a workspace is connected."}
                </p>
              </div>{:else if !repo.files.length}<p class="muted small">
                Working tree clean. No changes to review.
              </p>{:else}<div class="files">
                {#each repo.files as file}<button
                    class:active={selectedFile === file.path}
                    title={file.path}
                    onclick={() => openDiff(file.path)}
                    ><span
                      class="file-status"
                      class:added={file.status.includes("A") ||
                        file.status === "??"}>{file.status.trim()}</span
                    ><span class="truncate">{file.path}</span></button
                  >{/each}
              </div>{/if}
          </section>
          {#if selectedFile}<section class="diff">
              <div class="section-heading">
                <strong class="truncate" title={selectedFile}
                  >{selectedFile}</strong
                ><button
                  class="icon"
                  aria-label="Close diff"
                  onclick={() => {
                    selectedFile = "";
                    patch = "";
                    diffEpoch++;
                  }}>×</button
                >
              </div>
              <div class="diff-tabs">
                <button
                  class:active={!staged}
                  onclick={() => openDiff(selectedFile, false)}
                  >Working tree</button
                ><button
                  class:active={staged}
                  onclick={() => openDiff(selectedFile, true)}>Staged</button
                >
              </div>
              {#if diffError}<p class="text-error small">
                  {diffError}
                </p>{:else if !hunks.length}<p class="muted small">
                  {patch || "No text hunks on this side of the index."}
                </p>{/if}{#each hunks as hunk}<div class="hunk">
                  <div class="hunk-head">
                    <code>{hunk.header}</code><button
                      title="Quote this hunk in chat"
                      onclick={() =>
                        addReply(quoteHunk(selectedFile, hunk, staged))}
                      >↩ Discuss</button
                    >
                  </div>
                  <div class="diff-lines">
                    {#each hunk.lines as line}<div
                        class:add={line.kind === "add"}
                        class:remove={line.kind === "remove"}
                      >
                        <span>{line.old ?? ""}</span><span
                          >{line.next ?? ""}</span
                        ><code>{line.text}</code>
                      </div>{/each}
                  </div>
                </div>{/each}
            </section>{/if}
        {:else}<section>
            <div class="section-heading">
              <span class="eyebrow">Tool activity</span><span
                class="muted small"
                >{tools.length ? `${tools.length} recent` : "No activity"}</span
              >
            </div>
            {#each tools as tool (tool.id)}<details class="tool">
                <summary
                  ><span class:working={tool.status === "Running"}
                    >{tool.name}</span
                  ><small>{tool.status}</small></summary
                >
                <pre>{tool.detail.slice(0, 20000)}{tool.detail.length > 20000
                    ? "\n… Preview limited to 20,000 characters."
                    : ""}</pre>
              </details>{:else}<div class="inspector-empty">
                <span>⌁</span>
                <h3>A clear view of the work</h3>
                <p>
                  Real tool calls and their results appear here as Pi works.
                  {connected ? 'No tool activity recorded in this session yet.' : 'Nothing is running in preview.'}
                </p>
              </div>{/each}
            <details class="logs">
              <summary>Protocol log <span>{logs.length}</span></summary
              >{#each logs as log}<pre>{log.slice(0, 20000)}</pre>{/each}
            </details>
          </section>{/if}
        <section class="session-info">
          <span class="eyebrow">Session info</span>
          <dl>
            <dt>Status</dt>
            <dd>{stateLabel}</dd>
            <dt>Session</dt>
            <dd title={sessionPath}>
              {sessionName || sessionId || "Not connected"}
            </dd>
            <dt>Workspace</dt>
            <dd title={cwd}>{cwd || "—"}</dd>
            <dt>Model</dt>
            <dd>{modelKey || "—"}</dd>
            <dt>Thinking</dt>
            <dd>{thinking || "—"}</dd>
            {#if stats.cost !== undefined}<dt>Cost</dt>
              <dd>
                ${Number(stats.cost).toFixed(4)}
              </dd>{/if}{#if record(stats.tokens).total !== undefined}<dt>
                Total tokens
              </dt>
              <dd>
                {Number(record(stats.tokens).total).toLocaleString()}
              </dd>{/if}
          </dl>
          <button
            class="text-button"
            onclick={() => (inspectorTab = "activity")}
            >View session activity ↗</button
          >
        </section>
        <div class="inspector-note">
          Local-first. Powered by your Pi harness.<br />Review tool actions and
          changes before continuing.
        </div>
      </div>
    </aside>
  {/if}
  <footer class="statusbar">
    <span
      ><i class="connection-dot" class:online={connected}></i>{stateLabel}</span
    ><span>⑂ {repo?.branch || "No repository"}</span><span
      class="status-cwd truncate"
      title={cwd}>{cwd || "No workspace connected"}</span
    ><span class="status-model truncate" title={modelKey}>{modelKey || 'No model'}</span><span class="extension-status truncate"
      >{Object.values(extensionStatus).filter(Boolean).join(" · ")}</span
    ><span
      >{draft.trim() ? draft.trim().split(/\s+/).length : 0} draft words</span
    ><button aria-label="Toggle terminal" aria-pressed={showTerminal} onclick={() => showTerminal = !showTerminal}>Terminal</button><button
      onclick={() => (layoutOpen = !layoutOpen)}
      aria-label="Toggle panel controls">⊞</button
    >
  </footer>
</div>

<style>
  .console {
    display: grid;
    grid-template-columns: 235px minmax(340px, 1fr) 320px;
    grid-template-rows: minmax(0, 1fr) 29px;
    grid-template-areas: "nav main inspector" "status status status";
    height: 100dvh;
    overflow: hidden;
    background:
      radial-gradient(ellipse at 40% 0%, var(--glass-soft), transparent 65%),
      var(--bg);
  }
  .console.no-nav {
    grid-template-columns: minmax(340px, 1fr) 320px;
    grid-template-areas: "main inspector" "status status";
  }
  .console.no-inspector {
    grid-template-columns: 235px minmax(340px, 1fr);
    grid-template-areas: "nav main" "status status";
  }
  .console.no-nav.no-inspector,
  .console.flipped.no-nav.no-inspector {
    grid-template-columns: minmax(0, 1fr);
    grid-template-areas: "main" "status";
  }
  .console.flipped {
    grid-template-columns: 320px minmax(340px, 1fr) 235px;
    grid-template-areas: "inspector main nav" "status status status";
  }
  .console.flipped.no-nav {
    grid-template-columns: 320px minmax(340px, 1fr);
    grid-template-areas: "inspector main" "status status";
  }
  .console.flipped.no-inspector {
    grid-template-columns: minmax(340px, 1fr) 235px;
    grid-template-areas: "main nav" "status status";
  }
  .console.rotated:not(.no-inspector) {
    grid-template-columns: 235px minmax(340px, 1fr);
    grid-template-rows: minmax(0, 1fr) 260px 29px;
    grid-template-areas: "nav main" "nav inspector" "status status";
  }
  .console.rotated.flipped:not(.no-inspector) {
    grid-template-columns: minmax(340px, 1fr) 235px;
    grid-template-areas: "main nav" "inspector nav" "status status";
  }
  .console.rotated.no-nav:not(.no-inspector) {
    grid-template-columns: 1fr;
    grid-template-areas: "main" "inspector" "status";
  }
  .sidebar {
    grid-area: nav;
    min-height: 0;
    border-right: 1px solid var(--border-soft);
    padding: 23px 12px 12px;
    display: flex;
    flex-direction: column;
    background: var(--glass-soft);
    gap: 12px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 16px;
    font-weight: 600;
    color: var(--accent);
    padding: 0 8px 15px;
  }
  .brand:hover {
    text-decoration: none;
  }
  .pi-mark {
    font-family: Georgia, serif;
    font-weight: 700;
    border: 1px solid var(--accent);
    color: var(--accent);
    font-size: 23px;
    line-height: 32px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 35px;
    border-radius: 5px;
    box-shadow: 0 0 15px color-mix(in srgb, var(--accent) 7%, transparent);
  }
  nav {
    display: grid;
    gap: 5px;
  }
  nav button,
  nav a {
    display: flex;
    gap: 14px;
    align-items: center;
    text-align: left;
    padding: 11px 12px;
    border: 1px solid transparent;
    color: var(--muted);
    background: transparent;
    font-size: 13px;
    border-radius: 5px;
  }
  nav span {
    font-size: 21px;
    width: 22px;
    line-height: 22px;
  }
  nav a:hover {
    text-decoration: none;
    background: var(--glass);
  }
  nav .active {
    color: var(--accent);
    background: var(--glass);
    border-color: var(--border-soft);
  }
  .nav-section {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin: 22px 7px 0;
  }
  .eyebrow {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    font-weight: 500;
    color: var(--muted);
  }
  .icon {
    padding: 2px 6px;
    border-color: transparent;
    background: transparent;
    font-size: 20px;
    color: var(--muted);
  }
  .search {
    width: 100%;
    font-size: 12px;
    background: var(--glass-soft);
    border-color: var(--border-soft);
  }
  .session-list {
    overflow: auto;
    flex: 1;
    min-height: 0;
  }
  .session-list details {
    margin: 7px 0 17px;
  }
  .session-list summary {
    font-size: 10px;
    text-transform: uppercase;
    color: var(--muted);
    padding: 6px;
    cursor: pointer;
  }
  .session-list summary span {
    float: right;
  }
  .session-list button {
    display: block;
    text-align: left;
    width: 100%;
    margin: 2px 0;
    background: transparent;
    border-color: transparent;
    font-size: 12px;
    padding: 10px;
  }
  .session-list button.active {
    background: var(--glass);
    border-color: var(--border-soft);
    border-left: 2px solid var(--accent);
    color: var(--accent);
  }
  .session-list small {
    font-size: 10px;
    color: var(--muted);
    margin-top: 4px;
  }
  .truncate {
    display: block;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .nav-empty {
    font-size: 12px;
    color: var(--text);
    padding: 10px 9px;
    line-height: 1.7;
  }
  .nav-empty span {
    display: block;
    color: var(--muted);
    margin-top: 6px;
  }
  .sidebar-bottom {
    margin-top: auto;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    border-top: 1px solid var(--border-soft);
    padding: 16px 6px 2px;
    font-size: 11px;
    color: var(--muted);
  }
  .sidebar-bottom :global(.theme-control) {
    margin-top: 6px;
    width: 100%;
    justify-content: space-between;
  }
  .connection-dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--muted);
    flex-shrink: 0;
  }
  .connection-dot.online {
    background: var(--good);
  }
  .workspace-list {
    display: grid;
    gap: 4px;
  }
  .workspace-list button {
    display: flex;
    gap: 10px;
    align-items: center;
    font-size: 12px;
    text-align: left;
  }
  .workspace-list small {
    margin-left: auto;
  }
  .filter {
    font-size: 10px;
    color: var(--accent);
  }
  .main {
    grid-area: main;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    position: relative;
  }
  .topbar {
    display: flex;
    align-items: center;
    gap: 18px;
    min-height: 83px;
    padding: 16px 25px;
    border-bottom: 1px solid var(--border-soft);
    position: relative;
    background: var(--glass-soft);
  }
  .project {
    min-width: 0;
    max-width: 40%;
  }
  .project strong {
    display: block;
    font-size: 15px;
    font-weight: 600;
  }
  .project > span {
    font-size: 11px;
    color: var(--muted);
    margin-top: 2px;
  }
  .branch {
    font-size: 12px;
    border-left: 1px solid var(--border-soft);
    padding-left: 18px;
    white-space: nowrap;
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--muted);
  }
  .agent-state {
    margin-left: auto;
    font-size: 12px;
    white-space: nowrap;
  }
  .agent-state > span {
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .agent-state > .muted {
    font-size: 10px;
    margin-bottom: 2px;
  }
  .working {
    color: var(--good);
  }
  .stop {
    color: var(--bad);
    background: color-mix(in srgb, var(--bad) 8%, transparent);
    font-size: 12px;
  }
  .layout-button {
    font-size: 12px;
    padding: 7px;
  }
  .layout-menu {
    position: absolute;
    right: 16px;
    top: 69px;
    width: 215px;
    z-index: 20;
    padding: 12px;
    background: var(--panel);
    border: 1px solid var(--border);
    box-shadow: 0 12px 40px #0004;
    border-radius: 7px;
    display: grid;
    gap: 6px;
  }
  .layout-menu strong {
    font-size: 11px;
    color: var(--muted);
    margin: 2px 0 5px;
  }
  .layout-menu button {
    text-align: left;
    font-size: 12px;
  }
  .layout-menu button[aria-pressed="true"] {
    color: var(--accent);
  }
  .connection-banner,
  .alert,
  .notice {
    display: flex;
    align-items: center;
    gap: 12px;
    justify-content: space-between;
    font-size: 11px;
    padding: 10px 24px;
    border-bottom: 1px solid var(--border-soft);
  }
  .connection-banner {
    color: var(--muted);
    background: var(--glass-soft);
  }
  .connection-banner strong {
    color: var(--text);
    font-weight: 500;
  }
  .connection-banner button {
    font-size: 10px;
    padding: 5px 8px;
  }
  .alert {
    color: var(--bad);
    overflow-wrap: anywhere;
  }
  .notice {
    color: var(--accent);
  }
  .alert button,
  .notice button {
    padding: 0 5px;
    background: transparent;
    border: 0;
  }
  .workspace-form {
    padding: 20px;
    margin: 12px;
    border: 1px solid var(--border);
    border-radius: 5px;
  }
  .workspace-form label {
    display: grid;
    gap: 7px;
  }
  .workspace-form button {
    margin-right: 8px;
  }
  .messages {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 8px 30px 20px;
    overscroll-behavior: contain;
  }
  .welcome {
    max-width: 580px;
    margin: clamp(35px, 9vh, 100px) auto 35px;
  }
  .pi-mark.large {
    height: 44px;
    width: 43px;
    font-size: 28px;
    margin-bottom: 28px;
  }
  .welcome > .eyebrow {
    display: block;
    font-size: 10px;
    letter-spacing: 0.14em;
    color: var(--accent);
  }
  .welcome h1 {
    font-size: clamp(26px, 3vw, 40px);
    line-height: 1.2;
    font-weight: 500;
    letter-spacing: -1.2px;
    margin: 16px 0 18px;
  }
  .welcome p {
    max-width: 430px;
    line-height: 1.85;
    color: var(--muted);
    font-size: 13px;
  }
  .welcome-actions {
    display: flex;
    align-items: center;
    gap: 24px;
    margin: 26px 0 48px;
  }
  .welcome-actions button,
  .welcome-actions a {
    font-size: 12px;
  }
  .welcome-actions button {
    padding: 10px 15px;
  }
  .welcome-actions a span {
    padding-left: 12px;
  }
  .welcome-note {
    display: grid;
    gap: 14px;
    padding-top: 24px;
    border-top: 1px solid var(--border-soft);
    font-family: monospace;
    font-size: 10px;
    color: var(--muted);
  }
  .welcome-note b {
    font-weight: 400;
    font-family: inherit;
    letter-spacing: 0.02em;
    margin-left: 16px;
  }
  .message {
    display: flex;
    gap: 17px;
    padding: 25px 0;
    border-bottom: 1px solid var(--border-soft);
    max-width: 900px;
    margin: 0 auto;
    touch-action: pan-y;
  }
  .avatar {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--accent);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    font-size: 11px;
  }
  .avatar.pi {
    border: 1px solid var(--accent);
    border-radius: 4px;
    font:
      700 19px Georgia,
      serif;
    background: transparent;
    width: 26px;
    height: 27px;
  }
  .message-content {
    flex: 1;
    min-width: 0;
  }
  .message-meta {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-bottom: 8px;
    font-size: 11px;
    color: var(--muted);
    min-height: 26px;
  }
  .message-meta strong {
    font-size: 13px;
    color: var(--text);
    font-weight: 500;
  }
  .message-actions {
    display: flex;
    margin-left: auto;
    gap: 3px;
    opacity: 0;
  }
  .message:hover .message-actions,
  .message:focus-within .message-actions {
    opacity: 1;
  }
  .message-actions button {
    padding: 3px 6px;
    font-size: 10px;
    background: transparent;
    border-color: transparent;
    color: var(--muted);
  }
  .text-error {
    color: var(--bad);
  }
  .jump {
    position: absolute;
    bottom: 210px;
    left: 50%;
    transform: translateX(-50%);
    font-size: 11px;
    background: var(--panel);
    z-index: 2;
  }
  .composer-wrap {
    padding: 12px 26px 12px;
  }
  .composer {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: linear-gradient(130deg, var(--glass), var(--glass-soft));
    backdrop-filter: var(--blur);
    box-shadow: 0 4px 20px #0000000a;
  }
  .composer:focus-within {
    border-color: color-mix(in srgb, var(--accent) 60%, var(--border));
  }
  .composer textarea {
    width: 100%;
    border: 0;
    background: transparent;
    resize: none;
    padding: 18px;
    min-height: 85px;
    max-height: 220px;
    font-size: 13px;
    line-height: 1.6;
    outline: none;
  }
  .composer-footer {
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 4px 13px 13px;
  }
  .composer-footer select {
    border: 0;
    background: var(--glass-soft);
    font-size: 10px;
    color: var(--muted);
    max-width: 50%;
    text-overflow: ellipsis;
  }
  .shortcut {
    font-size: 10px;
    color: var(--muted);
    margin-left: auto;
    white-space: nowrap;
  }
  .composer-footer button {
    font-size: 12px;
    padding: 9px 14px;
  }
  .composer-footer button span {
    padding-left: 14px;
  }
  .composer-hint {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    font-size: 9px;
    color: var(--muted);
    margin: 9px 3px 0;
  }
  .reply-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 11px;
    padding: 10px 12px;
    border-left: 2px solid var(--accent);
    background: var(--glass);
    margin-bottom: 8px;
    gap: 10px;
  }
  .reply-bar > span {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .reply-bar button {
    border: 0;
    padding: 0 6px;
  }
  .widget {
    white-space: pre-wrap;
    color: var(--muted);
    font-size: 11px;
    max-height: 100px;
    overflow: auto;
  }
  .inspector {
    grid-area: inspector;
    min-height: 0;
    min-width: 0;
    border-left: 1px solid var(--border-soft);
    background: var(--glass-soft);
    display: flex;
    flex-direction: column;
  }
  .inspector-tabs {
    display: flex;
    gap: 13px;
    align-items: center;
    padding: 23px 18px;
    border-bottom: 1px solid var(--border-soft);
    height: 83px;
    flex-shrink: 0;
  }
  .inspector-tabs > button:not(.icon) {
    border: 0;
    background: transparent;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-size: 10px;
    color: var(--muted);
    padding: 6px 0;
  }
  .inspector-tabs > button.active {
    color: var(--text);
  }
  .inspector-tabs span {
    padding: 2px 6px;
    border-radius: 9px;
    background: var(--glass);
    margin-left: 4px;
  }
  .inspector-tabs .icon {
    margin-left: auto;
  }
  .inspector-content {
    overflow: auto;
    min-height: 0;
    flex: 1;
  }
  .inspector-content > section {
    padding: 21px 18px;
    border-bottom: 1px solid var(--border-soft);
  }
  .inspector-content :global(.request) {
    margin: 12px;
  }
  .section-heading {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    margin-bottom: 15px;
  }
  .section-heading strong {
    font-size: 12px;
    font-weight: 500;
  }
  .files {
    display: grid;
    gap: 4px;
  }
  .files button {
    border: 1px solid transparent;
    background: transparent;
    display: flex;
    gap: 12px;
    align-items: center;
    width: 100%;
    text-align: left;
    font-size: 11px;
    padding: 8px 0;
  }
  .files button.active {
    background: var(--glass);
    border-color: var(--border-soft);
  }
  .file-status {
    color: var(--accent);
    font-family: monospace;
    font-size: 10px;
    min-width: 16px;
  }
  .file-status.added {
    color: var(--good);
  }
  .inspector-empty {
    padding: 24px 0 30px;
  }
  .inspector-empty > span {
    font-size: 30px;
    color: var(--muted);
    font-weight: 300;
  }
  .inspector-empty h3 {
    font-size: 12px;
    font-weight: 500;
    margin: 14px 0 8px;
  }
  .inspector-empty p {
    font-size: 11px;
    color: var(--muted);
    line-height: 1.85;
    overflow-wrap: anywhere;
  }
  .session-info dl {
    display: grid;
    grid-template-columns: 70px minmax(0, 1fr);
    font-size: 10px;
    gap: 16px 14px;
    margin: 22px 0;
  }
  .session-info dt {
    color: var(--muted);
  }
  .session-info dd {
    margin: 0;
    text-align: right;
    overflow-wrap: anywhere;
  }
  .text-button {
    border: 0;
    background: transparent;
    color: var(--accent);
    font-size: 11px;
    padding: 0;
  }
  .inspector-note {
    font-size: 9px;
    color: var(--muted);
    padding: 22px 18px;
    line-height: 1.9;
  }
  .small {
    font-size: 11px;
  }
  .muted {
    color: var(--muted);
  }
  .diff-tabs {
    display: flex;
    gap: 6px;
    margin-bottom: 15px;
  }
  .diff-tabs button {
    font-size: 10px;
    padding: 5px 9px;
  }
  .diff-tabs .active {
    color: var(--accent);
  }
  .hunk {
    margin: 12px -10px;
    border: 1px solid var(--border-soft);
    border-radius: 4px;
    overflow: hidden;
  }
  .hunk-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 7px;
    background: var(--glass);
    padding: 7px;
  }
  .hunk-head code {
    font-size: 9px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
  }
  .hunk-head button {
    font-size: 9px;
    padding: 3px 5px;
    color: var(--accent);
    white-space: nowrap;
  }
  .diff-lines {
    overflow: auto;
  }
  .diff-lines > div {
    display: flex;
    width: max-content;
    min-width: 100%;
    font-size: 10px;
    line-height: 1.9;
  }
  .diff-lines span {
    width: 24px;
    flex-shrink: 0;
    text-align: right;
    color: var(--muted);
    padding-right: 5px;
    user-select: none;
  }
  .diff-lines code {
    font-size: 10px;
    white-space: pre;
    padding: 0 8px;
  }
  .diff-lines .add {
    color: var(--good);
    background: color-mix(in srgb, var(--good) 8%, transparent);
  }
  .diff-lines .remove {
    color: var(--bad);
    background: color-mix(in srgb, var(--bad) 8%, transparent);
  }
  .tool,
  .logs {
    font-size: 11px;
    border-bottom: 1px solid var(--border-soft);
    padding: 12px 0;
  }
  .tool summary,
  .logs summary {
    cursor: pointer;
  }
  .tool small,
  .logs summary span {
    float: right;
    color: var(--muted);
    font-size: 10px;
  }
  .tool pre,
  .logs pre {
    overflow: auto;
    font-size: 10px;
    max-height: 300px;
    background: var(--glass);
    padding: 10px;
  }
  .logs {
    margin-top: 20px;
  }
  .statusbar {
    grid-area: status;
    display: flex;
    align-items: center;
    gap: 18px;
    padding: 0 14px;
    border-top: 1px solid var(--border-soft);
    font-size: 9px;
    color: var(--muted);
    background: var(--panel);
    min-width: 0;
  }
  .statusbar > span:first-child {
    display: flex;
    gap: 7px;
    align-items: center;
  }
  .status-cwd {
    max-width: 30%;
  }
  .status-model { max-width: 25%; }
  .extension-status {
    margin-left: auto;
    max-width: 25%;
  }
  .statusbar button {
    font-size: 15px;
    padding: 0 3px;
    border: 0;
    background: transparent;
  }
  .flipped .sidebar {
    border-right: 0;
    border-left: 1px solid var(--border-soft);
  }
  .flipped .inspector {
    border-left: 0;
    border-right: 1px solid var(--border-soft);
  }
  .rotated .inspector {
    border-top: 1px solid var(--border-soft);
  }
  .rotated .inspector-tabs {
    height: 45px;
    padding: 10px 20px;
  }
  .rotated .inspector-content {
    display: flex;
    align-items: flex-start;
  }
  .rotated .inspector-content > section {
    min-width: 300px;
    flex: 1;
    border-bottom: 0;
  }
  .rotated .inspector-note {
    display: none;
  }
  @media (max-width: 1100px) {
    .topbar {
      padding: 14px 18px;
      gap: 10px;
    }
    .branch {
      display: none;
    }
    .messages {
      padding: 8px 22px;
    }
    .composer-wrap {
      padding: 10px 18px;
    }
    .composer-hint span:last-child {
      display: none;
    }
    .shortcut {
      display: none;
    }
    .composer-footer button {
      margin-left: auto;
    }
    .message-actions {
      opacity: 1;
    }
    .console:not(.no-inspector):not(.rotated) {
      grid-template-columns: 210px minmax(0, 1fr) 280px;
    }
    .console.flipped:not(.no-nav):not(.no-inspector):not(.rotated) {
      grid-template-columns: 280px minmax(0, 1fr) 210px;
    }
    .console.no-nav:not(.no-inspector):not(.rotated) {
      grid-template-columns: minmax(0, 1fr) 280px;
    }
    .console.flipped.no-nav:not(.no-inspector):not(.rotated) {
      grid-template-columns: 280px minmax(0, 1fr);
    }
  }
  @media (max-width: 760px) {
    /* Mobile panels overlay the conversation regardless of persisted desktop layout. */
    .console {
      grid-template-columns: minmax(0, 1fr) !important;
      grid-template-rows: minmax(0, 1fr) 29px !important;
      grid-template-areas: "main" "status" !important;
    }
    .sidebar {
      position: fixed;
      top: 75px;
      bottom: 29px;
      left: 0;
      width: 235px;
      z-index: 12;
      background: var(--panel);
      box-shadow: 10px 0 30px #0004;
    }
    .inspector {
      position: fixed;
      top: 75px;
      bottom: 29px;
      right: 0;
      width: min(320px, 90vw);
      z-index: 13;
      background: var(--panel);
      box-shadow: -10px 0 30px #0004;
    }
    .rotated .inspector-content {
      display: block;
    }
    .rotated .inspector-content > section {
      min-width: 0;
    }
    .topbar {
      min-height: 75px;
      height: 75px;
      padding: 12px 16px;
    }
    .project {
      max-width: 48%;
    }
    .agent-state {
      font-size: 10px;
    }
    .layout-button span {
      display: none;
    }
    .messages {
      padding: 0 18px 16px;
    }
    .welcome {
      margin-top: 35px;
    }
    .welcome h1 {
      font-size: 32px;
    }
    .welcome-actions {
      margin-bottom: 30px;
    }
    .composer-wrap {
      padding: 8px 12px;
    }
    .composer-footer select {
      max-width: 65%;
    }
    .composer-hint {
      font-size: 8px;
    }
    .connection-banner {
      padding: 9px 15px;
      font-size: 10px;
    }
    .status-cwd,
    .status-model,
    .extension-status {
      display: none;
    }
    .statusbar {
      gap: 12px;
    }
    .statusbar > span:nth-last-child(2) {
      margin-left: auto;
    }
    .message {
      gap: 10px;
    }
    .message-meta {
      flex-wrap: wrap;
    }
    .message-actions {
      width: 100%;
      margin-left: 0;
    }
    .layout-menu {
      top: 65px;
    }
    .welcome-note {
      gap: 10px;
    }
    .brand {
      padding-bottom: 0;
    }
    .sidebar {
      padding-top: 16px;
    }
  }
</style>
