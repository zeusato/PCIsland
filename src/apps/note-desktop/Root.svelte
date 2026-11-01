<script lang="ts">
  import { onMount } from 'svelte';
  import { noteCommands } from '$lib/ipc/commands';
  import { onNotesState, onDesktopNoteSelect } from '$lib/ipc/events';
  import { currentWindow, onWindowClose } from '$lib/ipc/window';
  import type { Note } from '$lib/ipc/bindings/Note';
  import { t } from '$lib/i18n/index.svelte';
  import NoteBody from '../../features/notes/views/NoteBody.svelte';
  let note = $state<Note | null>(null);
  let id = $state('');
  let font = $state(18);
  let topmost = $state(true);
  let editing = $state(false);
  let draft = $state('');
  let version: number | undefined;
  let busy = $state(false);
  let error = $state('');
  let pending = $state<string | null>(null);
  let deleted = $state(false);
  let request = 0;
  async function load() {
    const r=++request; const all=await noteCommands.list(); if(r!==request)return;
    note = all.find(n=>n.id===id) ?? null;
    if(!editing){try{const saved=localStorage.getItem(`note.draft.${id}`);if(saved){const value=JSON.parse(saved);draft=value.text;version=value.version;editing=true;}}catch{/* invalid draft */}}
  }
  async function select(next: string) { if(editing) {pending=next;return;} id=next; deleted=false; await noteCommands.desktopOptions(font,topmost,id); await load(); }
  async function run(action: () => Promise<unknown>) { if(busy)return;busy=true;error='';try{await action();}catch{error=t('notes.conflict');}finally{busy=false;} }
  function edit() { if(!note)return;draft=note.text;version=note.updatedAt;editing=true; }
  async function save() { await run(async()=>{note=await noteCommands.save(draft,id,version);editing=false;localStorage.removeItem(`note.draft.${id}`);}); }
  async function options(delta=0) { font=Math.max(12,Math.min(40,font+delta)); await noteCommands.desktopOptions(font,topmost); }
  function discard(){localStorage.removeItem(`note.draft.${id}`);editing=false;}
  $effect(()=>{if(editing&&id)localStorage.setItem(`note.draft.${id}`,JSON.stringify({text:draft,version}));});
  function close() { if(editing)pending='close';else void currentWindow.close(); }
  onMount(()=>{
    void noteCommands.desktopState().then(s=>{id=s.id;font=s.font;topmost=s.topmost;return load();}).catch(()=>error=t('notes.failed'));
    const listeners=[onWindowClose(()=>{if(editing){pending='close';return false;}return true;}),onNotesState(()=>void load()),onDesktopNoteSelect(next=>void select(next))];
    return()=>{request++; for(const listener of listeners)void listener.then(off=>off());};
  });
</script>

<div class="window" style={`--note-font:${font}px`} onwheel={e=>{if(e.ctrlKey){e.preventDefault();void options(e.deltaY>0?-2:2);}}}>
  <header data-tauri-drag-region>
    <span data-tauri-drag-region>{t('notes.desktop')}</span>
    <button aria-pressed={topmost} onclick={()=>{topmost=!topmost;void options();}}>{t('notes.topmost')}</button>
    <button aria-label={t('notes.fontSmaller')} disabled={font<=12} onclick={()=>options(-2)}>A−</button>
    <span>{font}</span><button aria-label={t('notes.fontLarger')} disabled={font>=40} onclick={()=>options(2)}>A+</button>
    <button aria-label={t('main.titlebar.close')} onclick={close}>×</button>
  </header>
  <main>
    {#if editing}<textarea bind:value={draft} aria-label={t('common.edit')} disabled={busy}></textarea>
    {:else if note}<NoteBody noteId={note.id} text={note.text} disabled={busy} ontoggle={line=>run(async()=>{await noteCommands.toggleCheck(id,line);await load();})} />
    {:else}<p>{t(deleted ? 'notes.deletedDesktop' : 'notes.empty')}</p>{/if}
  </main>
  <footer>
    {#if editing}<button disabled={busy || !draft.trim()} onclick={save}>{t('common.save')}</button><button disabled={busy} onclick={discard}>{t('common.cancel')}</button>
    {:else if note}<button onclick={edit}>{t('common.edit')}</button><button onclick={()=>run(async()=>{await noteCommands.remove(id);deleted=true;await load();})}>{t('common.delete')}</button>
    {:else if deleted}<button onclick={()=>run(async()=>{await noteCommands.restore(id);deleted=false;await load();})}>{t('common.undo')}</button>{/if}
    {#if error}<p role="alert">{error}</p>{/if}
    {#if pending}<div class="confirm"><p>{t('notes.unsaved')}</p><button onclick={async()=>{await save();if(!editing){const next=pending;pending=null;if(next==='close')void currentWindow.close();else if(next)await select(next);}}}>{t('common.save')}</button><button onclick={()=>{discard();const next=pending;pending=null;if(next==='close')void currentWindow.close();else if(next)void select(next);}}>{t('notes.discard')}</button><button onclick={()=>pending=null}>{t('common.cancel')}</button></div>{/if}
  </footer>
</div>
<style>
  .window { display:flex; flex-direction:column; height:100vh; background:var(--c-surface-1); border:1px solid var(--c-border-strong); }
  header,footer { padding:var(--sp-3); display:flex; gap:var(--sp-2); flex-wrap:wrap; align-items:center; }
  header { border-bottom:1px solid var(--c-border); font-size:var(--fs-caption); }
  header>span:first-child { flex:1; }
  main { flex:1; overflow:auto; min-height:0; padding:var(--sp-5); font-size:var(--note-font); --fs-caption:var(--note-font); }
  textarea { width:100%; height:100%; resize:none; font:inherit; color:inherit; background:var(--c-surface-2); border:1px solid var(--c-accent); user-select:text; }
  button { padding:var(--sp-2) var(--sp-3); border:1px solid var(--c-border-strong); border-radius:var(--r-control); background:var(--c-surface-2); color:inherit; cursor:pointer; }
  button[aria-pressed=true] { color:var(--c-accent); } button:disabled { opacity:.4; }
  footer { border-top:1px solid var(--c-border); } p { margin:0; } .confirm { width:100%; } [role=alert] { color:var(--c-danger); }
</style>
