<script lang="ts">
  import { onMount } from "svelte";
  let theme = $state("auto");
  function apply() {
    try {
      localStorage.setItem("pi-theme", theme);
    } catch {
      /* Private browsing may disallow storage. */
    }
    document.documentElement.dataset.theme =
      theme === "dark" ||
      (theme === "auto" && matchMedia("(prefers-color-scheme: dark)").matches)
        ? "dark"
        : "light";
  }
  onMount(() => {
    try {
      theme = localStorage.getItem("pi-theme") || "auto";
    } catch {
      /* Use system preference. */
    }
    const media = matchMedia("(prefers-color-scheme: dark)");
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  });
</script>

<label class="theme-control"
  >Theme <select aria-label="Theme" bind:value={theme} onchange={apply}
    ><option value="auto">Auto</option><option value="dark">Dark</option><option
      value="light">Light</option
    ></select
  ></label
>

<style>
  .theme-control {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--muted);
  }
  select {
    padding: 5px 8px;
  }
</style>
