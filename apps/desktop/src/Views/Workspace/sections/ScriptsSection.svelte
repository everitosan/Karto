<script lang="ts">
  // Sección Scripts: elige un diagrama, selecciona sus equipos conectables por
  // SSH, escribe/carga un script de la biblioteca (a nivel de máquina) y lo
  // ejecuta secuencial o en paralelo, viendo la salida por equipo en vivo.
  import { onMount } from "svelte";
  import { Icon, icons, Modal, Button, TechIcon } from "@karto/ui";
  import {
    scriptsUseCases,
    type Interpreter,
    type RunMode,
    type Script,
    type ScriptFolder,
    type ScriptTarget,
    type TargetRun,
  } from "$usecases/scripts";
  import { workspaceUseCases } from "$usecases/workspace";
  import { networkContext } from "../networkContext.svelte";
  import CodeEditor, { type CodeLanguage } from "$components/CodeEditor.svelte";
  import type { InfraMap } from "$domain/infra";
  import { NODE_CATALOG, type NodeKind } from "@karto/catalog";
  import { nodeKindLabel } from "$i18n/catalog";
  import { m } from "$paraglide/messages.js";

  // --- Estado ---------------------------------------------------------------
  // Último diagrama elegido: se persiste en localStorage para no tener que
  // reseleccionarlo al volver a la sección (igual que el acordeón de carpetas).
  const SELECTED_MAP_KEY = "karto.scriptsSelectedMapId";
  let maps = $state<InfraMap[]>([]);
  let selectedMapId = $state<string>(localStorage.getItem(SELECTED_MAP_KEY) ?? "");
  let targets = $state<ScriptTarget[]>([]);
  let selected = $state<Record<string, boolean>>({});

  let library = $state<Script[]>([]);
  let folders = $state<ScriptFolder[]>([]);
  // Acordeón de carpetas de scripts: colapsadas (vacío = todas abiertas). Es
  // chrome de navegación atado a este equipo, así que se persiste en
  // localStorage y sobrevive al cambiar de sección (igual que el árbol de
  // diagramas en Sidebar.svelte).
  const COLLAPSED_KEY = "karto.scriptsCollapsedFolders";

  function readCollapsed(): Record<string, boolean> {
    try {
      const raw = localStorage.getItem(COLLAPSED_KEY);
      if (raw) {
        const next: Record<string, boolean> = {};
        for (const id of JSON.parse(raw) as string[]) next[id] = true;
        return next;
      }
    } catch {
      // localStorage no disponible o JSON inválido → todas abiertas.
    }
    return {};
  }

  function writeCollapsed(map: Record<string, boolean>): void {
    try {
      const ids = Object.keys(map).filter((id) => map[id]);
      localStorage.setItem(COLLAPSED_KEY, JSON.stringify(ids));
    } catch {
      // localStorage no disponible: el estado vive solo en memoria esta sesión.
    }
  }

  let collapsed = $state<Record<string, boolean>>(readCollapsed());
  let editingFolderId = $state<string | null>(null); // carpeta en modo renombrar
  let editingId = $state<string | null>(null);
  const DEFAULT_BODY = "#!/usr/bin/env bash\n# Escribe aquí tu script…\nuname -a\n";
  let scriptName = $state("");
  let scriptBody = $state(DEFAULT_BODY);
  let interpreter = $state<Interpreter>("bash");
  // Icono Devicon (a color) por intérprete, para la lista de scripts.
  const INTERP_ICON: Record<Interpreter, string> = {
    bash: "bash-plain",
    python: "python-plain",
    postgresql: "postgresql-plain",
    mysql: "mysql-original",
    mariadb: "mariadb-original",
    mongodb: "mongodb-plain",
    redis: "redis-plain",
  };
  // Instantánea de lo último guardado/cargado, para detectar cambios sin guardar.
  let savedSnapshot = $state({ name: "", body: DEFAULT_BODY, interpreter: "bash" as Interpreter });

  let mode = $state<RunMode>("sequential");
  let running = $state(false);
  // Salida por equipo (nodeId → run). Se reemplaza en bloque para reactividad.
  let runs = $state<Record<string, TargetRun>>({});
  // Equipo cuya salida se ve a pantalla completa (o null).
  let fullNodeId = $state<string | null>(null);
  const fullRun = $derived(fullNodeId ? (runs[fullNodeId] ?? null) : null);

  // Máquinas reales (con shell) donde corren scripts bash/python: servidores,
  // VMs, hipervisores, contenedores, bastion, NAS y el genérico. Se excluyen los
  // nodos **lógicos** de aplicación (web_server, application, worker, reverse_proxy
  // → son servicios, no la máquina que entras por SSH), los clústeres y las BD y
  // appliances de red (las BD tendrán sus propios intérpretes en Fase 2).
  const SHELL_KINDS = new Set([
    "server",
    "vm",
    "hypervisor",
    "container",
    "generic",
    "bastion",
    "nas",
  ]);
  function isShell(interp: Interpreter): boolean {
    return interp === "bash" || interp === "python";
  }
  // Lenguaje del editor según el intérprete (mongo = JS de mongosh; redis sin
  // resaltado dedicado).
  const EDITOR_LANG: Record<Interpreter, CodeLanguage> = {
    bash: "shell",
    python: "python",
    postgresql: "sql",
    mysql: "sql",
    mariadb: "sql",
    mongodb: "javascript",
    redis: "plain",
  };
  // ¿El objetivo tiene sentido para el intérprete? Shell → host con shell; motor
  // de BD → nodo `database` cuyo `gestor` coincide con el motor exacto.
  function compatibleWith(interp: Interpreter, t: ScriptTarget): boolean {
    if (isShell(interp)) return SHELL_KINDS.has(t.kind);
    return t.kind === "database" && t.gestor === interp;
  }
  // Selectabilidad: shell necesita llave SSH; BD necesita credencial de BD.
  function selectability(interp: Interpreter, t: ScriptTarget): {
    connectable: boolean;
    reason: string | null;
  } {
    if (isShell(interp)) {
      return t.sshKey
        ? { connectable: true, reason: null }
        : { connectable: false, reason: "requiere credencial SSH con llave" };
    }
    return t.dbCred
      ? { connectable: true, reason: null }
      : { connectable: false, reason: "falta credencial de BD" };
  }

  // --- Derivados ------------------------------------------------------------
  // Objetivos compatibles con el intérprete, con su selectabilidad calculada.
  const displayTargets = $derived(
    targets
      .filter((t) => compatibleWith(interpreter, t))
      .map((t) => ({ ...t, ...selectability(interpreter, t) })),
  );
  // Objetivos agrupados por tipo de nodo (servidores, VMs, contenedores…) para
  // localizarlos de un vistazo. El orden es el lógico de cómputo→datos; los tipos
  // no listados caen al final. Cada grupo trae su etiqueta traducida y el icono
  // base del catálogo para la cabecera.
  const KIND_ORDER = [
    "server",
    "vm",
    "hypervisor",
    "container",
    "bastion",
    "nas",
    "generic",
    "database",
  ];
  const groupedTargets = $derived.by(() => {
    const byKind = new Map<string, typeof displayTargets>();
    for (const t of displayTargets) {
      const arr = byKind.get(t.kind) ?? [];
      arr.push(t);
      byKind.set(t.kind, arr);
    }
    return [...byKind.entries()]
      .map(([kind, items]) => ({
        kind,
        label: nodeKindLabel(kind as NodeKind),
        icon: (NODE_CATALOG[kind as NodeKind] ?? NODE_CATALOG.generic).icon,
        items,
      }))
      .sort((a, b) => {
        const ia = KIND_ORDER.indexOf(a.kind);
        const ib = KIND_ORDER.indexOf(b.kind);
        return (ia === -1 ? Infinity : ia) - (ib === -1 ? Infinity : ib);
      });
  });
  const selectedNodeIds = $derived(
    displayTargets.filter((t) => t.connectable && selected[t.nodeId]).map((t) => t.nodeId),
  );
  // Marca/desmarca de golpe todos los equipos conectables de una sección.
  function toggleGroup(items: { nodeId: string }[], checked: boolean) {
    const next = { ...selected };
    for (const t of items) next[t.nodeId] = checked;
    selected = next;
  }
  const canRun = $derived(
    !running && selectedNodeIds.length > 0 && scriptBody.trim().length > 0,
  );
  // Hay cambios sin guardar si el editor difiere de la última instantánea.
  const dirty = $derived(
    scriptName !== savedSnapshot.name ||
      scriptBody !== savedSnapshot.body ||
      interpreter !== savedSnapshot.interpreter,
  );
  const labelOf = $derived((nodeId: string) =>
    targets.find((t) => t.nodeId === nodeId)?.label ?? nodeId,
  );

  // Scripts sueltos (sin carpeta) y los de una carpeta dada.
  const looseScripts = $derived(library.filter((s) => s.folderId === null));
  const scriptsIn = $derived((folderId: string) =>
    library.filter((s) => s.folderId === folderId),
  );

  // --- Carga ----------------------------------------------------------------
  onMount(async () => {
    maps = await workspaceUseCases.listMaps().catch(() => []);
    await reloadLibrary();
    // Descarta el id persistido si el diagrama ya no existe (borrado).
    if (selectedMapId && !maps.some((m) => m.id === selectedMapId)) {
      selectedMapId = "";
    }
    // Sin selección previa y con un solo diagrama, elígelo por comodidad.
    if (!selectedMapId && maps.length === 1) selectedMapId = maps[0].id;
    if (selectedMapId) await loadTargets();
  });

  async function reloadLibrary() {
    library = await scriptsUseCases.list().catch(() => library);
    folders = await scriptsUseCases.listFolders().catch(() => folders);
    // Poda ids colapsados de carpetas ya inexistentes (borradas).
    const alive = new Set(folders.map((f) => f.id));
    const pruned = Object.fromEntries(
      Object.entries(collapsed).filter(([id]) => alive.has(id)),
    );
    if (Object.keys(pruned).length !== Object.keys(collapsed).length) {
      collapsed = pruned;
      writeCollapsed(collapsed);
    }
  }

  async function loadTargets() {
    // Recuerda la elección (o límpiala) para la próxima vez que entres.
    try {
      if (selectedMapId) localStorage.setItem(SELECTED_MAP_KEY, selectedMapId);
      else localStorage.removeItem(SELECTED_MAP_KEY);
    } catch {
      // localStorage no disponible: la selección vive solo en esta sesión.
    }
    if (!selectedMapId) {
      targets = [];
      selected = {};
      return;
    }
    targets = await scriptsUseCases.targets(selectedMapId).catch(() => []);
    // Marca todos por defecto; `selectedNodeIds` filtra luego por compatibilidad
    // y selectabilidad según el intérprete del script.
    const next: Record<string, boolean> = {};
    for (const t of targets) next[t.nodeId] = true;
    selected = next;
  }

  // --- Biblioteca de scripts ------------------------------------------------
  function loadScript(s: Script) {
    editingId = s.id;
    scriptName = s.name;
    scriptBody = s.body;
    interpreter = s.interpreter;
    savedSnapshot = { name: s.name, body: s.body, interpreter: s.interpreter };
  }

  function newScript() {
    editingId = null;
    scriptName = "";
    scriptBody = "";
    interpreter = "bash";
    savedSnapshot = { name: "", body: "", interpreter: "bash" };
  }

  async function saveScript() {
    const name = scriptName.trim();
    if (!name) return;
    await scriptsUseCases.upsert({ id: editingId ?? undefined, name, body: scriptBody, interpreter });
    await reloadLibrary();
    // Tras crear uno nuevo, engánchalo como el que se edita.
    if (!editingId) editingId = library.find((s) => s.name === name)?.id ?? null;
    // Ya está guardado: normaliza el nombre y la instantánea pasa a ser lo
    // actual (deja de estar "sucio").
    scriptName = name;
    savedSnapshot = { name, body: scriptBody, interpreter };
  }

  // Borrado de script con confirmación por modal.
  let pendingDelete = $state<Script | null>(null);

  function askDelete(s: Script) {
    pendingDelete = s;
  }

  async function confirmDelete() {
    const target = pendingDelete;
    pendingDelete = null;
    if (!target) return;
    await scriptsUseCases.remove(target.id);
    await reloadLibrary();
    // Si estabas editando el que borraste, limpia el editor.
    if (editingId === target.id) newScript();
  }

  // --- Carpetas -------------------------------------------------------------
  async function newFolder() {
    const id = await scriptsUseCases.createFolder(m.sidebar_new_folder());
    await reloadLibrary();
    editingFolderId = id; // entra directo a renombrar
  }

  async function commitFolderRename(id: string, value: string) {
    editingFolderId = null;
    const name = value.trim();
    const current = folders.find((f) => f.id === id);
    if (!name || !current || current.name === name) return;
    await scriptsUseCases.renameFolder(id, name);
    await reloadLibrary();
  }

  async function moveScript(scriptId: string, folderId: string | null) {
    closeMenu();
    await scriptsUseCases.setFolder(scriptId, folderId);
    await reloadLibrary();
  }

  function toggleCollapse(id: string) {
    collapsed = { ...collapsed, [id]: !collapsed[id] };
    writeCollapsed(collapsed);
  }

  // Borrado de carpeta con confirmación (sus scripts se conservan, quedan sueltos).
  let pendingFolderDelete = $state<ScriptFolder | null>(null);

  async function confirmFolderDelete() {
    const target = pendingFolderDelete;
    pendingFolderDelete = null;
    if (!target) return;
    await scriptsUseCases.removeFolder(target.id);
    await reloadLibrary();
  }

  // --- Menú contextual ------------------------------------------------------
  // Un único menú: sobre un script (mover/borrar) o sobre una carpeta (renombrar/borrar).
  type Menu =
    | { kind: "script"; x: number; y: number; script: Script }
    | { kind: "folder"; x: number; y: number; folder: ScriptFolder };
  let menu = $state<Menu | null>(null);

  function openScriptMenu(e: MouseEvent, script: Script) {
    e.preventDefault();
    menu = { kind: "script", x: e.clientX, y: e.clientY, script };
  }
  function openFolderMenu(e: MouseEvent, folder: ScriptFolder) {
    e.preventDefault();
    menu = { kind: "folder", x: e.clientX, y: e.clientY, folder };
  }
  function closeMenu() {
    menu = null;
  }

  // --- Ejecución ------------------------------------------------------------
  async function run() {
    if (!canRun) return;
    running = true;
    const nodeIds = selectedNodeIds;
    // Estado inicial: todos pendientes.
    const initial: Record<string, TargetRun> = {};
    for (const id of nodeIds) initial[id] = { nodeId: id, status: "pending", output: "" };
    runs = initial;

    // El backend ejecuta en segundo plano y la salida llega por eventos. El
    // `invoke` resuelve al instante (no al terminar), así que cerramos el estado
    // "ejecutando" al recibir un `done` por cada equipo.
    let done = 0;
    try {
      await scriptsUseCases.run(
        { nodeIds, body: scriptBody, interpreter, mode, contextId: networkContext.activeId },
        (ev) => {
          const cur = runs[ev.nodeId];
          if (!cur) return;
          if (ev.type === "status") {
            runs = { ...runs, [ev.nodeId]: { ...cur, status: ev.status } };
          } else if (ev.type === "line") {
            runs = {
              ...runs,
              [ev.nodeId]: { ...cur, output: cur.output + ev.line + "\n" },
            };
          } else {
            runs = {
              ...runs,
              [ev.nodeId]: { ...cur, exitCode: ev.exitCode, error: ev.error },
            };
            done += 1;
            if (done >= nodeIds.length) running = false;
          }
        },
      );
    } catch {
      // Fallo al lanzar el comando: no habrá eventos, reabre la UI.
      running = false;
    }
  }
</script>

{#snippet libItem(s: Script, indented: boolean)}
  <div
    class="lib-item"
    class:active={editingId === s.id}
    class:indented
    oncontextmenu={(e) => openScriptMenu(e, s)}
    role="listitem"
  >
    <button class="lib-load" onclick={() => loadScript(s)}>
      <TechIcon name={INTERP_ICON[s.interpreter]} size={15} />
      <span>{s.name}</span>
    </button>
    <button
      class="lib-del"
      onclick={() => askDelete(s)}
      title={m.scripts_delete_short()}
      aria-label={m.scripts_delete_named({ name: s.name })}
    >
      <Icon icon={icons.delete} size={14} />
    </button>
  </div>
{/snippet}

<div class="scripts">
  <!-- Barra de ejecución: diagrama objetivo + modo + lanzar. -->
  <div class="toolbar">
    <label class="ctl">
      <span>{m.scripts_diagram()}</span>
      <select bind:value={selectedMapId} onchange={loadTargets} disabled={running}>
        <option value="">{m.scripts_select_diagram()}</option>
        {#each maps as mp (mp.id)}
          <option value={mp.id}>{mp.name}</option>
        {/each}
      </select>
    </label>

    <div class="mode" role="group" aria-label={m.scripts_run_mode()}>
      <button
        class="seg"
        class:active={mode === "sequential"}
        onclick={() => (mode = "sequential")}
        disabled={running}
      >
        {m.scripts_sequential()}
      </button>
      <button
        class="seg"
        class:active={mode === "parallel"}
        onclick={() => (mode = "parallel")}
        disabled={running}
      >
        {m.scripts_parallel()}
      </button>
    </div>

    <button class="run" class:enabled={canRun} disabled={!canRun} onclick={run}>
      <Icon icon={icons.logout} size={16} />
      {running ? m.scripts_running() : m.scripts_run()}
    </button>
  </div>

  <div class="grid">
    <!-- (b) Biblioteca + editor de scripts. -->
    <section class="panel editor">
      <header>
        <Icon icon={icons.script} size={15} /> {m.scripts_script()}
      </header>
      <div class="editor-body">
        <div class="library">
          <div class="library-head">
            <span>{m.scripts_library()}</span>
            <div class="library-actions">
              <button onclick={newFolder} title={m.sidebar_new_folder()} aria-label={m.sidebar_new_folder()}>
                <Icon icon={icons.folder} size={15} />
              </button>
              <button onclick={newScript} title={m.scripts_new_script()} aria-label={m.scripts_new_script()}>
                <Icon icon={icons.add} size={15} />
              </button>
            </div>
          </div>
          {#each folders as f (f.id)}
            <div class="folder">
              <div class="folder-head" oncontextmenu={(e) => openFolderMenu(e, f)} role="group">
                <button
                  class="folder-toggle"
                  onclick={() => toggleCollapse(f.id)}
                  aria-label={collapsed[f.id] ? m.common_expand() : m.common_collapse()}
                >
                  <span class="chev" class:open={!collapsed[f.id]}>
                    <Icon icon={icons.chevron} size={13} />
                  </span>
                  <Icon icon={icons.folder} size={14} />
                  {#if editingFolderId === f.id}
                    <!-- svelte-ignore a11y_autofocus -->
                    <input
                      class="folder-rename"
                      value={f.name}
                      autofocus
                      onclick={(e) => e.stopPropagation()}
                      onblur={(e) => commitFolderRename(f.id, e.currentTarget.value)}
                      onkeydown={(e) => {
                        if (e.key === "Enter") e.currentTarget.blur();
                        else if (e.key === "Escape") (editingFolderId = null);
                      }}
                    />
                  {:else}
                    <span class="fname">{f.name}</span>
                    <span class="count">{scriptsIn(f.id).length}</span>
                  {/if}
                </button>
              </div>
              {#if !collapsed[f.id]}
                {#each scriptsIn(f.id) as s (s.id)}
                  {@render libItem(s, true)}
                {/each}
              {/if}
            </div>
          {/each}

          {#each looseScripts as s (s.id)}
            {@render libItem(s, false)}
          {/each}

          {#if library.length === 0 && folders.length === 0}
            <div class="empty small">{m.scripts_empty_library()}</div>
          {/if}
        </div>
        <div class="editor-pane">
          <div class="editor-toolbar">
            <input
              class="name"
              bind:value={scriptName}
              placeholder={m.scripts_name_placeholder()}
              spellcheck="false"
            />
            <select class="interp" bind:value={interpreter} title={m.scripts_interpreter()}>
              <optgroup label={m.scripts_group_shell()}>
                <option value="bash">Bash</option>
                <option value="python">Python</option>
              </optgroup>
              <optgroup label={m.scripts_group_db()}>
                <option value="postgresql">PostgreSQL</option>
                <option value="mysql">MySQL</option>
                <option value="mariadb">MariaDB</option>
                <option value="mongodb">MongoDB</option>
                <option value="redis">Redis</option>
              </optgroup>
            </select>
          </div>
          <CodeEditor bind:value={scriptBody} language={EDITOR_LANG[interpreter]} />
          <button
            class="save-fab"
            class:dirty
            onclick={saveScript}
            disabled={!scriptName.trim()}
            title={dirty ? m.scripts_save_dirty() : m.common_save()}
            aria-label={m.common_save()}
          >
            <Icon icon={icons.save} size={16} />
          </button>
        </div>
      </div>
    </section>

    <!-- (a) Equipos objetivo (nodos conectables del diagrama). -->
    <section class="panel targets">
      <header><Icon icon={icons.connect} size={15} /> {m.scripts_targets()}</header>
      {#if !selectedMapId}
        <div class="empty">{m.scripts_targets_empty_nomap()}</div>
      {:else if targets.length === 0}
        <div class="empty">{m.scripts_targets_empty()}</div>
      {:else if displayTargets.length === 0}
        <div class="empty">{m.scripts_targets_incompatible({ interpreter })}</div>
      {:else}
        <ul class="target-list">
          {#each groupedTargets as g (g.kind)}
            {@const conn = g.items.filter((t) => t.connectable)}
            {@const allSel = conn.length > 0 && conn.every((t) => selected[t.nodeId])}
            {@const someSel = conn.some((t) => selected[t.nodeId])}
            <li class="group-head" class:disabled={conn.length === 0}>
              <input
                type="checkbox"
                checked={allSel}
                indeterminate={someSel && !allSel}
                disabled={conn.length === 0 || running}
                onchange={(e) => toggleGroup(conn, e.currentTarget.checked)}
                aria-label={g.label}
              />
              <Icon icon={g.icon} size={14} />
              <span class="gname">{g.label}</span>
              <span class="gcount">{g.items.length}</span>
            </li>
            {#each g.items as t (t.nodeId)}
              <li class:disabled={!t.connectable}>
                <label>
                  <input
                    type="checkbox"
                    checked={t.connectable && selected[t.nodeId]}
                    disabled={!t.connectable || running}
                    onchange={(e) => (selected[t.nodeId] = e.currentTarget.checked)}
                  />
                  <span class="tlabel">{t.label}</span>
                  {#if (t.kind === "server" || t.kind === "vm") && t.os}
                    <span class="tmeta">{t.os}</span>
                  {:else if t.kind === "database" && t.instance}
                    <span class="tmeta">— {t.instance}</span>
                  {/if}
                </label>
                {#if !t.connectable}
                  <span class="reason">{t.reason}</span>
                {/if}
              </li>
            {/each}
          {/each}
        </ul>
      {/if}
    </section>

    <!-- (d) Salida por equipo. -->
    <section class="panel output">
      <header><Icon icon={icons.terminal} size={15} /> {m.scripts_output()}</header>
      {#if Object.keys(runs).length === 0}
        <div class="empty">{m.scripts_output_empty()}</div>
      {:else}
        <div class="out-list">
          {#each Object.values(runs) as r (r.nodeId)}
            <div class="out-item">
              <div class="out-head">
                <span class="badge {r.status}">{r.status}</span>
                <span class="tlabel">{labelOf(r.nodeId)}</span>
                {#if r.exitCode != null && r.exitCode !== 0}
                  <span class="code">exit {r.exitCode}</span>
                {/if}
                <button
                  class="out-full"
                  onclick={() => (fullNodeId = r.nodeId)}
                  title={m.scripts_fullscreen_enter()}
                  aria-label={m.scripts_fullscreen_enter()}
                >
                  <Icon icon={icons.expand} size={14} />
                </button>
              </div>
              {#if r.error}
                <pre class="err">{r.error}</pre>
              {/if}
              {#if r.output}
                <pre>{r.output}</pre>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </section>
  </div>
</div>

<!-- Salida de un equipo a pantalla completa. -->
{#if fullRun}
  <div class="full-overlay">
    <div class="full-head">
      <span class="badge {fullRun.status}">{fullRun.status}</span>
      <span class="tlabel">{labelOf(fullRun.nodeId)}</span>
      {#if fullRun.exitCode != null && fullRun.exitCode !== 0}
        <span class="code">exit {fullRun.exitCode}</span>
      {/if}
      <button
        class="out-full"
        onclick={() => (fullNodeId = null)}
        title={m.scripts_fullscreen_exit()}
        aria-label={m.scripts_fullscreen_exit()}
      >
        <Icon icon={icons.minimize} size={16} />
      </button>
    </div>
    <div class="full-body">
      {#if fullRun.error}
        <pre class="err">{fullRun.error}</pre>
      {/if}
      {#if fullRun.output}
        <pre>{fullRun.output}</pre>
      {:else if !fullRun.error}
        <div class="empty">{m.scripts_no_output()}</div>
      {/if}
    </div>
  </div>
{/if}

<Modal
  open={pendingDelete !== null}
  title={m.scripts_delete_script()}
  width="24rem"
  onClose={() => (pendingDelete = null)}
>
  <p class="confirm-text">
    {m.scripts_confirm_del_script_before()}<strong>«{pendingDelete?.name}»</strong>{m.scripts_confirm_del_script_after()}
  </p>
  {#snippet footer()}
    <Button variant="secondary" onclick={() => (pendingDelete = null)}>{m.common_cancel()}</Button>
    <Button variant="danger" onclick={confirmDelete}>{m.scripts_delete_short()}</Button>
  {/snippet}
</Modal>

<Modal
  open={pendingFolderDelete !== null}
  title={m.scripts_delete_folder()}
  width="24rem"
  onClose={() => (pendingFolderDelete = null)}
>
  <p class="confirm-text">
    {m.scripts_confirm_del_folder_before()}<strong>«{pendingFolderDelete?.name}»</strong>{m.scripts_confirm_del_folder_after()}
  </p>
  {#snippet footer()}
    <Button variant="secondary" onclick={() => (pendingFolderDelete = null)}>{m.common_cancel()}</Button>
    <Button variant="danger" onclick={confirmFolderDelete}>{m.scripts_delete_short()}</Button>
  {/snippet}
</Modal>

<!-- Menú contextual (script o carpeta). Cierra al hacer clic fuera / Escape. -->
{#if menu}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="menu-backdrop" onclick={closeMenu} oncontextmenu={(e) => e.preventDefault()}>
    <div class="ctx-menu" style="left: {menu.x}px; top: {menu.y}px" onclick={(e) => e.stopPropagation()}>
      {#if menu.kind === "script"}
        {@const script = menu.script}
        <div class="ctx-label">{m.scripts_move_to()}</div>
        <button class="ctx-item" onclick={() => moveScript(script.id, null)}>{m.scripts_no_folder()}</button>
        {#each folders as f (f.id)}
          <button class="ctx-item" onclick={() => moveScript(script.id, f.id)}>
            <Icon icon={icons.folder} size={13} />
            {f.name}
          </button>
        {/each}
        <div class="ctx-sep"></div>
        <button
          class="ctx-item danger"
          onclick={() => {
            closeMenu();
            askDelete(script);
          }}
        >
          <Icon icon={icons.delete} size={13} /> {m.scripts_delete_script()}
        </button>
      {:else}
        {@const folder = menu.folder}
        <button
          class="ctx-item"
          onclick={() => {
            editingFolderId = folder.id;
            closeMenu();
          }}
        >
          <Icon icon={icons.edit} size={13} /> {m.common_rename()}
        </button>
        <button
          class="ctx-item danger"
          onclick={() => {
            pendingFolderDelete = folder;
            closeMenu();
          }}
        >
          <Icon icon={icons.delete} size={13} /> {m.scripts_delete_folder()}
        </button>
      {/if}
    </div>
  </div>
{/if}

<svelte:window
  onkeydown={(e) => {
    if (e.key !== "Escape") return;
    if (fullNodeId) fullNodeId = null;
    else closeMenu();
  }}
/>

<style>
  .scripts {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 1rem;
    gap: 0.75rem;
  }
  .toolbar {
    display: flex;
    align-items: flex-end;
    gap: 1rem;
  }
  .ctl {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.8rem;
    color: var(--karto-color-text-muted);
  }
  .ctl select {
    padding: 0.35rem 0.5rem;
    background: var(--karto-color-surface);
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    color: var(--karto-color-text);
    color-scheme: dark;
    cursor: pointer;
  }
  .ctl option {
    background: var(--karto-color-bg, #0f1520);
    color: var(--karto-color-text);
  }
  .mode {
    display: inline-flex;
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    overflow: hidden;
  }
  .seg {
    padding: 0.35rem 0.75rem;
    border: 0;
    background: transparent;
    color: var(--karto-color-text-muted);
    cursor: pointer;
    font-size: 0.85rem;
  }
  .seg.active {
    background: var(--karto-color-accent);
    color: #04140a;
  }
  .run {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.4rem 0.9rem;
    border: 0;
    border-radius: var(--karto-radius);
    background: var(--karto-color-accent);
    color: #04140a;
    font-weight: 600;
    cursor: not-allowed;
    opacity: 0.5;
  }
  .run.enabled {
    cursor: pointer;
    opacity: 1;
  }
  .grid {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 1.4fr 1fr;
    grid-template-rows: 1fr 1fr;
    gap: 0.75rem;
  }
  .panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    overflow: hidden;
  }
  .editor {
    grid-column: 1;
    grid-row: 1 / span 2;
  }
  .targets {
    grid-column: 2;
    grid-row: 1;
  }
  .output {
    grid-column: 2;
    grid-row: 2;
  }
  .panel header {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.4rem 0.6rem;
    border-bottom: 1px solid var(--karto-color-border);
    font-size: 0.8rem;
    color: var(--karto-color-text);
    background: var(--karto-color-surface);
  }
  .library-actions {
    display: inline-flex;
    gap: 0.3rem;
  }
  .library-actions button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0.2rem;
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    background: transparent;
    color: var(--karto-color-text);
    cursor: pointer;
  }
  .library-actions button:hover:not(:disabled) {
    background: color-mix(in srgb, var(--karto-color-accent) 15%, transparent);
  }
  .library-actions button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .editor-body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(120px, 0.4fr) 1fr;
  }
  .library {
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    border-right: 1px solid var(--karto-color-border);
  }
  .library-head {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.35rem 0.4rem 0.35rem 0.6rem;
    border-bottom: 1px solid var(--karto-color-border);
    background: var(--karto-color-surface);
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--karto-color-text-muted);
  }
  .lib-item {
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--karto-color-border);
  }
  .lib-item.active {
    background: color-mix(in srgb, var(--karto-color-accent) 20%, transparent);
  }
  .lib-load {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    text-align: left;
    padding: 0.4rem 0.6rem;
    border: 0;
    background: transparent;
    color: var(--karto-color-text);
    font-size: 0.8rem;
    cursor: pointer;
  }
  .lib-load :global(svg) {
    flex-shrink: 0;
    color: var(--karto-color-text-muted);
  }
  .lib-load :global(i) {
    flex-shrink: 0;
  }
  .lib-load span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .lib-del {
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0.35rem 0.5rem;
    border: 0;
    background: transparent;
    color: var(--karto-color-text-muted);
    cursor: pointer;
    opacity: 0;
  }
  .lib-item:hover .lib-del,
  .lib-item.active .lib-del {
    opacity: 1;
  }
  .lib-del:hover {
    color: #d9534f;
  }
  .lib-item.indented .lib-load {
    padding-left: 1.5rem;
  }
  /* Carpetas */
  .folder-head {
    border-bottom: 1px solid var(--karto-color-border);
  }
  .folder-toggle {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.35rem;
    text-align: left;
    padding: 0.4rem 0.6rem;
    border: 0;
    background: color-mix(in srgb, var(--karto-color-surface) 60%, transparent);
    color: var(--karto-color-text);
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }
  .folder-toggle :global(svg) {
    flex-shrink: 0;
    color: var(--karto-color-text-muted);
  }
  .chev {
    display: inline-flex;
    transition: transform 0.12s ease;
  }
  .chev.open {
    transform: rotate(90deg);
  }
  .fname {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .count {
    flex-shrink: 0;
    font-size: 0.7rem;
    font-weight: 500;
    color: var(--karto-color-text-muted);
    background: var(--karto-color-border);
    border-radius: 999px;
    padding: 0.05rem 0.4rem;
  }
  .folder-rename {
    flex: 1;
    min-width: 0;
    border: 1px solid var(--karto-color-accent);
    border-radius: var(--karto-radius);
    padding: 0.1rem 0.3rem;
    background: var(--karto-color-bg, #0f1520);
    color: var(--karto-color-text);
    font-size: 0.8rem;
  }
  .folder-rename:focus {
    outline: none;
  }
  /* Menú contextual */
  .menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1100;
  }
  .ctx-menu {
    position: fixed;
    min-width: 11rem;
    max-height: 60vh;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    padding: 0.25rem;
    background: var(--karto-color-bg, #0b0f17);
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    box-shadow: 0 0.75rem 1.5rem rgba(0, 0, 0, 0.5);
  }
  .ctx-label {
    padding: 0.25rem 0.5rem;
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--karto-color-text-muted);
  }
  .ctx-item {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    text-align: left;
    padding: 0.35rem 0.5rem;
    border: 0;
    border-radius: var(--karto-radius);
    background: transparent;
    color: var(--karto-color-text);
    font-size: 0.8rem;
    cursor: pointer;
  }
  .ctx-item:hover {
    background: color-mix(in srgb, var(--karto-color-accent) 18%, transparent);
  }
  .ctx-item.danger {
    color: #d9534f;
  }
  .ctx-item :global(svg) {
    flex-shrink: 0;
    color: currentColor;
  }
  .ctx-sep {
    height: 1px;
    margin: 0.25rem 0.3rem;
    background: var(--karto-color-border);
  }
  .confirm-text {
    margin: 0;
    font-size: 0.9rem;
    line-height: 1.5;
    color: var(--karto-color-text);
  }
  .editor-pane {
    position: relative;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  /* Botón guardar: siempre outline (sin fondo). Icono/contorno atenuados cuando
     está guardado; en amarillo cuando hay cambios sin guardar. */
  .save-fab {
    position: absolute;
    right: 0.75rem;
    bottom: 0.75rem;
    z-index: 2;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0.35rem;
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    background: transparent;
    color: var(--karto-color-text-muted);
    cursor: pointer;
  }
  .save-fab:hover:not(:disabled) {
    color: var(--karto-color-text);
    border-color: var(--karto-color-text-muted);
  }
  .save-fab.dirty {
    color: #d0a215;
    border-color: #d0a215;
  }
  .save-fab:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .save-fab :global(svg) {
    color: currentColor;
  }
  .editor-toolbar {
    display: flex;
    align-items: stretch;
    border-bottom: 1px solid var(--karto-color-border);
  }
  .editor-pane .name {
    flex: 1;
    min-width: 0;
    border: 0;
    padding: 0.4rem 0.6rem;
    background: transparent;
    color: var(--karto-color-text);
    font-size: 0.85rem;
  }
  .editor-pane .name:focus {
    outline: none;
  }
  .editor-pane .interp {
    border: 0;
    border-left: 1px solid var(--karto-color-border);
    padding: 0.2rem 0.5rem;
    background: var(--karto-color-surface);
    color: var(--karto-color-text);
    color-scheme: dark;
    font-size: 0.8rem;
    cursor: pointer;
  }
  .editor-pane .interp:focus {
    outline: none;
  }
  .editor-pane .interp option {
    background: var(--karto-color-bg, #0f1520);
    color: var(--karto-color-text);
  }
  .target-list,
  .out-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    margin: 0;
    padding: 0.4rem;
    list-style: none;
  }
  .target-list li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.25rem 0.3rem;
    font-size: 0.85rem;
  }
  .target-list li.disabled .tlabel {
    color: var(--karto-color-text-muted);
  }
  .target-list li.disabled label {
    cursor: not-allowed;
  }
  .target-list li.disabled input {
    cursor: not-allowed;
  }
  /* Cabecera de grupo por tipo de nodo (servidores, VMs…). */
  .target-list li.group-head {
    gap: 0.4rem;
    justify-content: flex-start;
    margin-top: 0.35rem;
    padding: 0.2rem 0.3rem;
    font-size: 0.68rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--karto-color-text-muted);
    border-bottom: 1px solid var(--karto-color-border);
  }
  .target-list li.group-head:first-child {
    margin-top: 0;
  }
  .target-list li.group-head input {
    cursor: pointer;
  }
  .target-list li.group-head.disabled input {
    cursor: not-allowed;
  }
  .target-list li.group-head :global(svg) {
    color: var(--karto-color-text-muted);
  }
  .group-head .gname {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .group-head .gcount {
    flex-shrink: 0;
    font-weight: 500;
    color: var(--karto-color-text-muted);
    background: var(--karto-color-border);
    border-radius: 999px;
    padding: 0.02rem 0.35rem;
  }
  .target-list label {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    cursor: pointer;
  }
  .reason {
    font-size: 0.72rem;
    color: var(--karto-color-text-muted);
    font-style: italic;
  }
  /* Detalle junto al nombre: SO (server/vm) o nombre de la BD (database). */
  .tmeta {
    font-size: 0.72rem;
    color: var(--karto-color-text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .out-item {
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    margin-bottom: 0.5rem;
    overflow: hidden;
  }
  .out-head {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.3rem 0.5rem;
    background: var(--karto-color-surface);
    font-size: 0.8rem;
  }
  .badge {
    text-transform: uppercase;
    font-size: 0.65rem;
    font-weight: 700;
    padding: 0.1rem 0.4rem;
    border-radius: 999px;
    letter-spacing: 0.03em;
  }
  .badge.pending {
    background: var(--karto-color-border);
    color: var(--karto-color-text-muted);
  }
  .badge.running {
    background: #b58900;
    color: #04140a;
  }
  .badge.ok {
    background: var(--karto-color-accent);
    color: #04140a;
  }
  .badge.error {
    background: #d9534f;
    color: #fff;
  }
  .code {
    font-family: var(--karto-font-mono, monospace);
    font-size: 0.72rem;
    color: #d9534f;
  }
  .out-item pre {
    margin: 0;
    padding: 0.5rem;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: var(--karto-font-mono, monospace);
    font-size: 0.8rem;
    color: var(--karto-color-text);
  }
  .out-item pre.err {
    color: #d9534f;
  }
  .out-full {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0.15rem;
    border: 0;
    border-radius: var(--karto-radius);
    background: transparent;
    color: var(--karto-color-text-muted);
    cursor: pointer;
  }
  .out-full:hover {
    color: var(--karto-color-text);
  }
  /* Overlay de salida a pantalla completa */
  .full-overlay {
    position: fixed;
    inset: 0;
    z-index: 1200;
    display: flex;
    flex-direction: column;
    background: var(--karto-color-bg, #0b0f17);
  }
  .full-head {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.6rem 0.9rem;
    border-bottom: 1px solid var(--karto-color-border);
    background: var(--karto-color-surface);
    font-size: 0.9rem;
  }
  .full-body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 0.5rem 0.25rem;
  }
  .full-body pre {
    margin: 0;
    padding: 0.5rem 0.75rem;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: var(--karto-font-mono, monospace);
    font-size: 0.9rem;
    line-height: 1.5;
    color: var(--karto-color-text);
  }
  .full-body pre.err {
    color: #d9534f;
  }
  .empty {
    flex: 1;
    display: grid;
    place-items: center;
    padding: 1rem;
    text-align: center;
    font-size: 0.85rem;
    color: var(--karto-color-text-muted);
  }
  .empty.small {
    padding: 0.5rem;
    font-size: 0.75rem;
  }
</style>
