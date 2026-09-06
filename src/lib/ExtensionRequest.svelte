<script lang="ts">
  import { onMount } from "svelte";
  import { piExtensionResponse } from "$lib/pi/api";
  import type { JsonRecord } from "$lib/pi/protocol";
  let {
    request,
    ondone,
  }: { request: JsonRecord; ondone: (id: string) => void } = $props();
  let value = $state("");
  let error = $state("");
  let busy = $state(false);
  let finished = false;
  const id = $derived(String(request.id));
  const options = $derived(
    Array.isArray(request.options)
      ? request.options.filter((o): o is string => typeof o === "string")
      : [],
  );
  onMount(() => {
    value =
      typeof request.prefill === "string" ? request.prefill : options[0] || "";
    if (typeof request.timeout === "number") {
      const timer = setTimeout(() => void respond({ cancelled: true }), Math.max(0, request.timeout));
      return () => clearTimeout(timer);
    }
  });
  async function respond(data: JsonRecord) {
    if (busy || finished) return;
    busy = true;
    try {
      await piExtensionResponse({ id, ...data });
      finished = true;
      ondone(id);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<section class="request" aria-label="Extension request">
  <span class="eyebrow">Extension · response requested</span>
  <h3>{String(request.title || "Pi extension")}</h3>
  {#if request.message}<p>{String(request.message)}</p>{/if}
  {#if request.timeout}<p class="muted">
      This request expires automatically.
    </p>{/if}
  {#if request.method === "select"}<select
      aria-label="Extension choice"
      bind:value
      >{#each options as option}<option>{option}</option>{/each}</select
    >
  {:else if request.method === "input" || request.method === "editor"}<textarea
      aria-label="Extension response"
      rows={request.method === "editor" ? 5 : 2}
      placeholder={String(request.placeholder || "")}
      bind:value></textarea>{/if}
  {#if error}<p role="alert">{error}</p>{/if}
  <div class="actions">
    <button disabled={busy} onclick={() => respond({ cancelled: true })}
      >{request.method === "confirm" ? "Reject" : "Cancel"}</button
    ><button
      class="primary"
      disabled={busy}
      onclick={() =>
        respond(request.method === "confirm" ? { confirmed: true } : { value })}
      >{request.method === "confirm" ? "Confirm" : "Submit"}</button
    >
  </div>
</section>

<style>
  .request {
    border: 1px solid color-mix(in srgb, var(--accent) 45%, transparent);
    background: color-mix(in srgb, var(--accent) 5%, var(--panel));
    padding: 16px;
    border-radius: 6px;
    margin: 12px 0;
  }
  .eyebrow {
    color: var(--accent);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.07em;
  }
  h3 {
    font-size: 14px;
    margin: 10px 0;
  }
  p {
    font-size: 12px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .muted {
    color: var(--muted);
  }
  select,
  textarea {
    width: 100%;
    margin-bottom: 10px;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  .actions button {
    flex: 1;
  }
</style>
