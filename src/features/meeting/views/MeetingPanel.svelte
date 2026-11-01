<script lang="ts">
  import { onMount } from 'svelte';
  import Dropdown from '$lib/components/form/Dropdown.svelte';
  import Mic from '@lucide/svelte/icons/mic';
  import Volume2 from '@lucide/svelte/icons/volume-2';
  import Sparkles from '@lucide/svelte/icons/sparkles';
  import { meetingCommands, shelfCommands } from '$lib/ipc/commands';
  import { saveExport } from '$lib/ipc/dialog';
  import { t } from '$lib/i18n/index.svelte';
  import { meetingStore, startMeetingStore } from '../store.svelte';
  let {compact=false}:{compact?:boolean}=$props();
  const snapshot=$derived(meetingStore.snapshot);
  let selected=$state('');
  const meeting=$derived(snapshot?.meetings.find(m=>m.id===(selected || snapshot.active)) ?? snapshot?.meetings[0]);
  const active=$derived(snapshot?.meetings.find(m=>m.id===snapshot.active));
  let devices=$state<Array<{id:string;name:string;system:boolean}>>([]);let micDevice=$state('');let systemDevice=$state('');
  let micMenu=$state(false);let systemMenu=$state(false);let historyMenu=$state(false);
  $effect(()=>{if(micMenu)systemMenu=false;});
  $effect(()=>{if(systemMenu)micMenu=false;});
  let title=$state('');let mic=$state(true);let system=$state(true);let transcribe=$state(false);
  let busy=$state(false);let error=$state('');let now=$state(Date.now());
  let draft=$state('');let draftId=$state('');let draftRevision=0;let dirty=$state(false);let saving=$state(false);let noteError=$state(false);
  let playing=$state('');let editId=$state('');let editText=$state('');
  const key=(id:string)=>`meeting.draft.${id}`;
  const elapsed=$derived(active ? Math.max(0,now-active.startedAt) : 0);
  const time=(ms:number)=>{const s=Math.floor(ms/1000);return `${Math.floor(s/3600).toString().padStart(2,'0')}:${Math.floor(s/60)%60}`.replace(/:(\d)$/,':0$1')+`:${(s%60).toString().padStart(2,'0')}`;};
  async function run(action:()=>Promise<unknown>){if(busy)return;busy=true;error='';try{await action();}catch(e){error=typeof e==='object'&&e&&'message'in e?String(e.message):String(e);}finally{busy=false;}}
  async function saveNotes(){
    if(!dirty||saving||!draftId)return;saving=true;const id=draftId,text=draft,revision=draftRevision;
    try{await meetingCommands.notes(id,text,revision);if(draftId===id){draftRevision=revision+1;if(draft===text){dirty=false;localStorage.removeItem(key(id));}}noteError=false;}
    catch{noteError=true;}finally{saving=false;}
  }
  $effect(()=>{
    const m=meeting;if(!m||compact)return;
    if(draftId!==m.id){draftId=m.id;const saved=localStorage.getItem(key(m.id));let recovered:{text:string;revision:number}|null=null;try{recovered=saved?JSON.parse(saved):null;}catch{/* old local draft */}
      draft=recovered?.text??m.notes;draftRevision=recovered?.revision??m.noteRevision;dirty=!!recovered;noteError=false;
    }else if(!dirty&&!saving){draft=m.notes;draftRevision=m.noteRevision;}
  });
  function changed(){dirty=true;noteError=false;localStorage.setItem(key(draftId),JSON.stringify({text:draft,revision:draftRevision}));}
  $effect(()=>{if(!dirty||noteError||saving)return;void draft;const timer=setTimeout(()=>void saveNotes(),800);return()=>clearTimeout(timer);});
  $effect(()=>{if(!active)return;const timer=setInterval(()=>now=Date.now(),1000);return()=>clearInterval(timer);});
  onMount(()=>{void startMeetingStore();void meetingCommands.devices().then(d=>devices=d).catch(()=>{});try{const prefs=JSON.parse(localStorage.getItem('meeting.sources')??'{}');mic=prefs.mic??true;system=prefs.system??true;micDevice=prefs.micDevice??'';systemDevice=prefs.systemDevice??'';}catch{/* defaults */}});
  async function start(){localStorage.setItem('meeting.sources',JSON.stringify({mic,system,micDevice,systemDevice}));await run(async()=>{selected=await meetingCommands.start(title,mic,system,transcribe,micDevice||null,systemDevice||null);});}
  async function exportMeeting(){if(!meeting)return;const destination=await saveExport(`meeting-${meeting.id}`,'zip');if(destination)await run(()=>meetingCommands.export(meeting.id,destination));}
</script>

<div class="meeting" class:compact>
  <div class="controls">
    {#if active}
      <strong class="recording">● {t('meeting.recording')} · {time(elapsed)}</strong>
      <button class="accent" disabled={busy} onclick={()=>run(()=>meetingCommands.mark())}>{t(active.highlights.at(-1)?.endMs===null?'meeting.endMark':'meeting.mark')}</button>
      <button disabled={busy} onclick={()=>run(()=>meetingCommands.stop())}>{t('meeting.stop')}</button>
      <small>{t('meeting.shortcutHint')}</small>
      <div class="meters"><label>{t('meeting.mic')}<meter min="0" max="1" value={snapshot?.micLevel??0}></meter></label><label>{t('meeting.system')}<meter min="0" max="1" value={snapshot?.systemLevel??0}></meter></label></div>
    {:else}
      <input class="title-input" aria-label={t('meeting.title')} placeholder={t('meeting.title')} bind:value={title} />
      <div class="source-toggles" aria-label={t('meeting.sources')}>
        <button class="source-toggle" class:enabled={mic} aria-pressed={mic} onclick={()=>mic=!mic}><Mic size={16}/>{t('meeting.microphone')}<span class="source-dot"></span></button>
        <button class="source-toggle" class:enabled={system} aria-pressed={system} onclick={()=>system=!system}><Volume2 size={16}/>{t('meeting.computer')}<span class="source-dot"></span></button>
      </div>
      <div class="device-grid">
        <div class="device-field"><span class="field-label">{t('meeting.inputDevice')}</span>
          <Dropdown label={t('meeting.mic')} bind:value={micDevice} bind:open={micMenu} disabled={!mic}
            options={[{value:'',label:t('meeting.defaultDevice'),description:t('meeting.defaultDeviceHint')},...devices.filter(d=>!d.system).map(d=>({value:d.id,label:d.name}))]} />
        </div>
        <div class="device-field"><span class="field-label">{t('meeting.outputDevice')}</span>
          <Dropdown label={t('meeting.system')} bind:value={systemDevice} bind:open={systemMenu} disabled={!system}
            options={[{value:'',label:t('meeting.defaultDevice'),description:t('meeting.defaultDeviceHint')},...devices.filter(d=>d.system).map(d=>({value:d.id,label:d.name}))]} />
        </div>
      </div>
      <p class="hint source-hint">{t('meeting.systemHint')}</p>
      <label class="consent">
        <Sparkles size={16}/><span class="consent-copy"><strong>{t('meeting.autoTranscript')}</strong><small>{t('meeting.consentShort')}</small></span>
        <input type="checkbox" bind:checked={transcribe} disabled={!snapshot?.keyConfigured} aria-label={t('meeting.consent')}/>
      </label>
      {#if snapshot&&!snapshot.keyConfigured}<small>{t('meeting.noKey')}</small>{/if}
      <button class="accent start" disabled={busy||(!mic&&!system)} onclick={start}><Mic size={16}/>{t('meeting.start')}</button>
    {/if}
  </div>
  {#if error}<p role="alert">{error}</p>{/if}
  {#if !compact}
    <div class="toolbar">
      <div class="history-picker"><Dropdown label={t('meeting.history')} value={meeting?.id??''} bind:open={historyMenu}
        options={(snapshot?.meetings??[]).map(m=>({value:m.id,label:m.title,description:new Date(m.startedAt).toLocaleString()}))}
        onchange={async value=>{await saveNotes();selected=value;playing='';editId='';}} /></div>
      <button disabled={!meeting||busy||['recording','finalizing'].includes(meeting.status)} onclick={exportMeeting}>{t('meeting.export')}</button>
      {#if meeting}<button onclick={()=>run(()=>shelfCommands.open(meeting.folder))}>{t('shelf.reveal')}</button>{/if}
    </div>
    {#if meeting}
      <div class="workspace">
        <section class="timeline" aria-label={t('meeting.log')}>
          <div class="log-head"><strong>{meeting.title}</strong><span>{t(`meeting.status.${meeting.status}`)} · {time(meeting.durationMs)}</span></div>
          {#if meeting.error}<p role="alert">{meeting.error.includes('0x80070005')?t('meeting.audioDenied'):meeting.error}</p>{/if}
          {#if meeting.fullFile}<button onclick={()=>playing=`http://meetingaudio.localhost/${meeting.id}/full`}>{t('meeting.listenFull')}</button>{/if}
          {#if playing}<audio controls src={playing} autoplay></audio>{/if}
          {#each meeting.highlights as clip(clip.id)}
            <article class:marking={clip.endMs===null}>
              <header><strong>{time(clip.startMs)} → {clip.endMs===null?'●':time(clip.endMs)}</strong><span>{t(`meeting.clip.${clip.status}`)}</span></header>
              <div class="toolbar">
                <button disabled={clip.status!=='ready'} onclick={()=>playing=`http://meetingaudio.localhost/${meeting.id}/${clip.id}`}>{t('meeting.listen')}</button>
                <button disabled={clip.status!=='ready'||['queued','running'].includes(clip.transcriptStatus)} onclick={()=>run(()=>meetingCommands.retry(meeting.id,clip.id))}>{t('meeting.transcribe')}</button>
                {#if clip.transcript}<button onclick={()=>{editId=clip.id;editText=clip.editedText??clip.transcript;}}>{t('common.edit')}</button>{/if}
              </div>
              {#if editId===clip.id}<textarea bind:value={editText} aria-label={t('meeting.transcript')}></textarea><button onclick={()=>run(async()=>{await meetingCommands.editTranscript(meeting.id,clip.id,editText);editId='';})}>{t('common.save')}</button><button onclick={()=>editId=''}>{t('common.cancel')}</button>
              {:else if clip.transcript}<p class="transcript">{clip.editedText??clip.transcript}</p><small>{t(clip.editedText===null?'meeting.machineDraft':'meeting.userEdited')}</small>{/if}
              {#if clip.transcriptStatus!=='off'}<small>{t(`meeting.transcript.${clip.transcriptStatus}`)}</small>{/if}
              {#if clip.error}<p role="alert">{clip.error}</p>{/if}
            </article>
          {:else}<p class="hint">{t('meeting.markHint')}</p>{/each}
        </section>
        <section class="notes"><label for="meeting-notes">{t('meeting.personalNotes')}</label><textarea id="meeting-notes" bind:value={draft} oninput={changed} onblur={saveNotes} placeholder={t('meeting.noteHint')}></textarea><div class="toolbar"><small>{t(noteError?'notes.conflict':saving?'meeting.saving':dirty?'meeting.unsaved':'meeting.saved')}</small><button disabled={!dirty||saving} onclick={saveNotes}>{t('common.save')}</button></div></section>
      </div>
    {:else}<p class="hint">{t('meeting.empty')}</p>{/if}
  {/if}
</div>
<style>
  .meeting { display:flex; flex-direction:column; gap:var(--sp-5); min-width:0; width:100%; }
  .controls { display:grid; grid-template-columns:minmax(0,1fr); gap:var(--sp-5); padding:var(--sp-6); background:var(--c-surface-2); border-radius:var(--r-card); min-width:0; }
  .compact .controls { padding:var(--sp-5); gap:var(--sp-4); font-size:var(--fs-body-island); }
  button,input,textarea { font:inherit; color:inherit; background:var(--c-surface-2); border:1px solid var(--c-border-strong); border-radius:var(--r-control); padding:var(--sp-3); min-width:0; }
  button { cursor:pointer; } button:disabled { opacity:.4; cursor:default; } .accent { border-color:var(--c-accent); color:var(--c-accent-bright); }
  .recording { color:var(--c-accent-bright); } .hint,small { color:var(--c-text-2); font-size:var(--fs-caption); } .hint { margin:0; }
  .toolbar,.meters { display:flex; gap:var(--sp-3); flex-wrap:wrap; align-items:center; } label { display:flex; align-items:center; gap:var(--sp-2); } .consent { font-size:var(--fs-small); }
  .title-input { width:100%; min-height:var(--sp-10); background:var(--c-surface-1); }
  .source-toggles { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:var(--sp-4); }
  .source-toggle { display:flex; align-items:center; gap:var(--sp-4); min-height:var(--sp-10); padding:var(--sp-4) var(--sp-5); color:var(--c-text-3); }
  .source-toggle.enabled { color:var(--c-text-1); background:var(--c-accent-soft); border-color:var(--c-accent); }
  .source-dot { width:var(--sp-3); height:var(--sp-3); margin-left:auto; border-radius:var(--r-pill); background:var(--c-text-3); }
  .enabled .source-dot { background:var(--c-accent); }
  .device-grid { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:var(--sp-5); min-width:0; }
  .compact .device-grid { grid-template-columns:minmax(0,1fr); gap:var(--sp-4); }
  .device-field { min-width:0; } .field-label { display:block; color:var(--c-text-2); font-size:var(--fs-caption); margin-bottom:var(--sp-3); }
  .consent { padding-top:var(--sp-5); border-top:1px solid var(--c-border); gap:var(--sp-4); }
  .consent-copy { display:flex; flex-direction:column; gap:var(--sp-2); flex:1; min-width:0; } .consent-copy strong { font-weight:var(--fw-medium); }
  .consent :global(svg) { color:var(--c-accent-bright); flex:none; }
  .consent input { width:var(--sp-6); height:var(--sp-6); accent-color:var(--c-accent); flex:none; }
  .source-hint { line-height:1.5; }
  .start { display:flex; align-items:center; justify-content:center; gap:var(--sp-4); justify-self:start; min-height:var(--sp-10); padding-inline:var(--sp-7); background:var(--c-accent); color:var(--c-island); font-weight:var(--fw-semibold); border:0; }
  .compact .start { width:100%; } .start:hover:enabled { background:var(--c-accent-bright); }
  .history-picker { flex:1; min-width:0; }
  .workspace { display:grid; grid-template-columns:minmax(0,1.2fr) minmax(0,1fr); gap:var(--sp-6); align-items:start; }
  .timeline { display:flex; flex-direction:column; gap:var(--sp-4); min-width:0; }
  .log-head { display:flex; flex-direction:column; gap:var(--sp-2); }
  article { border-left:2px solid var(--c-border-strong); padding:var(--sp-4); background:var(--c-surface-2); border-radius:var(--r-control); }
  article.marking { border-color:var(--c-accent); } article header { display:flex; justify-content:space-between; gap:var(--sp-4); margin-bottom:var(--sp-3); } article small { display:block; }
  .notes { display:flex; flex-direction:column; gap:var(--sp-3); position:sticky; top:var(--sp-4); }
  textarea { width:100%; min-height:var(--island-list-max-h); resize:vertical; user-select:text; line-height:1.6; } article textarea { min-height:var(--shelf-slot); }
  .transcript { white-space:pre-wrap; overflow-wrap:anywhere; line-height:1.6; } audio { width:100%; } [role=alert] { color:var(--c-danger); overflow-wrap:anywhere; }
</style>
