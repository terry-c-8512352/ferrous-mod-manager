<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { DecoratedMod } from './catalog';

  interface Props {
    mods: DecoratedMod[];
    enabledCount: number;
    ontoggle: (modId: string) => void;
    onmove: (from: number, to: number) => void;
    onremove: (modId: string) => void;
  }

  let { mods, enabledCount, ontoggle, onmove, onremove }: Props = $props();

  let dragIndex = $state<number | null>(null);
  let overIndex = $state<number | null>(null);

  // Row index whose load-number input is being edited. While editing, that row
  // is made non-draggable so the text field stays focusable/typable (a draggable
  // ancestor otherwise swallows input interaction under WebKitGTK).
  let editIndex = $state<number | null>(null);

  // Array indices of the enabled mods, in order. The visible load number N maps
  // to enabledOrder[N - 1] — the collection-array index onmove expects.
  const enabledOrder = $derived.by(() => {
    const out: number[] = [];
    mods.forEach((m, i) => {
      if (m.enabled) out.push(i);
    });
    return out;
  });

  // Commit a typed load number: move the row so it becomes the Nth enabled mod.
  // Invalid / out-of-range input reverts the field to the current value.
  function commitLoadNo(from: number, input: HTMLInputElement) {
    const target = Number(input.value.trim());
    if (
      !Number.isInteger(target) ||
      target < 1 ||
      target > enabledOrder.length
    ) {
      input.value = mods[from]?.loadNo ?? '';
      return;
    }
    const to = enabledOrder[target - 1];
    if (to === from) {
      input.value = mods[from].loadNo;
      return;
    }
    onmove(from, to);
  }

  // Edge auto-scroll: while a drag is near the top/bottom of the list, scroll
  // it so off-screen rows can be reached (rows are tall, few fit on screen).
  let listEl: HTMLDivElement;
  let scrollVel = 0;
  let rafId = 0;
  const EDGE = 56; // px from each edge that triggers scrolling
  const MAX_SPEED = 14; // px per frame at the very edge

  function scrollTick() {
    if (scrollVel !== 0 && listEl) listEl.scrollTop += scrollVel;
    rafId = requestAnimationFrame(scrollTick);
  }

  function startAutoScroll() {
    if (!rafId) rafId = requestAnimationFrame(scrollTick);
  }

  function stopAutoScroll() {
    if (rafId) cancelAnimationFrame(rafId);
    rafId = 0;
    scrollVel = 0;
  }

  function updateScrollVel(clientY: number) {
    const rect = listEl.getBoundingClientRect();
    if (clientY < rect.top + EDGE) {
      const f = (rect.top + EDGE - clientY) / EDGE;
      scrollVel = -Math.ceil(Math.min(1, f) * MAX_SPEED);
    } else if (clientY > rect.bottom - EDGE) {
      const f = (clientY - (rect.bottom - EDGE)) / EDGE;
      scrollVel = Math.ceil(Math.min(1, f) * MAX_SPEED);
    } else {
      scrollVel = 0;
    }
  }

  function handleDrop(to: number) {
    const from = dragIndex;
    dragIndex = null;
    overIndex = null;
    stopAutoScroll();
    if (from === null || from === to) return;
    onmove(from, to);
  }

  onDestroy(stopAutoScroll);
</script>

<div class="col">
  <div class="col-header">
    <span class="title">LOAD ORDER</span>
    <span class="sub">{enabledCount} active</span>
    <span class="spacer"></span>
    <span class="hint">resolves top → bottom · drag or type # to reorder</span>
  </div>
  <div
    class="list"
    bind:this={listEl}
    ondragover={(e) => {
      e.preventDefault();
      if (dragIndex !== null) updateScrollVel(e.clientY);
    }}
    role="list"
  >
    {#each mods as mod, i (mod.mod_id)}
      <div
        class="row"
        class:disabled={!mod.enabled}
        class:drop-target={overIndex === i && dragIndex !== null && dragIndex !== i}
        draggable={editIndex === i ? 'false' : 'true'}
        ondragstart={(e) => {
          dragIndex = i;
          // WebKitGTK requires drag data for `drop` to fire at all.
          e.dataTransfer?.setData('text/plain', String(i));
          if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
          startAutoScroll();
        }}
        ondragend={() => {
          dragIndex = null;
          overIndex = null;
          stopAutoScroll();
        }}
        ondragover={(e) => {
          e.preventDefault();
          if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
          overIndex = i;
        }}
        ondragleave={() => {
          if (overIndex === i) overIndex = null;
        }}
        ondrop={(e) => {
          e.preventDefault();
          handleDrop(i);
        }}
        role="listitem"
      >
        <span class="grip">⋮⋮</span>
        <input
          class="load-no"
          type="text"
          inputmode="numeric"
          value={mod.loadNo}
          disabled={!mod.enabled}
          title="Type a position to move this mod"
          aria-label="Load position for {mod.name}"
          onpointerdown={() => (editIndex = i)}
          onfocus={() => (editIndex = i)}
          onblur={(e) => {
            commitLoadNo(i, e.currentTarget);
            editIndex = null;
          }}
          onkeydown={(e) => {
            if (e.key === 'Enter') {
              e.currentTarget.blur();
            } else if (e.key === 'Escape') {
              e.currentTarget.value = mod.loadNo;
              e.currentTarget.blur();
            }
          }}
        />
        <div class="thumb" style="background:{mod.catBg};color:{mod.catColor}">{mod.initials}</div>
        <div class="meta">
          <div class="name-line">
            <span class="name">{mod.name}</span>
            <span class="cat-tag" style="color:{mod.catColor};background:{mod.catBg}">
              {mod.catLabel}
            </span>
          </div>
          <div class="info-line">
            {#if mod.version}<span class="mono">{mod.version}</span>{/if}
          </div>
          {#if mod.hasIssue}
            <div class="issue" style="color:{mod.statusColor}">⚠ {mod.issueText}</div>
          {/if}
        </div>
        <button
          class="badge"
          style="color:{mod.statusColor};background:{mod.statusBg}"
          onclick={() => ontoggle(mod.mod_id)}
          title={mod.enabled ? 'Disable in collection' : 'Enable in collection'}
        >
          {mod.statusText}
        </button>
        <button
          class="remove"
          onclick={() => onremove(mod.mod_id)}
          title="Remove from collection"
          aria-label="Remove from collection">✕</button
        >
      </div>
    {/each}
    {#if mods.length === 0}
      <div class="empty">This collection is empty. Add mods from the installed list.</div>
    {/if}
  </div>
</div>

<style>
  .col {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--panel);
  }

  .col-header {
    flex: none;
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 11px 16px 9px;
    border-bottom: 1px solid var(--bd2);
  }

  .title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.05em;
    color: var(--muted);
  }

  .sub {
    font-size: 11px;
    color: var(--faint);
  }

  .spacer {
    flex: 1;
  }

  .hint {
    font-size: 11px;
    color: var(--faint);
  }

  .list {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--bd2);
    cursor: grab;
    background: var(--surface);
  }

  .row:hover {
    background: var(--panel);
  }

  .row:hover .remove {
    opacity: 1;
  }

  .row.disabled {
    opacity: 0.55;
  }

  .row.drop-target {
    box-shadow: inset 0 2px 0 0 var(--acc);
  }

  .grip {
    color: var(--faint);
    font-size: 14px;
    line-height: 1;
    letter-spacing: 1px;
    flex: none;
  }

  .load-no {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--acc-ink);
    font-weight: 600;
    width: 24px;
    flex: none;
    /* Render as plain text until hovered/focused. */
    border: 1px solid transparent;
    border-radius: 4px;
    background: transparent;
    text-align: center;
    padding: 1px 0;
    cursor: text;
    -moz-appearance: textfield;
    appearance: textfield;
  }

  .load-no:hover:not(:disabled) {
    border-color: var(--bd);
  }

  .load-no:focus {
    outline: none;
    border-color: var(--acc);
    background: var(--surface);
  }

  .load-no:disabled {
    color: var(--faint);
    cursor: default;
  }

  .thumb {
    width: 30px;
    height: 30px;
    border-radius: 7px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 700;
    flex: none;
  }

  .meta {
    min-width: 0;
    flex: 1;
  }

  .name-line {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .name {
    font-size: 13px;
    font-weight: 600;
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .cat-tag {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 7px;
    border-radius: 5px;
    white-space: nowrap;
    flex: none;
  }

  .info-line {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-top: 2px;
    font-size: 11px;
    color: var(--faint);
    min-height: 13px;
  }

  .mono {
    font-family: var(--mono);
  }

  .issue {
    margin-top: 3px;
    font-size: 11px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .badge {
    font-family: inherit;
    font-size: 11px;
    font-weight: 600;
    padding: 2px 9px;
    border: none;
    border-radius: 999px;
    white-space: nowrap;
    cursor: pointer;
    flex: none;
  }

  .remove {
    flex: none;
    background: none;
    border: none;
    color: var(--faint);
    cursor: pointer;
    font-size: 12px;
    padding: 2px 4px;
    opacity: 0;
    transition: opacity 0.12s;
  }

  .remove:hover {
    color: #c8432f;
  }

  .empty {
    padding: 32px 16px;
    text-align: center;
    color: var(--faint);
    font-size: 12px;
    line-height: 1.5;
  }
</style>
