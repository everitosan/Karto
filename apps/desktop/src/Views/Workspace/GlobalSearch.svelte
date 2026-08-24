<script lang="ts">
  // Búsqueda global (Fase 6): encuentra nodos por etiqueta, hostname, dirección
  // (IP) u otras propiedades en TODOS los diagramas. Al elegir un resultado
  // avisa al padre (que cambia de diagrama y pide enfocar el nodo). Componente
  // local a la vista Workspace.
  import { Icon, icons } from "@karto/ui";
  import type { SearchHit } from "$domain/infra";
  import { NODE_KIND_LABELS } from "$domain/infra";
  import { workspaceUseCases as uc } from "$usecases/workspace";

  interface Props {
    onPick: (hit: SearchHit) => void;
  }

  let { onPick }: Props = $props();

  let query = $state("");
  let results = $state<SearchHit[]>([]);
  let open = $state(false);
  let inputEl = $state<HTMLInputElement | null>(null);

  let debounce: ReturnType<typeof setTimeout> | undefined;
  // Secuencia para descartar respuestas fuera de orden (la última gana).
  let seq = 0;

  function onInput() {
    clearTimeout(debounce);
    const q = query;
    if (q.trim() === "") {
      results = [];
      open = false;
      return;
    }
    debounce = setTimeout(async () => {
      const mine = ++seq;
      const hits = await uc.searchNodes(q);
      if (mine === seq) {
        results = hits;
        open = true;
      }
    }, 200);
  }

  function pick(hit: SearchHit) {
    onPick(hit);
    open = false;
    query = "";
    results = [];
  }

  function clear() {
    query = "";
    results = [];
    open = false;
    inputEl?.focus();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (query) clear();
      else inputEl?.blur();
    } else if (e.key === "Enter" && results.length > 0) {
      pick(results[0]);
    }
  }
</script>

<div class="search">
  <div class="field">
    <Icon icon={icons.search} size={15} />
    <input
      bind:this={inputEl}
      bind:value={query}
      oninput={onInput}
      onkeydown={onKeydown}
      onfocus={() => (open = results.length > 0)}
      placeholder="Buscar por IP, host o etiqueta…"
      spellcheck="false"
      autocomplete="off"
    />
    {#if query}
      <button class="clear" onclick={clear} title="Limpiar" aria-label="Limpiar búsqueda">
        <Icon icon={icons.close} size={14} />
      </button>
    {/if}
  </div>

  {#if open}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="backdrop" onclick={() => (open = false)}></div>
    <ul class="results" role="listbox">
      {#if results.length === 0}
        <li class="empty">Sin resultados</li>
      {:else}
        {#each results as hit (hit.nodeId)}
          <li>
            <button class="hit" onclick={() => pick(hit)} role="option" aria-selected="false">
              <span class="label">{hit.label || NODE_KIND_LABELS[hit.kind] || hit.kind}</span>
              <span class="meta">
                <span class="matched">{hit.matched}</span>
                <span class="map">{hit.mapName}</span>
              </span>
            </button>
          </li>
        {/each}
      {/if}
    </ul>
  {/if}
</div>

<style>
  .search {
    position: relative;
    padding: 0.5rem;
  }
  .field {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.35rem 0.5rem;
    background: var(--karto-color-surface);
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    color: var(--karto-color-text);
  }
  .field :global(svg) {
    opacity: 0.6;
    flex: none;
  }
  input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font-family: var(--karto-font-body);
    font-size: 0.85rem;
    outline: none;
  }
  input::placeholder {
    color: var(--karto-color-text);
    opacity: 0.45;
  }
  .clear {
    display: flex;
    align-items: center;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--karto-color-text);
    opacity: 0.6;
    cursor: pointer;
  }
  .clear:hover {
    opacity: 1;
  }
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 30;
  }
  .results {
    position: absolute;
    z-index: 31;
    left: 0.5rem;
    right: 0.5rem;
    margin: 0.25rem 0 0;
    padding: 0.25rem;
    list-style: none;
    max-height: 18rem;
    overflow-y: auto;
    background: var(--karto-color-bg);
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    box-shadow: 0 8px 28px rgba(0, 0, 0, 0.45);
  }
  .hit {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    width: 100%;
    padding: 0.4rem 0.5rem;
    border: 0;
    border-radius: var(--karto-radius);
    background: transparent;
    color: var(--karto-color-text);
    text-align: left;
    cursor: pointer;
  }
  .hit:hover {
    background: var(--karto-color-surface);
  }
  .label {
    font-size: 0.85rem;
    font-family: var(--karto-font-body);
  }
  .meta {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
    font-size: 0.72rem;
    opacity: 0.6;
  }
  .matched {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .map {
    flex: none;
    color: var(--karto-color-accent);
    opacity: 0.85;
  }
  .empty {
    padding: 0.5rem;
    font-size: 0.8rem;
    opacity: 0.5;
  }
</style>
