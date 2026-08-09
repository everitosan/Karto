<script lang="ts">
  // Árbol de carpetas y diagramas (Fase 2). Componente local a la vista
  // Workspace. Soporta crear, renombrar, anidar y mover con drag & drop.
  import { onMount } from "svelte";
  import { Icon, icons } from "@karto/ui";
  import type { Folder, InfraMap } from "$domain/infra";
  import { workspaceUseCases as uc } from "$usecases/workspace";

  interface Props {
    selectedMapId: string | null;
  }

  let { selectedMapId = $bindable(null) }: Props = $props();

  let folders = $state<Folder[]>([]);
  let maps = $state<InfraMap[]>([]);
  let loaded = $state(false);

  // Edición inline (renombrar) y arrastre.
  let editing = $state<{ id: string; kind: "folder" | "map" } | null>(null);
  let editValue = $state("");
  let dragItem = $state<{ id: string; kind: "folder" | "map" } | null>(null);
  // Pista de destino del arrastre: insertar antes/después de una fila,
  // anidar dentro de una carpeta ("inside"), o soltar en la raíz.
  type Zone = "before" | "after" | "inside";
  let dropHint = $state<{ id: string; zone: Zone } | "root" | null>(null);

  // Acordeón: carpetas colapsadas (vacío = todas abiertas por defecto).
  let collapsed = $state<Set<string>>(new Set());
  const isOpen = (id: string) => !collapsed.has(id);
  const hasChildren = (id: string) =>
    childFolders(id).length > 0 || folderMaps(id).length > 0;

  function toggleFolder(id: string) {
    const next = new Set(collapsed);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    collapsed = next;
  }

  function openFolder(id: string) {
    if (collapsed.has(id)) {
      const next = new Set(collapsed);
      next.delete(id);
      collapsed = next;
    }
  }

  onMount(reload);

  async function reload() {
    [folders, maps] = await Promise.all([uc.listFolders(), uc.listMaps()]);
    loaded = true;
  }

  const childFolders = (parentId: string | null) =>
    folders
      .filter((f) => f.parentId === parentId)
      .sort((a, b) => a.position - b.position);

  const folderMaps = (folderId: string | null) =>
    maps
      .filter((m) => m.folderId === folderId)
      .sort((a, b) => a.position - b.position);

  /** Ids de todas las carpetas descendientes (para evitar ciclos al mover). */
  function descendantIds(folderId: string): Set<string> {
    const out = new Set<string>();
    const walk = (id: string) => {
      for (const child of folders.filter((f) => f.parentId === id)) {
        out.add(child.id);
        walk(child.id);
      }
    };
    walk(folderId);
    return out;
  }

  function focusOnMount(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  function startEdit(id: string, kind: "folder" | "map", current: string) {
    editing = { id, kind };
    editValue = current;
  }

  async function commitEdit() {
    if (!editing) return;
    const name = editValue.trim();
    const { id, kind } = editing;
    editing = null;
    if (!name) return;
    if (kind === "folder") await uc.renameFolder(id, name);
    else await uc.renameMap(id, name);
    await reload();
  }

  async function addFolder(parentId: string | null) {
    if (parentId) openFolder(parentId); // que el hijo nuevo sea visible
    const f = await uc.createFolder("Nueva carpeta", parentId);
    await reload();
    startEdit(f.id, "folder", f.name);
  }

  async function addMap(folderId: string | null) {
    if (folderId) openFolder(folderId);
    const m = await uc.createMap("Nuevo diagrama", folderId);
    await reload();
    selectedMapId = m.id;
    startEdit(m.id, "map", m.name);
  }

  async function removeFolder(f: Folder) {
    if (!confirm(`¿Eliminar la carpeta "${f.name}" y todo su contenido?`)) return;
    await uc.deleteFolder(f.id);
    await reload();
  }

  async function removeMap(m: InfraMap) {
    if (!confirm(`¿Eliminar el diagrama "${m.name}"?`)) return;
    await uc.deleteMap(m.id);
    if (selectedMapId === m.id) selectedMapId = null;
    await reload();
  }

  // --- Drag & drop ---
  function onDragStart(id: string, kind: "folder" | "map", e: DragEvent) {
    dragItem = { id, kind };
    e.dataTransfer?.setData("text/plain", id);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
  }

  /** ¿Puede la carpeta arrastrada pasar a tener este padre? (evita ciclos). */
  function canReparent(parentId: string | null): boolean {
    if (!dragItem) return false;
    if (dragItem.kind === "folder") {
      if (dragItem.id === parentId) return false;
      if (parentId && descendantIds(dragItem.id).has(parentId)) return false;
    }
    return true;
  }

  async function moveInto(parentId: string | null) {
    if (!dragItem) return;
    if (dragItem.kind === "folder") {
      await uc.moveFolder(dragItem.id, parentId, childFolders(parentId).length);
    } else {
      await uc.moveMap(dragItem.id, parentId, folderMaps(parentId).length);
    }
  }

  /** Renumera las posiciones de un grupo de hermanos según `orderedIds`. */
  async function persistOrder(
    parentId: string | null,
    kind: "folder" | "map",
    orderedIds: string[],
  ) {
    for (let i = 0; i < orderedIds.length; i++) {
      if (kind === "folder") await uc.moveFolder(orderedIds[i], parentId, i);
      else await uc.moveMap(orderedIds[i], parentId, i);
    }
  }

  /** Reordena el elemento arrastrado antes/después de `target` entre hermanos. */
  async function reorderAround(
    parentId: string | null,
    target: Folder | InfraMap,
    zone: "before" | "after",
  ) {
    const item = dragItem!;
    if (item.kind === "folder" && !canReparent(parentId)) return;
    const siblings =
      item.kind === "folder" ? childFolders(parentId) : folderMaps(parentId);
    const ids = siblings.map((s) => s.id).filter((id) => id !== item.id);
    let idx = ids.indexOf(target.id); // -1 si target es de otro tipo
    if (idx === -1) idx = ids.length;
    else if (zone === "after") idx += 1;
    ids.splice(idx, 0, item.id);
    await persistOrder(parentId, item.kind, ids);
  }

  // --- Handlers de fila ---
  /** Zona de la fila según la posición vertical del cursor. */
  function computeZone(
    item: Folder | InfraMap,
    isFolder: boolean,
    e: DragEvent,
  ): Zone {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const y = e.clientY - rect.top;
    const h = rect.height || 1;
    if (isFolder) {
      // Tercio superior/inferior = reordenar; centro = anidar dentro.
      if (y < h * 0.28) return "before";
      if (y > h * 0.72) return "after";
      return canReparent(item.id) ? "inside" : y < h / 2 ? "before" : "after";
    }
    return y < h / 2 ? "before" : "after";
  }

  function rowDragOver(item: Folder | InfraMap, isFolder: boolean, e: DragEvent) {
    if (!dragItem || dragItem.id === item.id) return;
    e.preventDefault();
    e.stopPropagation();
    const zone = computeZone(item, isFolder, e);
    // Spring-load: abre una carpeta colapsada al posar algo dentro de ella.
    if (isFolder && zone === "inside") openFolder(item.id);
    dropHint = { id: item.id, zone };
  }

  // Solo limpia la pista si el cursor sale de la fila (no al entrar a un hijo).
  function rowDragLeave(id: string, e: DragEvent) {
    const row = e.currentTarget as HTMLElement;
    if (e.relatedTarget instanceof Node && row.contains(e.relatedTarget)) return;
    if (dropHint !== "root" && dropHint?.id === id) dropHint = null;
  }

  // La zona se recalcula aquí (no se confía en `dropHint`, que el `dragleave`
  // sobre los hijos de la fila puede haber limpiado justo antes del `drop`).
  async function rowDrop(item: Folder | InfraMap, isFolder: boolean, e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    dropHint = null;
    const dragged = dragItem;
    if (!dragged || dragged.id === item.id) {
      dragItem = null;
      return;
    }
    const zone = computeZone(item, isFolder, e); // usa dragItem → aún asignado
    try {
      if (isFolder && zone === "inside") {
        if (canReparent(item.id)) await moveInto(item.id);
      } else {
        const parentId = isFolder
          ? (item as Folder).parentId
          : (item as InfraMap).folderId;
        // Evita anidar una carpeta dentro de su propio subárbol al reordenar.
        const cycle =
          dragged.kind === "folder" &&
          (parentId === dragged.id ||
            (parentId != null && descendantIds(dragged.id).has(parentId)));
        if (!cycle) {
          await reorderAround(parentId, item, zone === "inside" ? "after" : zone);
        }
      }
    } finally {
      dragItem = null;
      await reload();
    }
  }

  // --- Zona raíz (soltar en el área vacía = mover a la raíz, al final) ---
  function rootDragOver(e: DragEvent) {
    if (!canReparent(null)) return;
    e.preventDefault();
    dropHint = "root";
  }

  async function rootDrop() {
    const item = dragItem;
    dropHint = null;
    dragItem = null;
    if (!item || !canReparent(null)) return;
    await moveInto(null);
    await reload();
  }
</script>

<aside class="sidebar">
  <div class="head">
    <span>Diagramas</span>
    <div class="head-actions">
      <button title="Nueva carpeta" onclick={() => addFolder(null)}>
        <Icon icon={icons.folder} size={15} />
        <Icon icon={icons.add} size={11} />
      </button>
      <button title="Nuevo diagrama" onclick={() => addMap(null)}>
        <Icon icon={icons.diagram} size={15} />
        <Icon icon={icons.add} size={11} />
      </button>
    </div>
  </div>

  <div
    class="tree"
    class:drop-root={dropHint === "root"}
    role="tree"
    tabindex="-1"
    ondragover={rootDragOver}
    ondragleave={() => (dropHint === "root" ? (dropHint = null) : null)}
    ondrop={rootDrop}
  >
    {#if loaded && folders.length === 0 && maps.length === 0}
      <p class="empty">
        Sin diagramas todavía. Crea una carpeta o un diagrama con los botones de
        arriba.
      </p>
    {/if}

    {#each childFolders(null) as folder (folder.id)}
      {@render folderRow(folder, 0)}
    {/each}
    {#each folderMaps(null) as map (map.id)}
      {@render mapRow(map, 0)}
    {/each}
  </div>
</aside>

{#snippet folderRow(folder: Folder, depth: number)}
  <div
    class="row folder"
    class:hint-inside={dropHint !== "root" && dropHint?.id === folder.id && dropHint.zone === "inside"}
    class:hint-before={dropHint !== "root" && dropHint?.id === folder.id && dropHint.zone === "before"}
    class:hint-after={dropHint !== "root" && dropHint?.id === folder.id && dropHint.zone === "after"}
    style="--depth: {depth}"
    role="treeitem"
    aria-selected="false"
    tabindex="-1"
    draggable={editing?.id !== folder.id}
    ondragstart={(e) => onDragStart(folder.id, "folder", e)}
    ondragover={(e) => rowDragOver(folder, true, e)}
    ondragleave={(e) => rowDragLeave(folder.id, e)}
    ondrop={(e) => rowDrop(folder, true, e)}
  >
    {#if hasChildren(folder.id)}
      <button
        class="chevron"
        class:open={isOpen(folder.id)}
        title={isOpen(folder.id) ? "Colapsar" : "Expandir"}
        aria-expanded={isOpen(folder.id)}
        onclick={() => toggleFolder(folder.id)}
      >
        <Icon icon={icons.chevron} size={14} />
      </button>
    {:else}
      <span class="chevron-spacer"></span>
    {/if}
    <Icon icon={icons.folder} size={16} />
    {#if editing?.id === folder.id}
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="edit"
        bind:value={editValue}
        use:focusOnMount
        onblur={commitEdit}
        onkeydown={(e) => {
          if (e.key === "Enter") commitEdit();
          if (e.key === "Escape") (editing = null);
        }}
      />
    {:else}
      <button
        class="label"
        onclick={() => toggleFolder(folder.id)}
        ondblclick={() => startEdit(folder.id, "folder", folder.name)}
      >
        {folder.name}
      </button>
      <div class="actions">
        <button title="Subcarpeta" onclick={() => addFolder(folder.id)}>
          <Icon icon={icons.folder} size={13} />
        </button>
        <button title="Nuevo diagrama aquí" onclick={() => addMap(folder.id)}>
          <Icon icon={icons.add} size={13} />
        </button>
        <button title="Renombrar" onclick={() => startEdit(folder.id, "folder", folder.name)}>
          <Icon icon={icons.edit} size={13} />
        </button>
        <button title="Eliminar" onclick={() => removeFolder(folder)}>
          <Icon icon={icons.delete} size={13} />
        </button>
      </div>
    {/if}
  </div>

  {#if isOpen(folder.id)}
    {#each childFolders(folder.id) as child (child.id)}
      {@render folderRow(child, depth + 1)}
    {/each}
    {#each folderMaps(folder.id) as map (map.id)}
      {@render mapRow(map, depth + 1)}
    {/each}
  {/if}
{/snippet}

{#snippet mapRow(map: InfraMap, depth: number)}
  <div
    class="row map"
    class:active={selectedMapId === map.id}
    class:hint-before={dropHint !== "root" && dropHint?.id === map.id && dropHint.zone === "before"}
    class:hint-after={dropHint !== "root" && dropHint?.id === map.id && dropHint.zone === "after"}
    style="--depth: {depth}"
    role="treeitem"
    aria-selected={selectedMapId === map.id}
    tabindex="-1"
    draggable={editing?.id !== map.id}
    ondragstart={(e) => onDragStart(map.id, "map", e)}
    ondragover={(e) => rowDragOver(map, false, e)}
    ondragleave={(e) => rowDragLeave(map.id, e)}
    ondrop={(e) => rowDrop(map, false, e)}
  >
    <Icon icon={icons.diagram} size={16} />
    {#if editing?.id === map.id}
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="edit"
        bind:value={editValue}
        use:focusOnMount
        onblur={commitEdit}
        onkeydown={(e) => {
          if (e.key === "Enter") commitEdit();
          if (e.key === "Escape") (editing = null);
        }}
      />
    {:else}
      <button class="label" onclick={() => (selectedMapId = map.id)} ondblclick={() => startEdit(map.id, "map", map.name)}>
        {map.name}
      </button>
      <div class="actions">
        <button title="Renombrar" onclick={() => startEdit(map.id, "map", map.name)}>
          <Icon icon={icons.edit} size={13} />
        </button>
        <button title="Eliminar" onclick={() => removeMap(map)}>
          <Icon icon={icons.delete} size={13} />
        </button>
      </div>
    {/if}
  </div>
{/snippet}

<style>
  .sidebar {
    height: 100%;
    width: 100%;
    /* Fondo opaco: al colapsar, el panel se muestra como overlay sobre el
       canvas y no debe transparentar lo que hay detrás. */
    background: var(--karto-color-bg);
    border-right: 1px solid var(--karto-color-border);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    opacity: 0.7;
    padding: 0.75rem 0.75rem 0.5rem;
  }
  .head-actions {
    display: flex;
    gap: 0.25rem;
  }
  .head-actions button {
    display: inline-flex;
    align-items: center;
    gap: 1px;
    padding: 0.2rem 0.3rem;
    border: 0;
    border-radius: var(--karto-radius);
    background: transparent;
    color: inherit;
    cursor: pointer;
    opacity: 0.8;
  }
  .head-actions button:hover {
    background: var(--karto-color-surface);
    opacity: 1;
  }
  .tree {
    flex: 1;
    padding: 0 0.25rem 0.5rem;
    min-height: 3rem;
  }
  .tree.drop-root {
    outline: 1px dashed var(--karto-color-accent);
    outline-offset: -3px;
    border-radius: var(--karto-radius);
  }
  .empty {
    font-size: 0.8rem;
    opacity: 0.5;
    padding: 0.5rem 0.6rem;
    line-height: 1.4;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.5rem;
    padding-left: calc(0.5rem + var(--depth) * 0.9rem);
    border-radius: var(--karto-radius);
    cursor: pointer;
    position: relative;
  }
  .row:hover {
    background: var(--karto-color-surface);
  }
  .row.hint-inside {
    outline: 1px dashed var(--karto-color-accent);
    outline-offset: -2px;
    background: color-mix(in srgb, var(--karto-color-accent) 12%, transparent);
  }
  /* Línea de inserción para reordenar entre hermanos. */
  .row.hint-before::before,
  .row.hint-after::after {
    content: "";
    position: absolute;
    left: calc(0.5rem + var(--depth) * 0.9rem);
    right: 0.5rem;
    height: 2px;
    background: var(--karto-color-accent);
    border-radius: 2px;
  }
  .row.hint-before::before {
    top: -1px;
  }
  .row.hint-after::after {
    bottom: -1px;
  }
  .row.folder .label {
    font-weight: 600;
    color: var(--karto-color-text-muted);
  }
  .chevron {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.1rem;
    height: 1.1rem;
    flex-shrink: 0;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--karto-color-text-muted);
    cursor: pointer;
    border-radius: 4px;
    transition: transform 0.15s ease;
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .chevron:hover {
    color: var(--karto-color-text);
    background: var(--karto-color-bg);
  }
  .chevron-spacer {
    display: inline-block;
    width: 1.1rem;
    flex-shrink: 0;
  }
  .row.map.active {
    background: var(--karto-color-accent);
    color: #06130a;
  }
  .label {
    flex: 1;
    text-align: left;
    border: 0;
    background: transparent;
    color: inherit;
    font-family: var(--karto-font-body);
    font-size: 0.9rem;
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .edit {
    flex: 1;
    background: var(--karto-color-bg);
    color: var(--karto-color-text);
    border: 1px solid var(--karto-color-accent);
    border-radius: var(--karto-radius);
    padding: 0.1rem 0.35rem;
    font-family: var(--karto-font-body);
    font-size: 0.9rem;
  }
  .actions {
    display: none;
    gap: 0.1rem;
  }
  .row:hover .actions {
    display: flex;
  }
  .actions button {
    display: inline-flex;
    padding: 0.2rem;
    border: 0;
    border-radius: var(--karto-radius);
    background: transparent;
    color: inherit;
    cursor: pointer;
    opacity: 0.7;
  }
  .actions button:hover {
    background: var(--karto-color-bg);
    opacity: 1;
  }
</style>
