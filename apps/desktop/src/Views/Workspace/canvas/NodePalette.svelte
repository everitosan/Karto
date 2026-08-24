<script lang="ts">
  // Paleta de tipos de nodo, agrupada por categoría. Se arrastra un tipo al
  // canvas para crear un nodo. Cada categoría es un acordeón y una barra de
  // búsqueda filtra los tipos por etiqueta.
  import { Icon, icons } from "@karto/ui";
  import { NODE_CATALOG } from "$domain/infra";
  import { nodeGroups, nodeKindLabel, nodeSearchText } from "$i18n/catalog";
  import { m } from "$paraglide/messages.js";
  import { DND_MIME } from "./dnd";

  const groups = nodeGroups();

  let query = $state("");

  // Categorías colapsadas. El estado abierto/cerrado es chrome de navegación
  // atado a este equipo, así que se persiste en localStorage.
  const COLLAPSED_KEY = "karto.collapsedNodeCategories";

  function readCollapsed(): Set<string> {
    try {
      const raw = localStorage.getItem(COLLAPSED_KEY);
      if (raw) return new Set(JSON.parse(raw) as string[]);
    } catch {
      // localStorage no disponible o JSON inválido → todas abiertas.
    }
    return new Set();
  }

  let collapsed = $state<Set<string>>(readCollapsed());
  const isOpen = (category: string) => !collapsed.has(category);

  function toggleCategory(category: string) {
    const next = new Set(collapsed);
    if (next.has(category)) next.delete(category);
    else next.add(category);
    collapsed = next;
    try {
      localStorage.setItem(COLLAPSED_KEY, JSON.stringify([...next]));
    } catch {
      // localStorage no disponible: el estado vive solo en memoria esta sesión.
    }
  }

  // Al filtrar, se ignora el acordeón: se muestran todos los tipos que casan y
  // se ocultan las categorías sin coincidencias.
  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return groups.map((g) => ({ ...g, forceOpen: false }));
    return groups
      .map((g) => ({
        ...g,
        forceOpen: true,
        kinds: g.kinds.filter((k) => nodeSearchText(k).includes(q)),
      }))
      .filter((g) => g.kinds.length > 0);
  });

  function onDragStart(kind: string, e: DragEvent) {
    e.dataTransfer?.setData(DND_MIME, kind);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "copy";
  }
</script>

<div class="palette">
  <div class="title">{m.palette_title()}</div>
  <div class="search">
    <Icon icon={icons.search} size={14} />
    <input
      type="text"
      placeholder={m.palette_search()}
      bind:value={query}
      spellcheck="false"
    />
    {#if query}
      <button class="clear" title={m.common_clear()} onclick={() => (query = "")}>
        <Icon icon={icons.close} size={13} />
      </button>
    {/if}
  </div>

  {#each filtered as group (group.category)}
    {@const open = group.forceOpen || isOpen(group.category)}
    <button
      class="category"
      class:open
      aria-expanded={open}
      onclick={() => toggleCategory(group.category)}
    >
      <Icon icon={icons.chevron} size={12} />
      <span>{group.label}</span>
      <span class="count">{group.kinds.length}</span>
    </button>
    {#if open}
      {#each group.kinds as kind (kind)}
        <div
          class="item"
          role="button"
          tabindex="0"
          draggable="true"
          ondragstart={(e) => onDragStart(kind, e)}
        >
          <Icon icon={NODE_CATALOG[kind].icon} size={18} />
          <span>{nodeKindLabel(kind)}</span>
        </div>
      {/each}
    {/if}
  {:else}
    <p class="hint">{m.palette_no_matches({ query })}</p>
  {/each}

  {#if !query}
    <p class="hint">{m.palette_drag_hint()}</p>
  {/if}
</div>

<style>
  .palette {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    padding: 0.5rem;
    width: 11rem;
    border-right: 1px solid var(--karto-color-border);
    overflow-y: auto;
  }
  .title {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    opacity: 0.6;
    padding: 0.25rem;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.25rem 0.4rem;
    margin-bottom: 0.15rem;
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    background: var(--karto-color-bg);
    color: var(--karto-color-text-muted);
  }
  .search:focus-within {
    border-color: var(--karto-color-accent);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    color: var(--karto-color-text);
    font-family: var(--karto-font-body);
    font-size: 0.82rem;
    outline: none;
  }
  .search .clear {
    display: inline-flex;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    cursor: pointer;
    opacity: 0.7;
  }
  .search .clear:hover {
    opacity: 1;
  }
  .category {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    width: 100%;
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    opacity: 0.55;
    padding: 0.5rem 0.25rem 0.15rem;
    border: 0;
    background: transparent;
    color: var(--karto-color-text-muted);
    cursor: pointer;
    text-align: left;
  }
  .category:hover {
    opacity: 0.85;
  }
  .category > span:first-of-type {
    flex: 1;
  }
  .category :global(svg) {
    transition: transform 0.15s ease;
    flex-shrink: 0;
  }
  .category.open :global(svg) {
    transform: rotate(90deg);
  }
  .category .count {
    opacity: 0.6;
    font-variant-numeric: tabular-nums;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.5rem;
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    background: var(--karto-color-surface);
    font-size: 0.82rem;
    cursor: grab;
    user-select: none;
  }
  .item:hover {
    border-color: var(--karto-color-accent);
  }
  .item:active {
    cursor: grabbing;
  }
  .hint {
    margin-top: 0.5rem;
    font-size: 0.72rem;
    opacity: 0.45;
    line-height: 1.3;
    padding: 0 0.25rem;
  }
</style>
