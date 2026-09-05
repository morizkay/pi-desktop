<script lang="ts">
  let { children } = $props();
</script>

{@render children()}

<style>
  :global(:root) {
    --bg: #f5f4f0;
    --panel: #eeede8;
    --glass: rgba(255, 255, 255, 0.55);
    --glass-strong: #faf9f6;
    --glass-soft: rgba(0, 0, 0, 0.025);
    --text: #252622;
    --muted: #74766f;
    --border: #d4d5cc;
    --border-soft: #dedfd7;
    --accent: #9b6600;
    --accent-fg: #17140b;
    --button-accent: #ffbd38;
    --good: #368249;
    --bad: #bd4945;
    --blur: blur(18px);
    --ease: cubic-bezier(0.32, 0.72, 0, 1);
    --spring: var(--ease);
    font-family:
      Inter,
      -apple-system,
      BlinkMacSystemFont,
      "Segoe UI",
      sans-serif;
    font-size: 14px;
    line-height: 1.5;
    color: var(--text);
    color-scheme: light;
  }
  :global(:root[data-theme="dark"]) {
    --bg: #121414;
    --panel: #161818;
    --glass: rgba(255, 255, 255, 0.035);
    --glass-strong: #1c1e1e;
    --glass-soft: rgba(255, 255, 255, 0.025);
    --text: #e6e7e3;
    --muted: #959b98;
    --border: #343837;
    --border-soft: #2a2e2d;
    --accent: #efb63e;
    --accent-fg: #17140b;
    --good: #6ac484;
    --bad: #ed8278;
    color-scheme: dark;
  }
  :global(*),
  :global(*::before),
  :global(*::after) {
    box-sizing: border-box;
  }
  :global(body) {
    margin: 0;
    background: var(--bg);
    color: var(--text);
  }
  :global(button),
  :global(input),
  :global(select),
  :global(textarea) {
    font: inherit;
    color: inherit;
    border: 1px solid var(--border);
    background: var(--glass);
    border-radius: 5px;
    padding: 8px 11px;
  }
  :global(button) {
    cursor: pointer;
    transition:
      background 0.15s var(--ease),
      border-color 0.15s var(--ease);
  }
  :global(button:hover:not(:disabled)) {
    background: var(--glass-strong);
    border-color: var(--muted);
  }
  :global(button:disabled),
  :global(select:disabled) {
    opacity: 0.42;
    cursor: not-allowed;
  }
  :global(button.primary) {
    background: var(--button-accent);
    border-color: var(--button-accent);
    color: var(--accent-fg);
    font-weight: 600;
  }
  :global(button.primary:hover:not(:disabled)) {
    background: #ffd175;
    border-color: #ffd175;
  }
  :global(a) {
    color: var(--accent);
    text-decoration: none;
  }
  :global(a:hover) {
    text-decoration: underline;
  }
  :global(:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 3px;
  }
  :global(textarea) {
    resize: vertical;
  }
  :global(code),
  :global(pre) {
    font-family: "SFMono-Regular", Consolas, "Liberation Mono", monospace;
    font-size: 12px;
  }
  :global(.markdown) {
    overflow-wrap: anywhere;
    line-height: 1.75;
  }
  :global(.markdown > :first-child) {
    margin-top: 0;
  }
  :global(.markdown > :last-child) {
    margin-bottom: 0;
  }
  :global(.markdown p) {
    margin: 0.65em 0;
  }
  :global(.markdown pre) {
    overflow: auto;
    background: var(--glass-soft);
    border: 1px solid var(--border-soft);
    border-radius: 5px;
    padding: 16px;
    line-height: 1.65;
  }
  :global(.markdown code:not(pre code)) {
    padding: 2px 5px;
    background: var(--glass);
    border: 1px solid var(--border-soft);
    border-radius: 3px;
  }
  :global(.markdown blockquote) {
    border-left: 2px solid var(--accent);
    margin: 12px 0;
    padding: 0 16px;
    color: var(--muted);
  }
  :global(.markdown table) {
    display: block;
    overflow: auto;
    border-collapse: collapse;
  }
  :global(.markdown td),
  :global(.markdown th) {
    padding: 6px 12px;
    border: 1px solid var(--border);
  }
  :global(.markdown h1),
  :global(.markdown h2),
  :global(.markdown h3) {
    font-size: 1.15em;
    line-height: 1.4;
  }
  :global(.hljs-keyword),
  :global(.hljs-selector-tag),
  :global(.hljs-literal) {
    color: var(--accent);
  }
  :global(.hljs-string),
  :global(.hljs-addition) {
    color: var(--good);
  }
  :global(.hljs-comment) {
    color: var(--muted);
  }
  :global(.hljs-number),
  :global(.hljs-deletion) {
    color: var(--bad);
  }
  :global(::selection) {
    background: color-mix(in srgb, var(--accent) 28%, transparent);
  }
  :global(*) {
    scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
  }
  @media (prefers-reduced-motion: reduce) {
    :global(*),
    :global(*::before),
    :global(*::after) {
      animation: none !important;
      transition: none !important;
      scroll-behavior: auto !important;
    }
  }
</style>
