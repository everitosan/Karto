<script lang="ts">
  // Editor del canvas para un diagrama. Vive dentro de <SvelteFlowProvider>
  // (Canvas) para poder usar useSvelteFlow (screenToFlowPosition, viewport).
  // Orquesta la carga del grafo y el autoguardado vía comandos Tauri.
  import { onMount, untrack } from "svelte";
  import {
    SvelteFlow,
    Background,
    Controls,
    useSvelteFlow,
    type Node as FlowNode,
    type Edge as FlowEdge,
    type NodeTypes,
    type EdgeTypes,
    type Connection,
  } from "@xyflow/svelte";
  import "@xyflow/svelte/dist/style.css";
  import { Icon, icons } from "@karto/ui";
  import type { Graph, InfraEdge, InfraNode, NodeKind } from "$domain/infra";
  import { NODE_KIND_LABELS } from "$domain/infra";
  import { workspaceUseCases as uc } from "$usecases/workspace";
  import InfraNodeView from "./InfraNode.svelte";
  import ZoneNodeView from "./ZoneNode.svelte";
  import NodePalette from "./NodePalette.svelte";
  import PropertiesPanel from "./PropertiesPanel.svelte";
  import NodeContextMenu from "./NodeContextMenu.svelte";
  import InfraEdgeView from "./InfraEdge.svelte";
  import { DND_MIME } from "./dnd";

  interface Props {
    mapId: string;
  }

  let { mapId }: Props = $props();

  const nodeTypes: NodeTypes = {
    infra: InfraNodeView as unknown as NodeTypes[string],
    zone: ZoneNodeView as unknown as NodeTypes[string],
  };
  const edgeTypes: EdgeTypes = {
    infra: InfraEdgeView as unknown as EdgeTypes[string],
  };

  const ZONE_DEFAULT = { w: 260, h: 170 };

  let flowNodes = $state.raw<FlowNode[]>([]);
  let flowEdges = $state.raw<FlowEdge[]>([]);
  let selectedNodeId = $state<string | null>(null);
  // Menú contextual (click derecho) sobre un nodo.
  let contextMenu = $state<{ x: number; y: number; nodeId: string; kind: NodeKind } | null>(null);
  // Barra flotante de la arista seleccionada: punto medio en coords de flujo.
  let edgeToolbar = $state<{ id: string; fx: number; fy: number } | null>(null);
  // Se incrementa al mover/zoom para recalcular la posición en pantalla del overlay.
  let viewportTick = $state(0);

  const { screenToFlowPosition, getViewport, flowToScreenPosition, getIntersectingNodes } =
    useSvelteFlow();

  const selectedNode = $derived(
    selectedNodeId ? flowNodes.find((n) => n.id === selectedNodeId) : undefined,
  );

  // Posición en pantalla de la barra (depende del viewport → viewportTick).
  const toolbarPos = $derived.by(() => {
    void viewportTick;
    if (!edgeToolbar) return null;
    return flowToScreenPosition({ x: edgeToolbar.fx, y: edgeToolbar.fy });
  });
  const toolbarShape = $derived(
    edgeToolbar
      ? ((flowEdges.find((e) => e.id === edgeToolbar!.id)?.data?.shape as string) ?? "default")
      : "default",
  );

  // Formas de línea (mini-trazos), como en Obsidian.
  const EDGE_SHAPES: { value: string; title: string; d: string }[] = [
    { value: "default", title: "Curva", d: "M2 10 C8 10 16 2 22 2" },
    { value: "smoothstep", title: "Escalonada suave", d: "M2 10 H9 Q12 10 12 7 V5 Q12 2 15 2 H22" },
    { value: "step", title: "Escalonada", d: "M2 10 H12 V2 H22" },
    { value: "straight", title: "Recta", d: "M2 6 H22" },
  ];

  const toFlowNode = (n: InfraNode): FlowNode => {
    if (n.kind === "zone") {
      const w = Number(n.properties._w) || ZONE_DEFAULT.w;
      const h = Number(n.properties._h) || ZONE_DEFAULT.h;
      return {
        id: n.id,
        type: "zone",
        position: { x: n.x, y: n.y },
        parentId: n.parentId ?? undefined,
        width: w,
        height: h,
        zIndex: 0,
        data: {
          kind: n.kind,
          label: n.label,
          properties: n.properties,
          onResize: (rw: number, rh: number) => persistZoneSize(n.id, rw, rh),
        },
      };
    }
    return {
      id: n.id,
      type: "infra",
      position: { x: n.x, y: n.y },
      parentId: n.parentId ?? undefined,
      zIndex: 1,
      data: { kind: n.kind, label: n.label, properties: n.properties },
    };
  };

  // Svelte Flow exige que un nodo padre aparezca ANTES que sus hijos en el array.
  // Solo las zonas son padres (sin padre propio), así que basta ponerlas primero.
  function sortParentsFirst(nodes: FlowNode[]): FlowNode[] {
    return [...nodes].sort((a, b) => (a.parentId ? 1 : 0) - (b.parentId ? 1 : 0));
  }

  // Handles conectados: si el nodo es origen de alguna arista, su handle source
  // (der.) queda visible; si es destino, su handle target (izq.). Se marca con
  // las clases `src-conn`/`tgt-conn` en el nodo para mostrar SOLO el handle en uso.
  const srcIds = $derived(new Set(flowEdges.map((e) => e.source)));
  const tgtIds = $derived(new Set(flowEdges.map((e) => e.target)));
  $effect(() => {
    const s = srcIds;
    const t = tgtIds;
    const current = untrack(() => flowNodes);
    let changed = false;
    const next = current.map((n) => {
      const desired =
        [s.has(n.id) ? "src-conn" : "", t.has(n.id) ? "tgt-conn" : ""]
          .filter(Boolean)
          .join(" ") || undefined;
      if ((n.class ?? undefined) === desired) return n;
      changed = true;
      if (desired) return { ...n, class: desired };
      const { class: _drop, ...rest } = n;
      return rest as FlowNode;
    });
    if (changed) flowNodes = next;
  });

  // Persiste el tamaño de una zona en sus propiedades (`_w`/`_h`), conservando
  // el resto (tipo/cidr/color/notas).
  async function persistZoneSize(id: string, w: number, h: number) {
    const node = flowNodes.find((n) => n.id === id);
    const props = {
      ...((node?.data?.properties as Record<string, string>) ?? {}),
      _w: String(Math.round(w)),
      _h: String(Math.round(h)),
    };
    await uc.setNodeProperties(id, props);
    flowNodes = flowNodes.map((n) =>
      n.id === id
        ? { ...n, width: w, height: h, data: { ...n.data, properties: props } }
        : n,
    );
  }

  // Forma de la línea guardada en el `style` JSON de la arista (`{ shape }`).
  function edgeShape(style: string): string {
    try {
      const s = JSON.parse(style || "{}");
      return typeof s.shape === "string" ? s.shape : "default";
    } catch {
      return "default";
    }
  }

  // Datos de la arista custom: forma + reporte del punto medio para la barra.
  function edgeData(id: string, shape: string) {
    return {
      shape,
      onGeom: (fx: number, fy: number) => (edgeToolbar = { id, fx, fy }),
      onDeselect: () => {
        if (edgeToolbar?.id === id) edgeToolbar = null;
      },
    };
  }

  const toFlowEdge = (e: InfraEdge): FlowEdge => ({
    id: e.id,
    source: e.sourceId,
    target: e.targetId,
    label: e.label ?? undefined,
    type: "infra",
    data: edgeData(e.id, edgeShape(e.style)),
  });

  onMount(async () => {
    const graph: Graph = await uc.loadGraph(mapId);
    flowNodes = sortParentsFirst(graph.nodes.map(toFlowNode));
    flowEdges = graph.edges.map(toFlowEdge);
  });

  // --- Agrupación (nodos dentro de una zona) ---

  // Posición absoluta de un nodo (suma la del padre si está agrupado).
  function absolutePos(node: FlowNode): { x: number; y: number } {
    if (node.parentId) {
      const parent = flowNodes.find((n) => n.id === node.parentId);
      if (parent) {
        return { x: parent.position.x + node.position.x, y: parent.position.y + node.position.y };
      }
    }
    return { x: node.position.x, y: node.position.y };
  }

  // Zona que contiene un nodo tras arrastrarlo (la más pequeña si hay varias).
  // Usa la intersección de rectángulos de Svelte Flow (coordenadas absolutas).
  function zoneUnder(node: FlowNode): FlowNode | undefined {
    const zones = getIntersectingNodes(node).filter(
      (n) => n.type === "zone" && n.id !== node.id,
    );
    return zones.sort(
      (a, b) => (a.width ?? 0) * (a.height ?? 0) - (b.width ?? 0) * (b.height ?? 0),
    )[0];
  }

  // --- Autoguardado ---
  async function onNodeDragStop({ targetNode }: { targetNode: FlowNode | null }) {
    if (!targetNode) return;
    const node = targetNode;

    // Las zonas no se anidan entre sí: solo persistir su posición.
    if (node.type === "zone") {
      uc.setNodePosition(node.id, node.position.x, node.position.y);
      return;
    }

    const oldParentId = node.parentId;
    const newParentId = zoneUnder(node)?.id;

    if (newParentId === oldParentId) {
      // Mismo grupo (o sin grupo): la posición ya está en el sistema correcto.
      uc.setNodePosition(node.id, node.position.x, node.position.y);
      return;
    }

    // Reparent: convertir la posición entre sistemas de coordenadas.
    const abs = absolutePos(node);
    let pos = abs;
    if (newParentId) {
      const zone = flowNodes.find((n) => n.id === newParentId)!;
      pos = { x: abs.x - zone.position.x, y: abs.y - zone.position.y };
    }
    flowNodes = sortParentsFirst(
      flowNodes.map((n) =>
        n.id === node.id ? { ...n, parentId: newParentId, position: pos } : n,
      ),
    );
    await uc.setNodeParent(node.id, newParentId ?? null);
    await uc.setNodePosition(node.id, pos.x, pos.y);
  }

  async function onConnect(conn: Connection) {
    // Svelte Flow ya añadió una arista visual con id temporal; la persistimos y
    // reemplazamos su id por el del backend.
    const be = await uc.createEdge(mapId, conn.source, conn.target);
    flowEdges = flowEdges.map((e) =>
      e.source === conn.source &&
      e.target === conn.target &&
      String(e.id).startsWith("xy-edge")
        ? { ...e, id: be.id, type: "infra", data: edgeData(be.id, "default") }
        : e,
    );
  }

  async function onDelete({ nodes, edges }: { nodes: FlowNode[]; edges: FlowEdge[] }) {
    for (const n of nodes) {
      await unparentChildren(n);
      uc.deleteNode(n.id);
      if (selectedNodeId === n.id) selectedNodeId = null;
    }
    for (const e of edges) uc.deleteEdge(String(e.id));
  }

  let viewportTimer: ReturnType<typeof setTimeout> | undefined;
  function onMoveEnd() {
    clearTimeout(viewportTimer);
    viewportTimer = setTimeout(() => {
      uc.setMapViewport(mapId, JSON.stringify(getViewport()));
    }, 400);
  }

  // Al hacer pan/zoom, recalcular la posición en pantalla de la barra flotante.
  function onMove() {
    viewportTick++;
  }

  // Editar etiqueta de una arista (doble click o desde su menú). Un click simple
  // solo la selecciona (comportamiento por defecto de Svelte Flow).
  async function editEdgeLabel(id: string) {
    const edge = flowEdges.find((e) => e.id === id);
    const current = typeof edge?.label === "string" ? edge.label : "";
    const next = prompt("Etiqueta de la conexión (protocolo/puerto):", current);
    if (next === null) return;
    const label = next.trim() || null;
    await uc.setEdgeLabel(id, label);
    flowEdges = flowEdges.map((e) =>
      e.id === id ? { ...e, label: label ?? undefined } : e,
    );
  }

  // Doble click sobre una arista: Svelte Flow no emite evento propio, así que se
  // detecta en el contenedor buscando el grupo `.svelte-flow__edge` (tiene data-id).
  function onFlowDblClick(event: MouseEvent) {
    const el = (event.target as HTMLElement).closest?.(".svelte-flow__edge") as HTMLElement | null;
    const id = el?.dataset.id;
    if (id) editEdgeLabel(id);
  }

  async function setEdgeShape(id: string, shape: string) {
    await uc.setEdgeStyle(id, JSON.stringify({ shape }));
    flowEdges = flowEdges.map((e) =>
      e.id === id ? { ...e, data: { ...e.data, shape } } : e,
    );
  }

  async function deleteEdgeById(id: string) {
    await uc.deleteEdge(id);
    flowEdges = flowEdges.filter((e) => e.id !== id);
    if (edgeToolbar?.id === id) edgeToolbar = null;
  }

  // --- Drag & drop desde la paleta ---
  function onDragOver(e: DragEvent) {
    if (e.dataTransfer?.types.includes(DND_MIME)) {
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
    }
  }

  // Zona (top-level) que contiene un punto absoluto; la más pequeña si hay varias.
  function zoneAtPoint(x: number, y: number): FlowNode | undefined {
    return flowNodes
      .filter((n) => n.type === "zone")
      .filter((z) => {
        const w = z.width ?? 0;
        const h = z.height ?? 0;
        return x >= z.position.x && x <= z.position.x + w && y >= z.position.y && y <= z.position.y + h;
      })
      .sort((a, b) => (a.width ?? 0) * (a.height ?? 0) - (b.width ?? 0) * (b.height ?? 0))[0];
  }

  async function onDrop(e: DragEvent) {
    const kind = e.dataTransfer?.getData(DND_MIME) as NodeKind | undefined;
    if (!kind) return;
    e.preventDefault();
    const pos = screenToFlowPosition({ x: e.clientX, y: e.clientY });
    const be = await uc.createNode(mapId, kind, NODE_KIND_LABELS[kind], pos.x, pos.y);
    let flowNode = toFlowNode(be);

    // Si se suelta dentro de una zona, queda agrupado (posición relativa).
    const zone = kind !== "zone" ? zoneAtPoint(pos.x, pos.y) : undefined;
    if (zone) {
      const rel = { x: pos.x - zone.position.x, y: pos.y - zone.position.y };
      flowNode = { ...flowNode, parentId: zone.id, position: rel };
      await uc.setNodeParent(be.id, zone.id);
      await uc.setNodePosition(be.id, rel.x, rel.y);
    }
    flowNodes = sortParentsFirst([...flowNodes, flowNode]);
    selectedNodeId = be.id;
  }

  // --- Callbacks del panel de propiedades ---
  async function updateLabel(label: string) {
    if (!selectedNodeId) return;
    const id = selectedNodeId;
    await uc.renameNode(id, label);
    flowNodes = flowNodes.map((n) =>
      n.id === id ? { ...n, data: { ...n.data, label } } : n,
    );
  }

  async function updateProperties(properties: Record<string, string>) {
    if (!selectedNodeId) return;
    const id = selectedNodeId;
    await uc.setNodeProperties(id, properties);
    flowNodes = flowNodes.map((n) =>
      n.id === id ? { ...n, data: { ...n.data, properties } } : n,
    );
  }

  // Saca del grupo a los hijos de `parent`, convirtiendo su posición a absoluta
  // (si no, al perder el padre quedarían con coords relativas sin referencia).
  async function unparentChildren(parent: FlowNode) {
    const children = flowNodes.filter((n) => n.parentId === parent.id);
    for (const c of children) {
      const abs = { x: parent.position.x + c.position.x, y: parent.position.y + c.position.y };
      await uc.setNodeParent(c.id, null);
      await uc.setNodePosition(c.id, abs.x, abs.y);
    }
    if (children.length > 0) {
      flowNodes = flowNodes.map((n) =>
        n.parentId === parent.id
          ? {
              ...n,
              parentId: undefined,
              position: {
                x: parent.position.x + n.position.x,
                y: parent.position.y + n.position.y,
              },
            }
          : n,
      );
    }
  }

  async function deleteNodeById(id: string) {
    const node = flowNodes.find((n) => n.id === id);
    if (node) await unparentChildren(node);
    await uc.deleteNode(id);
    flowNodes = flowNodes.filter((n) => n.id !== id);
    flowEdges = flowEdges.filter((e) => e.source !== id && e.target !== id);
    if (selectedNodeId === id) selectedNodeId = null;
  }

  async function deleteSelected() {
    if (selectedNodeId) await deleteNodeById(selectedNodeId);
  }

  // --- Menú contextual ---
  function onNodeContextMenu({ node, event }: { node: FlowNode; event: MouseEvent }) {
    event.preventDefault();
    // Seleccionar también el nodo: así el panel de propiedades ya lo refleja.
    selectedNodeId = node.id;
    contextMenu = {
      x: event.clientX,
      y: event.clientY,
      nodeId: node.id,
      kind: node.data.kind as NodeKind,
    };
  }
</script>

<div class="editor">
  <NodePalette />

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="flow" ondragover={onDragOver} ondrop={onDrop} ondblclick={onFlowDblClick}>
    <SvelteFlow
      bind:nodes={flowNodes}
      bind:edges={flowEdges}
      {nodeTypes}
      {edgeTypes}
      snapGrid={[16, 16]}
      elevateNodesOnSelect={false}
      fitView
      onnodedragstop={onNodeDragStop}
      onconnect={onConnect}
      ondelete={onDelete}
      onmove={onMove}
      onmoveend={onMoveEnd}
      onnodeclick={({ node }) => (selectedNodeId = node.id)}
      onnodecontextmenu={onNodeContextMenu}
      onpaneclick={() => { selectedNodeId = null; contextMenu = null; edgeToolbar = null; }}
    >
      <Background gap={16} />
      <Controls showLock={false} />
    </SvelteFlow>
  </div>

  {#if selectedNode}
    <PropertiesPanel
      nodeId={selectedNode.id}
      kind={selectedNode.data.kind as NodeKind}
      label={selectedNode.data.label as string}
      properties={selectedNode.data.properties as Record<string, string>}
      onLabel={updateLabel}
      onProperties={updateProperties}
      onDeleteNode={deleteSelected}
      onClose={() => (selectedNodeId = null)}
    />
  {/if}
</div>

{#if contextMenu}
  <NodeContextMenu
    x={contextMenu.x}
    y={contextMenu.y}
    nodeId={contextMenu.nodeId}
    kind={contextMenu.kind}
    onDelete={() => deleteNodeById(contextMenu!.nodeId)}
    onClose={() => (contextMenu = null)}
  />
{/if}

{#if edgeToolbar && toolbarPos}
  <div class="edge-toolbar" style="left: {toolbarPos.x}px; top: {toolbarPos.y}px" role="toolbar" tabindex="-1">
    {#each EDGE_SHAPES as s (s.value)}
      <button
        class="tbtn"
        class:active={toolbarShape === s.value}
        title={s.title}
        onclick={() => setEdgeShape(edgeToolbar!.id, s.value)}
      >
        <svg viewBox="0 0 24 12" width="20" height="12">
          <path d={s.d} fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      </button>
    {/each}
    <span class="tdiv"></span>
    <button class="tbtn" title="Editar etiqueta" onclick={() => editEdgeLabel(edgeToolbar!.id)}>
      <Icon icon={icons.edit} size={15} />
    </button>
    <button class="tbtn danger" title="Eliminar conexión" onclick={() => deleteEdgeById(edgeToolbar!.id)}>
      <Icon icon={icons.delete} size={15} />
    </button>
  </div>
{/if}


<style>
  .editor {
    display: flex;
    height: 100%;
    min-height: 0;
  }
  .flow {
    flex: 1;
    min-width: 0;
    height: 100%;
  }
  /* Barra flotante de la arista seleccionada (overlay, no dentro del SVG). */
  .edge-toolbar {
    position: fixed;
    z-index: 30;
    transform: translate(-50%, calc(-100% - 10px));
    display: flex;
    align-items: center;
    gap: 0.1rem;
    padding: 0.15rem;
    background: var(--karto-color-bg);
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.45);
  }
  .edge-toolbar .tbtn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.7rem;
    height: 1.5rem;
    padding: 0;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--karto-color-text);
    cursor: pointer;
  }
  .edge-toolbar .tbtn:hover {
    background: var(--karto-color-surface);
  }
  .edge-toolbar .tbtn.active {
    color: var(--karto-color-accent);
    background: color-mix(in srgb, var(--karto-color-accent) 15%, transparent);
  }
  .edge-toolbar .tbtn.danger:hover {
    color: #f87171;
  }
  .edge-toolbar .tdiv {
    width: 1px;
    height: 1rem;
    margin: 0 0.15rem;
    background: var(--karto-color-border);
  }
  /* Tema oscuro para los controles/minimapa de Svelte Flow. */
  .flow :global(.svelte-flow) {
    background: transparent;
  }
  /* Tema oscuro para los controles (zoom, fit, lock): por defecto vienen con
     fondo blanco e iconos oscuros, que se ven en blanco sobre el lienzo. */
  .flow :global(.svelte-flow__controls) {
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    overflow: hidden;
  }
  .flow :global(.svelte-flow__controls-button) {
    background: var(--karto-color-surface);
    border-bottom: 1px solid var(--karto-color-border);
    color: var(--karto-color-text);
    fill: var(--karto-color-text);
  }
  .flow :global(.svelte-flow__controls-button:hover) {
    background: color-mix(in srgb, var(--karto-color-accent) 18%, var(--karto-color-surface));
  }
  .flow :global(.svelte-flow__controls-button svg) {
    fill: currentColor;
  }
  /* Arista seleccionada: se resalta en el color de acento. */
  .flow :global(.svelte-flow__edge.selected .svelte-flow__edge-path) {
    stroke: var(--karto-color-accent);
    stroke-width: 2;
  }
  /* Puntos de conexión (handles): ocultos por defecto para no ensuciar el
     lienzo. Aparecen al pasar el ratón sobre el nodo y mientras se arrastra una
     conexión (para poder soltar en cualquier nodo). Las líneas ya existentes se
     ven siempre; esto solo oculta el punto, no la arista. */
  .flow :global(.svelte-flow__handle) {
    opacity: 0;
    transition: opacity 0.12s ease;
  }
  .flow :global(.svelte-flow__node:hover .svelte-flow__handle),
  .flow :global(.svelte-flow__node.selected .svelte-flow__handle),
  .flow :global(.svelte-flow:has(.svelte-flow__handle.connectingfrom) .svelte-flow__handle) {
    opacity: 1;
  }
  /* Nodo con conexión: solo el handle realmente en uso (source si es origen,
     target si es destino). */
  .flow :global(.svelte-flow__node.src-conn .svelte-flow__handle.source),
  .flow :global(.svelte-flow__node.tgt-conn .svelte-flow__handle.target) {
    opacity: 1;
  }
  /* Etiqueta de la conexión: chip legible en tema oscuro (por defecto es clara). */
  .flow :global(.svelte-flow__edge-label) {
    padding: 0.05rem 0.4rem;
    border-radius: 4px;
    background: var(--karto-color-surface);
    border: 1px solid var(--karto-color-border);
    color: var(--karto-color-text);
    font-family: var(--karto-font-body);
    font-size: 0.7rem;
    line-height: 1.5;
  }
</style>
