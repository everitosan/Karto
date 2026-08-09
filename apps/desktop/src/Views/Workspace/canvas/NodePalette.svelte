<script lang="ts">
  // Paleta de tipos de nodo, agrupada por categoría. Se arrastra un tipo al
  // canvas para crear un nodo.
  import { Icon } from "@karto/ui";
  import { nodesByCategory, NODE_CATALOG, NODE_KIND_LABELS } from "$domain/infra";
  import { DND_MIME } from "./dnd";

  const groups = nodesByCategory();

  function onDragStart(kind: string, e: DragEvent) {
    e.dataTransfer?.setData(DND_MIME, kind);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "copy";
  }
</script>

<div class="palette">
  <div class="title">Nodos</div>
  {#each groups as group (group.category)}
    <div class="category">{group.label}</div>
    {#each group.kinds as kind (kind)}
      <div
        class="item"
        role="button"
        tabindex="0"
        draggable="true"
        ondragstart={(e) => onDragStart(kind, e)}
      >
        <Icon icon={NODE_CATALOG[kind].icon} size={18} />
        <span>{NODE_KIND_LABELS[kind]}</span>
      </div>
    {/each}
  {/each}
  <p class="hint">Arrastra un tipo al lienzo.</p>
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
  .category {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    opacity: 0.45;
    padding: 0.5rem 0.25rem 0.15rem;
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
