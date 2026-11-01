<script lang="ts">
  import type { ShelfItem } from '$lib/ipc/bindings/ShelfItem';
  import { shelfCommands } from '$lib/ipc/commands';
  import { dragFilesOut } from '$lib/ipc/drag';
  import { pickShelfFiles, saveExport } from '$lib/ipc/dialog';
  import { t } from '$lib/i18n/index.svelte';
  let { items, receiving = false }: { items: ShelfItem[]; receiving?: boolean } = $props();
  let selected = $state<string[]>([]);
  let anchor = '';
  let page = $state(0);
  let error = $state('');
  let busy = $state(false);
  let progress = $state({completed: 0, total: 0, name: ''});
  let format = $state('png');
  let edge = $state(1920);
  const chosen = $derived(items.filter(i => selected.includes(i.id)));
  const targets = $derived(chosen.length ? chosen : items);
  const valid = $derived(targets.length > 0 && targets.every(i => !i.missing));
  // Bound rendering, never storage. Bulk actions span every page.
  const pages = $derived(Math.max(1, Math.ceil(items.length / 50)));
  const visible = $derived(items.slice(Math.min(page, pages - 1) * 50, (Math.min(page, pages - 1) + 1) * 50));
  async function run(action: () => Promise<unknown>) {
    error = ''; try { await action(); } catch (e) { error = typeof e === 'object' && e && 'message' in e ? String(e.message) : String(e); }
  }
  function select(item: ShelfItem, event: MouseEvent) {
    if (event.shiftKey && anchor) {
      const a = items.findIndex(i => i.id === anchor), b = items.indexOf(item);
      selected = items.slice(Math.min(a,b), Math.max(a,b)+1).map(i=>i.id);
    } else if (event.ctrlKey || event.metaKey) selected = selected.includes(item.id) ? selected.filter(id=>id!==item.id) : [...selected,item.id];
    else selected = [item.id];
    anchor = item.id;
  }
  async function zip() {
    const destination = await saveExport('PCisland-files', 'zip'); if (!destination) return;
    busy = true; progress = {completed: 0,total: 0,name: ''};
    await run(() => shelfCommands.zip(targets.map(i=>i.id), destination, p=>progress=p)); busy = false;
  }
  async function convert() {
    if (chosen.length !== 1) return;
    const destination = await saveExport(chosen[0]!.name.replace(/\.[^.]+$/, '') + '-converted', format === 'jpeg' ? 'jpg' : format);
    if (!destination) return;
    busy = true; await run(() => shelfCommands.convert(chosen[0]!.id,destination,format,edge)); busy = false;
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions (Shortcuts bubble from the file buttons.) -->
<section role="group" class="shelf" aria-label={t('main.nav.shelf')} tabindex="-1" onkeydown={(e) => {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'a') { e.preventDefault(); selected = items.map(i=>i.id); }
  if (e.key === 'Escape' && selected.length) { e.stopPropagation(); selected = []; }
}}>
  <div class="toolbar">
    <button disabled={busy} onclick={() => run(async () => { const paths = await pickShelfFiles(); if(paths.length) await shelfCommands.add(paths); })}>{t('shelf.addFiles')}</button>
    <button onclick={() => selected = chosen.length === items.length ? [] : items.map(i=>i.id)}>{t('shelf.selectAll')}</button>
    <span>{chosen.length} / {items.length}</span>
  </div>
  <div class="files" class:receiving>
    {#each visible as item (item.id)}
      <div class="row" class:missing={item.missing}>
        <button class="file" aria-pressed={selected.includes(item.id)} title={item.path} onclick={e => select(item,e)}>
          {#if item.thumbnailUrl}<img loading="lazy" src={item.thumbnailUrl} alt="" draggable="false" />{:else}<span aria-hidden="true">{item.isDir ? '▣' : '▤'}</span>{/if}
          <span class="name">{item.name}<small>{item.missing ? t('shelf.missingFile') : item.isDir ? t('shelf.folder') : `${Math.ceil(item.size/1024)} KB`}</small></span>
        </button>
        <button disabled={item.missing || receiving} title={t('shelf.dragSelected')} aria-label={`${t('shelf.dragSelected')}: ${item.name}`} onmousedown={e=>{ if(e.button===0) { e.stopPropagation(); void run(()=>dragFilesOut(selected.includes(item.id) ? chosen.filter(i=>!i.missing).map(i=>i.path) : [item.path])); }}}>↗</button>
      </div>
    {:else}<p>{t('shelf.dropHere')}</p>{/each}
  </div>
  {#if pages > 1}<div class="toolbar"><button disabled={page===0} onclick={()=>page--}>←</button><span>{Math.min(page,pages-1)+1}/{pages}</span><button disabled={page>=pages-1} onclick={()=>page++}>→</button></div>{/if}
  <div class="toolbar">
    <button disabled={!valid || receiving} onmousedown={e=>{if(e.button===0)void run(()=>dragFilesOut(targets.map(i=>i.path)));}}>{t(chosen.length ? 'shelf.dragSelected' : 'shelf.dragAll')}</button>
    <button disabled={!valid} onclick={()=>run(()=>shelfCommands.copy(targets.map(i=>i.id)))}>{t('shelf.copyFiles')}</button>
    <button disabled={!valid || busy} onclick={zip}>ZIP</button>
    {#if chosen.length===1}
      <button disabled={!valid} onclick={()=>run(()=>shelfCommands.open(chosen[0]!.path))}>{t('shelf.openFile')}</button>
      <button onclick={()=>run(()=>shelfCommands.open(chosen[0]!.path,true))}>{t('shelf.reveal')}</button>
    {/if}
    <button disabled={!targets.length || busy} onclick={()=>run(async()=>{ await shelfCommands.removeMany(targets.map(i=>i.id)); selected=[]; })}>{t(chosen.length ? 'shelf.removeSelected' : 'shelf.action.clear')}</button>
  </div>
  {#if chosen.length===1 && /\.(png|jpe?g)$/i.test(chosen[0]!.name)}
    <div class="toolbar"><select aria-label={t('shelf.format')} bind:value={format}><option value="png">PNG</option><option value="jpeg">JPEG</option><option value="webp">WebP</option></select><input type="number" min="64" max="8192" step="1" bind:value={edge} aria-label={t('shelf.maxEdge')} /><button disabled={busy} onclick={convert}>{t('shelf.convert')}</button></div>
  {/if}
  {#if busy}<progress max={progress.total || 1} value={progress.completed}></progress><span class="status">{progress.name}</span><button onclick={()=>shelfCommands.cancel()}>{t('common.cancel')}</button>{/if}
  {#if error}<p role="alert">{error}</p>{/if}
  <p class="hint">{t('shelf.multiHint')}</p>
</section>

<style>
  .shelf { display:flex; flex-direction:column; gap:var(--sp-3); min-width:0; padding:var(--sp-3); font-size:var(--fs-small); }
  .toolbar { display:flex; gap:var(--sp-2); flex-wrap:wrap; align-items:center; }
  button,select,input { border:1px solid var(--c-border-strong); border-radius:var(--r-control); background:var(--c-surface-2); color:var(--c-text-1); font:inherit; padding:var(--sp-2) var(--sp-3); }
  button { cursor:pointer; } button:disabled { opacity:.4; cursor:default; }
  .files { overflow-y:auto; max-height:var(--island-list-max-h); min-height:var(--shelf-thumb); }
  .row { display:flex; gap:var(--sp-2); margin-bottom:var(--sp-2); }
  .file { display:flex; align-items:center; text-align:left; gap:var(--sp-3); flex:1; min-width:0; }
  .file[aria-pressed=true] { border-color:var(--c-accent); background:var(--c-accent-soft); }
  img { width:var(--sp-8); height:var(--sp-8); object-fit:contain; }
  .name { flex:1; min-width:0; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
  small { display:block; color:var(--c-text-3); }
  .missing { opacity:.5; } .receiving { outline:1px dashed var(--c-accent); }
  input { width:var(--shelf-slot); } p { margin:0; } .hint,.status { color:var(--c-text-2); font-size:var(--fs-caption); overflow-wrap:anywhere; } progress { width:100%; accent-color:var(--c-accent); }
</style>
