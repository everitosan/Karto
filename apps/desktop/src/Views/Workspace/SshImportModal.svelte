<script lang="ts">
  // Importación desde configuración SSH (Fase 4). Dos etapas:
  //  1) Elegir origen: sugerencias halladas bajo ~/.ssh (config y config.d/**),
  //     un archivo soltado sobre la ventana, o uno elegido con el diálogo.
  //  2) Vista previa: elegir qué hosts crear y el diagrama destino (existente o
  //     nuevo), con los duplicados marcados para no repetirlos.
  // Local a la vista Workspace.
  import { Modal, Button, Icon, Checkbox, icons } from "@karto/ui";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import type { AccessContext, CandidateFile, ImportedHost, InfraMap } from "$domain/infra";
  import { workspaceUseCases as uc } from "$usecases/workspace";
  import { hostTarget, isDuplicate } from "./sshImport";
  import { m } from "$paraglide/messages.js";

  interface Props {
    open: boolean;
    maps: InfraMap[];
    selectedMapId: string | null;
    onClose: () => void;
    /** Se invoca tras importar, con el diagrama destino para seleccionarlo. */
    onImported: (mapId: string) => void;
  }

  let { open, maps, selectedMapId, onClose, onImported }: Props = $props();

  // --- Etapa 1: origen ---
  let candidates = $state<CandidateFile[]>([]);
  let loadingCandidates = $state(false);
  let dragActive = $state(false);
  // Origen elegido: null = seguimos en el selector de archivos.
  let sourcePath = $state<string | null>(null);
  let sourceName = $state("");

  // --- Etapa 2: vista previa ---
  let loadingHosts = $state(false);
  let error = $state<string | null>(null);
  let hosts = $state<ImportedHost[]>([]);
  let selected = $state<Set<string>>(new Set());
  let duplicates = $state<Set<string>>(new Set());
  let targetMode = $state<"existing" | "new">("existing");
  let targetMapId = $state<string | null>(null);
  let newMapName = $state(m.ssh_hosts_map_default());
  let importing = $state(false);

  // Contexto de acceso destino: la IP importada se guarda como dirección de este
  // contexto. Si solo hay uno ("Principal"), se usa sin preguntar; si hay varios,
  // se muestra un selector.
  let contexts = $state<AccessContext[]>([]);
  let targetContextId = $state<string | null>(null);

  const basename = (p: string) => p.split(/[\\/]/).pop() ?? p;

  // Al abrir: resetea y carga las sugerencias.
  $effect(() => {
    if (open) void init();
  });

  // Escucha de arrastrar/soltar de la ventana mientras el modal está abierto.
  // Tauri captura el file-drop del SO y lo entrega aquí (no vía HTML5), así que
  // recibimos rutas reales que el backend puede parsear.
  $effect(() => {
    if (!open) return;
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        const p = event.payload;
        if (p.type === "enter" || p.type === "over") dragActive = true;
        else if (p.type === "leave") dragActive = false;
        else if (p.type === "drop") {
          dragActive = false;
          const path = p.paths?.[0];
          if (path) void loadSource(path);
        }
      })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  });

  async function init() {
    sourcePath = null;
    error = null;
    hosts = [];
    dragActive = false;
    importing = false;
    loadingCandidates = true;
    try {
      contexts = await uc.listContexts();
      targetContextId = contexts[0]?.id ?? null;
      candidates = await uc.sshImportCandidates();
    } catch (e) {
      error = String(e);
    } finally {
      loadingCandidates = false;
    }
  }

  // Carga los hosts de un archivo concreto y pasa a la vista previa.
  async function loadSource(path: string) {
    sourcePath = path;
    sourceName = basename(path);
    error = null;
    hosts = [];
    loadingHosts = true;
    try {
      hosts = await uc.sshImportParseFile(path);
      targetMode = maps.length > 0 ? "existing" : "new";
      targetMapId = selectedMapId ?? maps[0]?.id ?? null;
      newMapName = m.ssh_hosts_map_named({ name: sourceName });
      await refreshDuplicates();
    } catch (e) {
      error = String(e);
    } finally {
      loadingHosts = false;
    }
  }

  async function browse() {
    const picked = await openDialog({
      multiple: false,
      directory: false,
      title: m.ssh_browse_title(),
    });
    if (typeof picked === "string") await loadSource(picked);
  }

  function back() {
    sourcePath = null;
    error = null;
  }

  // Recalcula duplicados contra el diagrama destino y marca por defecto lo
  // no-duplicado.
  async function refreshDuplicates() {
    const dupes = new Set<string>();
    if (targetMode === "existing" && targetMapId) {
      const graph = await uc.loadGraph(targetMapId);
      for (const h of hosts) {
        if (isDuplicate(h, graph.nodes)) dupes.add(h.alias);
      }
    }
    duplicates = dupes;
    selected = new Set(hosts.filter((h) => !dupes.has(h.alias)).map((h) => h.alias));
  }

  async function onTargetChange() {
    await refreshDuplicates();
  }

  function toggle(alias: string) {
    const next = new Set(selected);
    if (next.has(alias)) next.delete(alias);
    else next.add(alias);
    selected = next;
  }

  const selectedCount = $derived(selected.size);
  const canImport = $derived(
    selectedCount > 0 &&
      !importing &&
      targetContextId !== null &&
      (targetMode === "new" ? newMapName.trim().length > 0 : targetMapId !== null),
  );

  async function doImport() {
    importing = true;
    error = null;
    try {
      let mapId = targetMapId;
      if (targetMode === "new") {
        const map = await uc.createMap(newMapName.trim(), null);
        mapId = map.id;
      }
      if (!mapId || !targetContextId) return;
      const chosen = hosts.filter((h) => selected.has(h.alias));
      await uc.sshImportHosts(mapId, targetContextId, chosen);
      onImported(mapId);
    } catch (e) {
      error = String(e);
      importing = false;
    }
  }
</script>

<Modal {open} title={m.ssh_import_title()} width="42rem" {onClose}>
  <div class="body">
    {#if error}
      <p class="error">{error}</p>
    {/if}

    {#if sourcePath === null}
      <!-- Etapa 1: elegir origen -->
      <div
        class="dropzone"
        class:active={dragActive}
        role="region"
        aria-label={m.ssh_drop_aria()}
      >
        <Icon icon={icons.terminal} size={22} />
        <p>{m.ssh_drop_text()}</p>
        <Button variant="secondary" onclick={browse}>{m.ssh_choose_file()}</Button>
      </div>

      <div class="suggestions">
        <span class="head">{m.ssh_suggestions()}</span>
        {#if loadingCandidates}
          <p class="muted">{m.ssh_searching()}</p>
        {:else if candidates.length === 0}
          <p class="muted">{m.ssh_no_files()}</p>
        {:else}
          <ul class="files">
            {#each candidates as c (c.path)}
              <li>
                <button class="file" onclick={() => loadSource(c.path)}>
                  <Icon icon={icons.terminal} size={14} />
                  <div class="file-main">
                    <span class="fname">{c.name}</span>
                    <span class="fpath">{c.path}</span>
                  </div>
                  <span class="chip">{c.hostCount} {c.hostCount === 1 ? m.ssh_host() : m.ssh_hosts()}</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {:else}
      <!-- Etapa 2: vista previa del archivo elegido -->
      <button class="back" onclick={back}>← {m.ssh_choose_another()}</button>
      <div class="source-line">
        <Icon icon={icons.terminal} size={14} />
        <span class="fname">{sourceName}</span>
        <span class="fpath">{sourcePath}</span>
      </div>

      {#if loadingHosts}
        <p class="muted">{m.ssh_reading({ name: sourceName })}</p>
      {:else if hosts.length === 0}
        <p class="muted">{m.ssh_no_hosts()}</p>
      {:else}
        <label class="field">
          <span>{m.ssh_target_diagram()}</span>
          <div class="target-row">
            <select bind:value={targetMode} onchange={onTargetChange}>
              <option value="existing" disabled={maps.length === 0}>{m.ssh_existing()}</option>
              <option value="new">{m.sidebar_new_map()}</option>
            </select>
            {#if targetMode === "existing"}
              <select bind:value={targetMapId} onchange={onTargetChange}>
                {#each maps as mp (mp.id)}<option value={mp.id}>{mp.name}</option>{/each}
              </select>
            {:else}
              <input placeholder={m.ssh_diagram_name_placeholder()} bind:value={newMapName} />
            {/if}
          </div>
        </label>

        {#if contexts.length > 1}
          <label class="field">
            <span>{m.ssh_target_context()}</span>
            <select bind:value={targetContextId}>
              {#each contexts as ctx (ctx.id)}<option value={ctx.id}>{ctx.name}</option>{/each}
            </select>
          </label>
        {/if}

        <ul class="hosts">
          {#each hosts as host (host.alias)}
            {@const dup = duplicates.has(host.alias)}
            <li class:dup>
              <Checkbox
                checked={selected.has(host.alias)}
                onchange={() => toggle(host.alias)}
              >
                <span class="host-row">
                  <span class="alias">{host.alias}</span>
                  <span class="target-host">{hostTarget(host)}</span>
                  {#if host.user}<span class="chip">{host.user}</span>{/if}
                  {#if host.port}<span class="chip">:{host.port}</span>{/if}
                  {#if host.identityFile}
                    <span class="chip key" title={host.identityFile}>
                      <Icon icon={icons.key} size={11} />
                    </span>
                  {/if}
                  {#if dup}<span class="chip dup-chip">{m.ssh_already_exists()}</span>{/if}
                </span>
              </Checkbox>
            </li>
          {/each}
        </ul>
        <p class="muted count">{m.ssh_selected_count({ count: selectedCount, total: hosts.length })}</p>
      {/if}
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>{m.common_cancel()}</Button>
    {#if sourcePath !== null && hosts.length > 0}
      <Button variant="primary" disabled={!canImport} onclick={doImport}>
        {importing ? m.ssh_importing() : `${m.ssh_import_action()} ${selectedCount || ""}`.trim()}
      </Button>
    {/if}
  {/snippet}
</Modal>

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
    width: 100%;
    /* Nada debe forzar scroll horizontal dentro del modal. */
    overflow-x: hidden;
  }
  .muted {
    opacity: 0.6;
    font-size: 0.85rem;
  }
  .error {
    color: #ff6b6b;
    font-size: 0.85rem;
  }

  /* --- Drop zone --- */
  .dropzone {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    padding: 1.5rem 1rem;
    border: 1px dashed var(--karto-color-border);
    border-radius: var(--karto-radius);
    text-align: center;
    color: var(--karto-color-text-muted);
    transition: border-color 0.15s ease, background 0.15s ease;
  }
  .dropzone.active {
    border-color: var(--karto-color-accent);
    background: color-mix(in srgb, var(--karto-color-accent) 10%, transparent);
    color: var(--karto-color-text);
  }
  .dropzone p {
    margin: 0;
    font-size: 0.85rem;
  }

  /* --- Sugerencias --- */
  .suggestions {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .suggestions .head {
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    opacity: 0.6;
  }
  .files {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    max-height: 16rem;
    overflow-y: auto;
  }
  .file {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.4rem 0.5rem;
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    background: var(--karto-color-surface);
    color: inherit;
    cursor: pointer;
    text-align: left;
    font-size: 0.85rem;
  }
  .file:hover {
    border-color: var(--karto-color-accent);
  }
  .file :global(svg) {
    flex-shrink: 0;
  }
  .file-main {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .file-main .fname {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .file-main .fpath {
    margin-top: 0.2rem;
  }
  .file .chip {
    flex-shrink: 0;
  }

  /* --- Vista previa --- */
  .back {
    align-self: flex-start;
    background: transparent;
    border: 0;
    color: var(--karto-color-accent);
    cursor: pointer;
    font-size: 0.8rem;
    padding: 0;
  }
  .source-line {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.8rem;
  }
  .fname {
    font-weight: 600;
  }
  .fpath {
    flex: 1;
    min-width: 0;
    opacity: 0.55;
    font-family: var(--karto-font-mono, monospace);
    font-size: 0.72rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--karto-space-2);
    font-size: 0.78rem;
    opacity: 0.9;
  }
  .target-row {
    display: flex;
    gap: 0.5rem;
  }
  .target-row > * {
    flex: 1;
    min-width: 0;
  }
  select,
  input {
    background: var(--karto-color-surface);
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    color: var(--karto-color-text);
    padding: 0.45rem 0.6rem;
    font-family: var(--karto-font-body);
    font-size: 0.9rem;
    width: 100%;
  }
  select {
    appearance: none;
    -webkit-appearance: none;
    color-scheme: dark;
    cursor: pointer;
  }
  select:focus,
  input:focus {
    outline: none;
    border-color: var(--karto-color-accent);
  }
  .hosts {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
    max-height: 16rem;
    overflow-y: auto;
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
  }
  .hosts li {
    padding: 0.3rem 0.5rem;
  }
  .hosts li:hover {
    background: var(--karto-color-surface);
  }
  .hosts li.dup {
    opacity: 0.7;
  }
  .host-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.4rem;
    font-size: 0.85rem;
  }
  .alias {
    font-weight: 600;
  }
  .target-host {
    opacity: 0.7;
    font-family: var(--karto-font-mono, monospace);
    font-size: 0.8rem;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 0.05rem 0.35rem;
    border-radius: 999px;
    background: var(--karto-color-surface);
    border: 1px solid var(--karto-color-border);
    font-size: 0.7rem;
    opacity: 0.85;
    white-space: nowrap;
  }
  .file .chip {
    margin-left: auto;
  }
  .chip.key {
    padding: 0.1rem 0.3rem;
  }
  .chip.dup-chip {
    margin-left: auto;
    border-color: color-mix(in srgb, #ffb020 40%, transparent);
    color: #ffb020;
    background: color-mix(in srgb, #ffb020 10%, transparent);
  }
  .count {
    margin: 0;
  }
</style>
