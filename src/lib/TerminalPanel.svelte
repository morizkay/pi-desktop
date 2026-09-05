<script lang="ts">
  import { onMount } from 'svelte';
  import { hasDesktop, terminalStart, terminalPoll, terminalWrite, terminalResize, terminalClose } from '$lib/pi/api';
  import type { Terminal } from '@xterm/xterm';
  import '@xterm/xterm/css/xterm.css';
  let {cwd,visible,onhide,onrunning}: {cwd:string;visible:boolean;onhide:()=>void;onrunning:(active:boolean)=>void} = $props();
  let host:HTMLDivElement, term:Terminal|undefined, timer:ReturnType<typeof setTimeout>|undefined;
  let id=$state<number|null>(null), terminalCwd=$state(''), error=$state(''), status=$state('Not started'), starting=$state(false), desktop=$state(false);
  let disposed=false, resizeObserver:ResizeObserver|undefined, input=Promise.resolve(), queued=0;
  async function start() {
    starting=true;error='';
    try {
      const [{Terminal},{FitAddon}]=await Promise.all([import('@xterm/xterm'),import('@xterm/addon-fit')]);
      if(disposed)return;
      term?.dispose();
      term=new Terminal({fontSize:12,fontFamily:'SFMono-Regular,Consolas,monospace',scrollback:3000,convertEol:false,theme:{background:'#121414',foreground:'#e6e7e3',cursor:'#efb63e'}});
      const fit=new FitAddon();term.loadAddon(fit);term.open(host);fit.fit();
      const opened=await terminalStart(Math.min(term.cols,500),Math.min(term.rows,300));
      if(disposed){await terminalClose(opened.id);return;}
      id=opened.id;terminalCwd=opened.cwd;status='Running';onrunning(true);
      term.onData(data=>{
        if(id===null)return;
        if(queued+data.length>32768){error='Input queue full. Paste a smaller block; this input was not sent.';return;}
        const target=id;queued+=data.length;
        input=input.then(async()=>{for(const chunk of data.match(/[\s\S]{1,1024}/gu)||[]){if(id!==target)return;await terminalWrite(target,chunk);}}).catch(e=>{error=`Input failed; part of a paste may have arrived. Check the terminal before retrying. ${e}`;}).finally(()=>{queued-=data.length;});
      });
      resizeObserver?.disconnect();
      resizeObserver=new ResizeObserver(()=>{if(!term||id===null||!host.clientWidth)return;fit.fit();void terminalResize(id,Math.min(term.cols,500),Math.min(term.rows,300)).catch(e=>{error=String(e);});});resizeObserver.observe(host);
      term.focus();void poll(opened.id);
    }catch(e){error=String(e);status='Failed';}finally{starting=false;}
  }
  async function poll(target:number) {
    if(disposed||id!==target)return;
    try{
      const result=await terminalPoll(target);
      if(disposed||id!==target)return;
      if(result.bytes.length)await new Promise<void>(resolve=>term!.write(new Uint8Array(result.bytes),resolve));
      if(result.exited){id=null;onrunning(false);status='Exited';await terminalClose(target);return;}
      timer=setTimeout(()=>void poll(target),40);
    }catch(e){if(!disposed){error=String(e);status='Disconnected';}}
  }
  async function close() {
    if(id!==null&&!confirm('Close this shell and terminate its active job?'))return;
    const target=id;
    try{if(target!==null)await terminalClose(target);id=null;onrunning(false);clearTimeout(timer);onhide();}catch(e){error=String(e);}
  }
  onMount(()=>{desktop=hasDesktop();return()=>{disposed=true;clearTimeout(timer);resizeObserver?.disconnect();term?.dispose();if(id!==null)void terminalClose(id).catch(()=>{});};});
</script>
<section class="terminal-panel" class:hidden={!visible} aria-label="Terminal panel">
  <header><strong>Terminal</strong><span title={terminalCwd||cwd}>{terminalCwd||cwd||'No workspace'}</span><small>{status}</small>{#if id===null}<button disabled={!desktop||starting||!cwd} onclick={start}>{starting?'Starting…':'Start shell'}</button>{/if}<button disabled={starting} aria-label="Close terminal" onclick={close}>×</button></header>
  {#if terminalCwd && terminalCwd!==cwd}<p class="warning">Shell remains in {terminalCwd}. Close and reopen it to use the new workspace.</p>{/if}
  {#if error}<p class="warning" role="alert">{error}</p>{/if}
  {#if !desktop}<p class="hint">Desktop only. No shell runs in browser preview.</p>{:else if !terminalCwd}<p class="hint">Start a real shell in this workspace. Commands run with your user permissions and are not sent to Pi.</p>{/if}
  <div class="terminal-host" bind:this={host}></div>
</section>
<style>
  .hidden{display:none!important}
  .terminal-panel{flex:0 0 280px;min-height:180px;max-height:50vh;border-top:1px solid var(--border);background:#121414;color:#e6e7e3;display:flex;flex-direction:column;resize:vertical;overflow:hidden}header{display:flex;align-items:center;gap:12px;padding:7px 14px;border-bottom:1px solid #343837;font-size:12px}header span{flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:#959b98}button{padding:3px 8px;color:#e6e7e3;background:#222626;border:1px solid #343837}.terminal-host{flex:1;min-height:0;padding:8px}.warning,.hint{margin:5px 14px;font-size:11px;color:#efb63e}.hint{color:#959b98}
</style>
