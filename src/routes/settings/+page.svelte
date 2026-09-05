<script lang="ts">
  import { onMount } from 'svelte';
  import ThemeControl from '$lib/ThemeControl.svelte';
  import { inModelScope } from '$lib/pi/model-scope';
  import { hasDesktop, piReadSettings, piPatchSettings, piListProviderIds, piListModelsStore, piListAuthProviders, piSetApiKey, piRemoveApiKey, piCustomProviders, piSaveCustomProvider, piRemoveCustomProvider, piSaveEnabledModels, piCheckUpdate, piPackageInstall, piPackageRemove, piPackageUpdate, type PiSettingsView, type PiAuthProviderView, type CustomProvider, type UpdateInfo } from '$lib/pi/api';
  let desktop = $state(false), busy = $state(''), error = $state(''), saved = $state('');
  let settings = $state<PiSettingsView | null>(null), auth = $state<PiAuthProviderView[]>([]), custom = $state<CustomProvider[]>([]);
  let catalog = $state<Array<{provider:string;id:string;name:string}>>([]), providerIds = $state<string[]>([]);
  let provider = $state(''), model = $state(''), thinking = $state(''), query = $state('');
  let keyProvider = $state(''), key = $state(''), customId = $state(''), endpoint = $state(''), api = $state('openai-completions'), modelIds = $state('');
  let source = $state(''), updates = $state<Record<string, UpdateInfo>>({});
  const scoped = $derived(settings?.enabledModels || []);
  const shown = $derived(catalog.filter(m => `${m.provider}/${m.id}`.toLowerCase().includes(query.toLowerCase())));
  async function action(label: string, fn: () => Promise<unknown>) {
    busy = label; error = ''; saved = '';
    try { await fn(); } catch(e) { error = String(e); } finally { busy = ''; }
  }
  async function load() {
    settings = await piReadSettings(); auth = await piListAuthProviders(); custom = await piCustomProviders();
    providerIds = [...new Set([...(await piListProviderIds()), ...auth.map(p=>p.id), ...custom.map(p=>p.id)])].sort();
    const list = [];
    for (const p of providerIds) {
      const raw = await piListModelsStore(p);
      const models = new Map<string,string>();
      for (const m of raw) { const o = m as Record<string,unknown>; if (typeof o.id === 'string') models.set(o.id, typeof o.name === 'string' ? o.name : o.id); }
      for (const id of custom.find(c=>c.id===p)?.models || []) if (!models.has(id)) models.set(id,id);
      for (const [id,name] of models) list.push({provider:p,id,name});
    }
    catalog = list; provider = settings.defaultProvider || ''; model = settings.defaultModel || ''; thinking = settings.defaultThinkingLevel || '';
  }
  async function toggle(p: string, id: string, enabled: boolean) {
    await action('Saving model scope…', async () => {
      const selected = new Set(catalog.filter(m=>inModelScope(m.provider,m.id,scoped)).map(m=>`${m.provider}/${m.id}`));
      // Preserve explicit IDs absent from today's catalog rather than deleting them.
      for (const pattern of scoped) if (!pattern.includes('*') && !pattern.includes('?')) selected.add(pattern);
      if (enabled) selected.add(`${p}/${id}`); else { selected.delete(`${p}/${id}`); selected.delete(id); }
      if (!selected.size) throw new Error('Keep at least one model enabled, or use All models. Pi treats an empty scope as all models.');
      await piSaveEnabledModels([...selected]); settings = await piReadSettings(); saved = 'Model scope saved. Console picker uses it on return; Pi cycling applies it on its next startup.';
    });
  }
  async function check(source?: string) { const result = await piCheckUpdate(source); updates = {...updates, [source || 'harness']:result}; }
  onMount(() => { desktop = hasDesktop(); if(desktop) void action('Loading configuration…', load); });
</script>

<div class="settings-page">
  <header><a href="/">‹ Pi Console</a><h1>Settings</h1><ThemeControl /></header>
  <p class="intro">Your models, providers and tools. Configuration stays on this machine.</p>
  {#if !desktop}<p class="banner" role="status">Disconnected preview — open the desktop app to manage your local Pi configuration. No credentials are loaded here.</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if saved}<p class="success" role="status">{saved}</p>{/if}
  {#if busy}<p role="status">{busy}</p>{/if}
  <fieldset disabled={!desktop || !!busy}>
    <section><h2>Startup model</h2><p>Startup defaults are separate from the active session's model.</p>
      <label>Provider<input list="providers" bind:value={provider} placeholder="anthropic" /></label>
      <datalist id="providers">{#each providerIds as p}<option value={p}></option>{/each}</datalist>
      <label>Model<input list="models" bind:value={model} placeholder="Model ID" /></label>
      <datalist id="models">{#each catalog.filter(m=>m.provider===provider) as m}<option value={m.id}>{m.name}</option>{/each}</datalist>
      <label>Thinking<select bind:value={thinking}>{#each ['', 'off','minimal','low','medium','high','xhigh','max'] as t}<option value={t}>{t || 'Default'}</option>{/each}</select></label>
      <button class="primary" onclick={()=>action('Saving…',async()=>{settings=await piPatchSettings({defaultProvider:provider,defaultModel:model,defaultThinkingLevel:thinking});saved='Startup defaults saved.';})}>Save defaults</button>
    </section>
    <section><h2>Provider credentials</h2><p>Keys are never read back into this page. OAuth credentials remain managed by <code>pi /login</code> and <code>/logout</code>.</p>
      {#each auth as p}<div class="row"><strong>{p.id}</strong><span>{p.authType}</span>{#if p.authType==='api_key'}<button onclick={()=>{if(confirm(`Remove the saved API key for ${p.id}?`)) void action('Removing key…',async()=>{await piRemoveApiKey(p.id);auth=await piListAuthProviders();});}}>Remove key</button>{/if}</div>{/each}
      <label>Provider ID<input list="providers" bind:value={keyProvider} /></label><label>API key<input type="password" autocomplete="off" bind:value={key} /></label>
      <button class="primary" disabled={!key.trim() || !keyProvider.trim()} onclick={()=>action('Saving key…',async()=>{await piSetApiKey(keyProvider.trim(),key.trim());key='';await load();saved='Key saved securely. Restart Pi Console to reload provider authentication.';})}>Save API key</button>
    </section>
    <section class="wide"><h2>Enabled models</h2><p>Controls the Console picker and Pi's Ctrl+P model cycling—not API access. Toggling expands existing patterns into the catalog shown here. Unknown explicit IDs are retained.</p>
      <div class="row"><input aria-label="Search models" placeholder="Search providers or model IDs…" bind:value={query}/><button onclick={()=>action('Resetting scope…',async()=>{await piSaveEnabledModels([]);settings=await piReadSettings();})}>All models</button></div>
      {#if scoped.length}<p>Saved scope: <code>{scoped.join(', ')}</code></p>{/if}
      <div class="model-list">{#each shown as m}<label class="model"><input type="checkbox" checked={inModelScope(m.provider,m.id,scoped)} onchange={(e)=>toggle(m.provider,m.id,e.currentTarget.checked)}/><span>{m.name}<small>{m.provider}/{m.id}</small></span></label>{:else}<p>No catalog models. Configure a custom provider below, or refresh Pi's catalog with <code>pi update --models</code>.</p>{/each}</div>
    </section>
    <section class="wide"><h2>Custom providers</h2><p>Add OpenAI-compatible local servers or other supported endpoints. Existing headers, auth configuration and model metadata are preserved when editing. Save credentials separately above. Use a dummy API key for a keyless local server.</p>
      {#each custom as p}<div class="row"><strong>{p.id}</strong><span>{p.baseUrl}</span><button onclick={()=>{customId=p.id;endpoint=p.baseUrl || '';api=p.api || 'openai-completions';modelIds=p.models.join('\n');}}>Edit</button><button onclick={()=>{if(confirm(`Remove custom configuration for ${p.id}? Saved credentials are not removed.`)) void action('Removing provider…',async()=>{await piRemoveCustomProvider(p.id);await load();});}}>Remove</button></div>{/each}
      <div class="fields"><label>Provider ID<input bind:value={customId} placeholder="local-ollama" /></label><label>Endpoint URL<input type="url" bind:value={endpoint} placeholder="http://localhost:11434/v1" /></label><label>API<select bind:value={api}>{#each ['openai-completions','openai-responses','anthropic-messages','google-generative-ai'] as a}<option>{a}</option>{/each}</select></label></div>
      <label>Model IDs (one per line)<textarea rows="3" bind:value={modelIds} placeholder="qwen2.5-coder:7b"></textarea></label>
      <button class="primary" disabled={!customId.trim() || !endpoint.trim() || !modelIds.trim()} onclick={()=>action('Saving provider…',async()=>{await piSaveCustomProvider(customId.trim(),endpoint.trim(),api,[...new Set(modelIds.split('\n').map(x=>x.trim()).filter(Boolean))]);await load();saved='Custom provider saved. Restart Pi Console to reload the catalog.';})}>Save provider</button>
    </section>
    <section class="wide"><h2>Extensions & packages</h2><p>Packages execute with full system access. Review source before installing. Update checks contact the public npm registry only when you click Check. Pinned, git and local sources are never reported as up to date without verification.</p>
      <div class="row"><input aria-label="Package source" bind:value={source} placeholder="npm:package or git:host/repository"/><button class="primary" disabled={!source.trim()} onclick={()=>{if(confirm('Install this package with full system access?')) void action('Installing…',async()=>{const s=source.trim();await piPackageInstall(/^(npm:|git:|https?:|ssh:|\/|\.\/|\.\.\/)/.test(s)?s:`npm:${s}`);source='';await load();});}}>Install</button></div>
      {#each settings?.packages || [] as pkg}<div class="package"><div class="row"><strong>{pkg}</strong><button onclick={()=>action('Checking version…',()=>check(pkg))}>Check</button>{#if updates[pkg]?.available}<button class="primary" onclick={()=>action('Updating…',async()=>{await piPackageUpdate(pkg);await check(pkg);saved='Update command completed; version rechecked. Restart Pi Console to load package changes.';})}>Update</button>{/if}<button onclick={()=>{if(confirm(`Remove ${pkg}?`)) void action('Removing…',async()=>{await piPackageRemove(pkg);await load();});}}>Remove</button></div>{#if updates[pkg]}<p>{updates[pkg].installed || 'Unknown installed version'}{updates[pkg].latest ? ` → ${updates[pkg].latest}` : ''} · {updates[pkg].note}</p>{/if}</div>{:else}<p>No configured packages.</p>{/each}
    </section>
    <section class="wide"><h2>Pi harness</h2><p>Check installed Pi against the official latest release. Upgrades remain an explicit terminal action using your installation channel.</p><button onclick={()=>action('Checking Pi version…',()=>check())}>Check harness update</button>{#if updates.harness}<p role="status"><strong>{updates.harness.available?'Update available':'Version check'}</strong> · Installed {updates.harness.installed || 'unknown'} · Latest {updates.harness.latest || 'unknown'}</p><p>{updates.harness.note}</p>{/if}</section>
  </fieldset>
</div>
<style>
  .settings-page{max-width:1080px;margin:auto;padding:35px 28px 60px}header{display:flex;align-items:center;gap:24px}h1{font-size:28px;flex:1;font-weight:550}h2{font-size:15px;margin:0;color:var(--accent)}p{font-size:12px;color:var(--muted);line-height:1.7}.intro{font-size:14px;margin-bottom:30px}.banner{padding:16px;border:1px solid var(--border)}.error{color:var(--bad)}.success{color:var(--good)}fieldset{border:0;padding:0;display:grid;grid-template-columns:1fr 1fr;gap:20px;min-width:0}section{background:var(--glass);border:1px solid var(--border-soft);border-radius:7px;padding:23px;min-width:0}.wide{grid-column:1/-1}label{display:grid;gap:6px;font-size:12px;margin:12px 0}input,select,textarea{width:100%;min-width:0}.row{display:flex;gap:12px;align-items:center;padding:8px 0;flex-wrap:wrap}.row strong{flex:1;overflow-wrap:anywhere;font-size:13px}.row span{color:var(--muted);font-size:12px;overflow-wrap:anywhere}.row>input{flex:1;min-width:150px}.model-list{max-height:330px;overflow:auto;display:grid;grid-template-columns:1fr 1fr;gap:8px}.model{display:flex;align-items:center;margin:0;padding:9px;background:var(--glass-soft)}.model input{width:auto;accent-color:var(--accent)}small{display:block;color:var(--muted);overflow-wrap:anywhere}.fields{display:grid;grid-template-columns:1fr 2fr 1fr;gap:16px}.package{border-bottom:1px solid var(--border-soft);padding:10px 0}code{overflow-wrap:anywhere}@media(max-width:700px){fieldset,.model-list,.fields{grid-template-columns:1fr}.settings-page{padding:20px 15px}header{gap:12px}section{padding:16px}}
</style>
