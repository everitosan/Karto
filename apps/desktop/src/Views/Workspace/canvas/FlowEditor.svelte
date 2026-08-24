<script lang="ts">
  // Editor del canvas para un diagrama. Vive dentro de <SvelteFlowProvider>
  // (Canvas) para poder usar useSvelteFlow (screenToFlowPosition, viewport).
  // Orquesta la carga del grafo y el autoguardado vía comandos Tauri.
  import { onMount, tick, untrack } from "svelte";
  import {
    SvelteFlow,
    Background,
    Controls,
    ConnectionMode,
    SelectionMode,
    useSvelteFlow,
    getNodesBounds,
    getViewportForBounds,
    Panel,
    type Node as FlowNode,
    type Edge as FlowEdge,
    type NodeTypes,
    type EdgeTypes,
    type Connection,
  } from "@xyflow/svelte";
  import "@xyflow/svelte/dist/style.css";
  import { toPng } from "html-to-image";
  import { pickExportImagePath, pickSubsetExportPath } from "$usecases/dialog";
  import { vaultUseCases } from "$usecases/vault";
  import { Icon, icons } from "@karto/ui";
  import SubsetExportModal from "./SubsetExportModal.svelte";
  import type { Graph, InfraEdge, InfraNode, NodeKind } from "$domain/infra";
  import { nodeKindLabel } from "$i18n/catalog";
  import { m } from "$paraglide/messages.js";
  import { workspaceUseCases as uc } from "$usecases/workspace";
  import InfraNodeView from "./InfraNode.svelte";
  import ZoneNodeView from "./ZoneNode.svelte";
  import NodePalette from "./NodePalette.svelte";
  import PropertiesPanel from "./PropertiesPanel.svelte";
  import NodeContextMenu from "./NodeContextMenu.svelte";
  import InfraEdgeView from "./InfraEdge.svelte";
  import { DND_MIME } from "./dnd";
  import { peekFocus, clearFocus } from "../focusNode.svelte";
  import { onFactsCollected } from "./connectFlow.svelte";
  import { checkNodes, nodeHealth } from "./nodeHealth.svelte";

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
  let contextMenu = $state<{ x: number; y: number; nodeId: string; kind: NodeKind; canProbe: boolean } | null>(null);
  // Barra flotante de la arista seleccionada: punto medio en coords de flujo.
  let edgeToolbar = $state<{ id: string; fx: number; fy: number } | null>(null);
  // Se incrementa al mover/zoom para recalcular la posición en pantalla del overlay.
  let viewportTick = $state(0);

  const {
    screenToFlowPosition,
    getViewport,
    setViewport,
    setCenter,
    flowToScreenPosition,
    getIntersectingNodes,
  } = useSvelteFlow();

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
    { value: "default", title: m.flow_edge_default(), d: "M2 10 C8 10 16 2 22 2" },
    { value: "smoothstep", title: m.flow_edge_smoothstep(), d: "M2 10 H9 Q12 10 12 7 V5 Q12 2 15 2 H22" },
    { value: "step", title: m.flow_edge_step(), d: "M2 10 H12 V2 H22" },
    { value: "straight", title: m.flow_edge_straight(), d: "M2 6 H22" },
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
      data: { kind: n.kind, label: n.label, properties: n.properties, endpoints: n.endpoints },
    };
  };

  // Svelte Flow exige que un nodo padre aparezca ANTES que sus hijos en el array.
  // Solo las zonas son padres (sin padre propio), así que basta ponerlas primero.
  function sortParentsFirst(nodes: FlowNode[]): FlowNode[] {
    return [...nodes].sort((a, b) => (a.parentId ? 1 : 0) - (b.parentId ? 1 : 0));
  }

  // Handles en uso por nodo: qué lados (top/right/bottom/left) tienen alguna
  // arista conectada, para dejar visible ese punto en reposo (indicador de que
  // el nodo tiene una conexión activa). Se deriva de los handles guardados en
  // cada arista y se propaga a `data.activeHandles` del nodo.
  const handleUse = $derived.by(() => {
    const map = new Map<string, Set<string>>();
    const add = (nodeId: string, handle?: string | null) => {
      if (!handle) return;
      (map.get(nodeId) ?? map.set(nodeId, new Set()).get(nodeId)!).add(handle);
    };
    for (const e of flowEdges) {
      add(e.source, e.sourceHandle);
      add(e.target, e.targetHandle);
    }
    return map;
  });
  $effect(() => {
    const use = handleUse;
    const current = untrack(() => flowNodes);
    let changed = false;
    const next = current.map((n) => {
      if (n.type !== "infra" && n.type !== "zone") return n;
      const active = Array.from(use.get(n.id) ?? []).sort();
      const prev = ((n.data?.activeHandles as string[]) ?? []).join(",");
      if (prev === active.join(",")) return n;
      changed = true;
      return { ...n, data: { ...n.data, activeHandles: active } };
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
    // Al agrandar la zona puede abarcar nodos que antes quedaban fuera: captúralos.
    const zone = flowNodes.find((n) => n.id === id);
    if (zone) await captureNodesInZone(zone);
  }

  // Estado de la arista guardado en su `style` JSON: forma de la línea y los
  // handles (lados) por los que sale/entra, para reconstruir el trazado al
  // recargar. `sh`/`th` = source/target handle.
  type EdgeStyle = { shape: string; sh?: string; th?: string };
  function parseStyle(style: string): EdgeStyle {
    try {
      const s = JSON.parse(style || "{}");
      return {
        shape: typeof s.shape === "string" ? s.shape : "default",
        sh: typeof s.sh === "string" ? s.sh : undefined,
        th: typeof s.th === "string" ? s.th : undefined,
      };
    } catch {
      return { shape: "default" };
    }
  }
  const serializeStyle = (s: EdgeStyle): string =>
    JSON.stringify({ shape: s.shape, sh: s.sh, th: s.th });

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

  const toFlowEdge = (e: InfraEdge): FlowEdge => {
    const s = parseStyle(e.style);
    return {
      id: e.id,
      source: e.sourceId,
      target: e.targetId,
      sourceHandle: s.sh,
      targetHandle: s.th,
      label: e.label ?? undefined,
      type: "infra",
      data: edgeData(e.id, s.shape),
    };
  };

  // Viewport guardado del mapa (JSON `{x,y,zoom}`). Devuelve null si está vacío
  // (`{}` por defecto) o mal formado → el caller cae a `fitView`.
  function parseViewport(raw: string | undefined): { x: number; y: number; zoom: number } | null {
    if (!raw) return null;
    try {
      const v = JSON.parse(raw);
      if (typeof v?.x === "number" && typeof v?.y === "number" && typeof v?.zoom === "number") {
        return { x: v.x, y: v.y, zoom: v.zoom };
      }
    } catch {
      // JSON inválido → fitView.
    }
    return null;
  }

  onMount(async () => {
    const graph: Graph = await uc.loadGraph(mapId);
    flowNodes = sortParentsFirst(graph.nodes.map(toFlowNode));
    flowEdges = graph.edges.map(toFlowEdge);
    // Si ya hay un foco pendiente para este mapa (llegada desde la búsqueda), el
    // $effect de abajo lo atenderá; solo restauramos viewport si no lo hay.
    if (peekFocus()?.mapId === mapId) return;
    // Restaurar el viewport guardado (por-mapa, portable con el vault). Si no hay
    // uno válido, se mantiene el `fitView` inicial de <SvelteFlow>.
    const saved = parseViewport(flowNodes.length ? await loadSavedViewport() : undefined);
    if (saved) {
      await tick();
      setViewport(saved);
    }
  });

  // Atiende peticiones de enfoque de la búsqueda global: cuando hay un foco para
  // este mapa y el nodo ya está cargado, lo selecciona y centra. Reactivo, así
  // cubre tanto el remonte por cambio de diagrama como el diagrama ya abierto.
  $effect(() => {
    const f = peekFocus();
    if (f && f.mapId === mapId && flowNodes.some((n) => n.id === f.nodeId)) {
      clearFocus();
      const id = f.nodeId;
      tick().then(() => focusNodeById(id));
    }
  });

  // Al conectar por SSH, el sondeo del equipo llega de forma asíncrona: parchea
  // las propiedades del nodo en vivo (el panel las refleja al leerlas del nodo).
  onMount(() => {
    onFactsCollected((nodeId, facts) => {
      // Obtener datos por SSH prueba que el equipo respondió → márcalo alcanzable.
      nodeHealth[nodeId] = "reachable";
      flowNodes = flowNodes.map((n) =>
        n.id === nodeId
          ? {
              ...n,
              data: {
                ...n.data,
                properties: { ...(n.data.properties as Record<string, string>), ...facts },
              },
            }
          : n,
      );
    });
    return () => onFactsCollected(null);
  });

  // Selecciona y centra un nodo (usado al llegar desde la búsqueda global).
  function focusNodeById(id: string) {
    const node = flowNodes.find((n) => n.id === id);
    if (!node) return;
    selectedNodeId = id;
    flowNodes = flowNodes.map((n) => ({ ...n, selected: n.id === id }));
    const abs = absolutePos(node);
    // El nodo se posiciona por su esquina; centrar en su punto medio aproximado.
    const cx = abs.x + (node.width ?? 150) / 2;
    const cy = abs.y + (node.height ?? 60) / 2;
    setCenter(cx, cy, { zoom: Math.max(getViewport().zoom, 1), duration: 300 });
  }

  // Lee el viewport persistido del mapa actual desde la lista de diagramas.
  async function loadSavedViewport(): Promise<string | undefined> {
    const maps = await uc.listMaps();
    return maps.find((m) => m.id === mapId)?.viewport;
  }

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
    // reemplazamos su id por el del backend. Guardamos también los handles
    // (lados) usados en el `style` para reconstruir el trazado al recargar.
    const be = await uc.createEdge(mapId, conn.source, conn.target);
    const sh = conn.sourceHandle ?? undefined;
    const th = conn.targetHandle ?? undefined;
    await uc.setEdgeStyle(be.id, serializeStyle({ shape: "default", sh, th }));
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
    const next = prompt(m.flow_edge_label_prompt(), current);
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
    // Conserva los handles (sh/th) al reescribir el estilo, solo cambia la forma.
    const edge = flowEdges.find((e) => e.id === id);
    await uc.setEdgeStyle(
      id,
      serializeStyle({
        shape,
        sh: edge?.sourceHandle ?? undefined,
        th: edge?.targetHandle ?? undefined,
      }),
    );
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

  // Al soltar una zona sobre nodos ya existentes, los captura como hijos (mismo
  // resultado que si primero estuviera la zona). Se toma el centro de cada nodo
  // top-level: si cae dentro del rectángulo de la zona, se reparenta.
  async function captureNodesInZone(zone: FlowNode) {
    const zx = zone.position.x;
    const zy = zone.position.y;
    const zw = zone.width ?? ZONE_DEFAULT.w;
    const zh = zone.height ?? ZONE_DEFAULT.h;
    const captured: { id: string; rel: { x: number; y: number } }[] = [];
    const next = flowNodes.map((n) => {
      if (n.type !== "infra" || n.parentId) return n;
      const cx = n.position.x + (n.width ?? 0) / 2;
      const cy = n.position.y + (n.height ?? 0) / 2;
      if (cx < zx || cx > zx + zw || cy < zy || cy > zy + zh) return n;
      const rel = { x: n.position.x - zx, y: n.position.y - zy };
      captured.push({ id: n.id, rel });
      return { ...n, parentId: zone.id, position: rel };
    });
    if (captured.length === 0) return;
    flowNodes = sortParentsFirst(next);
    for (const c of captured) {
      await uc.setNodeParent(c.id, zone.id);
      await uc.setNodePosition(c.id, c.rel.x, c.rel.y);
    }
  }

  async function onDrop(e: DragEvent) {
    const kind = e.dataTransfer?.getData(DND_MIME) as NodeKind | undefined;
    if (!kind) return;
    e.preventDefault();
    const pos = screenToFlowPosition({ x: e.clientX, y: e.clientY });
    const be = await uc.createNode(mapId, kind, nodeKindLabel(kind), pos.x, pos.y);
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
    // Zona nueva: captura los nodos existentes que queden dentro.
    if (kind === "zone") await captureNodesInZone(flowNode);
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

  async function updateEndpoints(endpoints: Record<string, string>) {
    if (!selectedNodeId) return;
    const id = selectedNodeId;
    await uc.setNodeEndpoints(id, endpoints);
    flowNodes = flowNodes.map((n) =>
      n.id === id ? { ...n, data: { ...n.data, endpoints } } : n,
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

  // --- Export del diagrama (PNG/SVG) ---
  let flowEl = $state<HTMLDivElement | null>(null);
  let exportMenuOpen = $state(false);
  let exporting = $state(false);
  // Export selectivo: nodos marcados como seleccionados en el lienzo.
  let subsetOpen = $state(false);
  const selectedNodeIds = $derived(flowNodes.filter((n) => n.selected).map((n) => n.id));

  // --- Health check ---
  let checkingHealth = $state(false);
  async function checkHealth() {
    // Comprueba todos los nodos con dirección (las zonas no conectan).
    const ids = flowNodes.filter((n) => n.type !== "zone").map((n) => n.id);
    if (ids.length === 0) return;
    checkingHealth = true;
    try {
      await checkNodes(ids);
    } finally {
      checkingHealth = false;
    }
  }

  const EXPORT_BG = "#0b0f17";

  // Lee los trazos de las aristas del DOM (coords absolutas de flujo). Las
  // dibujamos nosotros porque WebKitGTK (Tauri/Linux) no rasteriza el SVG de
  // las aristas dentro del <foreignObject> que genera html-to-image: los nodos
  // (HTML) salen, las líneas (SVG) no. Así el export es fiable en Linux.
  function collectEdgePaths(): { d: string; stroke: string; width: number }[] {
    if (!flowEl) return [];
    const paths = flowEl.querySelectorAll<SVGPathElement>(".svelte-flow__edge-path");
    return Array.from(paths)
      .map((p) => {
        const cs = getComputedStyle(p);
        const stroke = cs.stroke && cs.stroke !== "none" ? cs.stroke : "#8a8f98";
        return { d: p.getAttribute("d") ?? "", stroke, width: parseFloat(cs.strokeWidth) || 1 };
      })
      .filter((e) => e.d);
  }

  interface Frame {
    width: number;
    height: number;
    vp: { x: number; y: number; zoom: number };
  }

  // Rasteriza SOLO los nodos (capa HTML, fondo transparente) con el encuadre
  // dado; las aristas se componen aparte.
  async function renderNodesLayer(el: HTMLElement, f: Frame): Promise<string> {
    return toPng(el, {
      width: f.width,
      height: f.height,
      style: {
        width: `${f.width}px`,
        height: `${f.height}px`,
        transform: `translate(${f.vp.x}px, ${f.vp.y}px) scale(${f.vp.zoom})`,
      },
    });
  }

  // PNG: lienzo con fondo + aristas (Path2D) + imagen de nodos encima.
  async function composePng(nodesUrl: string, f: Frame): Promise<Blob> {
    const canvas = document.createElement("canvas");
    canvas.width = f.width;
    canvas.height = f.height;
    const ctx = canvas.getContext("2d")!;
    ctx.fillStyle = EXPORT_BG;
    ctx.fillRect(0, 0, f.width, f.height);
    ctx.save();
    ctx.translate(f.vp.x, f.vp.y);
    ctx.scale(f.vp.zoom, f.vp.zoom);
    for (const e of collectEdgePaths()) {
      ctx.strokeStyle = e.stroke;
      ctx.lineWidth = e.width;
      ctx.stroke(new Path2D(e.d));
    }
    ctx.restore();
    const img = new Image();
    img.src = nodesUrl;
    await img.decode();
    ctx.drawImage(img, 0, 0, f.width, f.height);
    return await new Promise<Blob>((resolve, reject) =>
      canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("toBlob nulo"))), "image/png"),
    );
  }

  // SVG: aristas vectoriales + capa de nodos embebida como imagen PNG.
  function composeSvg(nodesUrl: string, f: Frame): string {
    const edges = collectEdgePaths()
      .map((e) => `<path d="${e.d}" fill="none" stroke="${e.stroke}" stroke-width="${e.width}"/>`)
      .join("");
    return (
      `<svg xmlns="http://www.w3.org/2000/svg" width="${f.width}" height="${f.height}" ` +
      `viewBox="0 0 ${f.width} ${f.height}">` +
      `<rect width="100%" height="100%" fill="${EXPORT_BG}"/>` +
      `<g transform="translate(${f.vp.x} ${f.vp.y}) scale(${f.vp.zoom})">${edges}</g>` +
      `<image href="${nodesUrl}" x="0" y="0" width="${f.width}" height="${f.height}"/>` +
      `</svg>`
    );
  }

  async function exportImage(format: "png" | "svg") {
    exportMenuOpen = false;
    if (flowNodes.length === 0 || !flowEl) return;
    const maps = await uc.listMaps();
    const name = maps.find((m) => m.id === mapId)?.name ?? "diagrama";
    const path = await pickExportImagePath(format, name);
    if (!path) return;

    const viewportEl = flowEl.querySelector<HTMLElement>(".svelte-flow__viewport");
    if (!viewportEl) return;

    exporting = true;
    try {
      // Encaja todos los nodos en un lienzo del tamaño de su caja + margen.
      const bounds = getNodesBounds(flowNodes);
      const margin = 80;
      const width = Math.min(Math.ceil(bounds.width) + margin * 2, 4096);
      const height = Math.min(Math.ceil(bounds.height) + margin * 2, 4096);
      const vp = getViewportForBounds(bounds, width, height, 0.2, 2, 0.1);
      const frame: Frame = { width, height, vp };

      const nodesUrl = await renderNodesLayer(viewportEl, frame);
      let bytes: number[];
      if (format === "png") {
        const blob = await composePng(nodesUrl, frame);
        bytes = Array.from(new Uint8Array(await blob.arrayBuffer()));
      } else {
        bytes = Array.from(new TextEncoder().encode(composeSvg(nodesUrl, frame)));
      }
      await uc.exportWrite(path, bytes);
    } catch (e) {
      console.error("export falló", e);
      alert(e instanceof Error ? e.message : m.flow_export_error());
    } finally {
      exporting = false;
    }
  }

  // --- Export selectivo (nodos seleccionados → .karto nuevo cifrado) ---
  async function doSubsetExport(opts: {
    password: string;
    includeCredentials: boolean;
    includeFacts: boolean;
    includeIp: boolean;
    includeNotes: boolean;
  }) {
    const ids = selectedNodeIds;
    if (ids.length === 0) return;
    const maps = await uc.listMaps();
    const name = maps.find((m) => m.id === mapId)?.name ?? "seleccion";
    const dest = await pickSubsetExportPath(`${name}-seleccion`);
    if (!dest) return; // canceló el diálogo: el modal sigue abierto para reintentar
    await vaultUseCases.exportSubset({ dest, nodeIds: ids, mapName: name, ...opts });
    subsetOpen = false;
  }

  // --- Menú contextual ---
  // ¿El nodo tiene alguna forma de ser sondeado? Hostname, URL (apps web) o algún
  // endpoint por contexto. Si no, "Comprobar estado" daría noTarget y se oculta.
  function canProbeNode(node: FlowNode): boolean {
    const props = (node.data.properties as Record<string, string>) ?? {};
    const endpoints = (node.data.endpoints as Record<string, string>) ?? {};
    const has = (v?: string) => !!v?.trim();
    return (
      has(props.hostname) ||
      has(props.url_admin) ||
      has(props.url) ||
      Object.values(endpoints).some(has)
    );
  }

  function onNodeContextMenu({ node, event }: { node: FlowNode; event: MouseEvent }) {
    event.preventDefault();
    // Seleccionar también el nodo: así el panel de propiedades ya lo refleja.
    selectedNodeId = node.id;
    contextMenu = {
      x: event.clientX,
      y: event.clientY,
      nodeId: node.id,
      kind: node.data.kind as NodeKind,
      canProbe: canProbeNode(node),
    };
  }
</script>

<div class="editor">
  <NodePalette />

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="flow" bind:this={flowEl} ondragover={onDragOver} ondrop={onDrop} ondblclick={onFlowDblClick}>
    <SvelteFlow
      bind:nodes={flowNodes}
      bind:edges={flowEdges}
      {nodeTypes}
      {edgeTypes}
      connectionMode={ConnectionMode.Loose}
      snapGrid={[16, 16]}
      deleteKey={["Delete", "Backspace"]}
      selectionOnDrag
      panOnDrag={false}
      panActivationKey=" "
      panOnScroll
      selectionMode={SelectionMode.Partial}
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
      <Panel position="top-right">
        <div class="export">
          <button
            class="export-btn"
            title={m.flow_health_title()}
            disabled={checkingHealth}
            onclick={checkHealth}
          >
            <Icon icon={icons.connect} size={15} />
            {checkingHealth ? m.flow_health_checking() : m.flow_health_label()}
          </button>
          <button
            class="export-btn"
            title={m.flow_export_subset_title()}
            disabled={selectedNodeIds.length === 0}
            onclick={() => (subsetOpen = true)}
          >
            <Icon icon={icons.folder} size={15} />
            {m.flow_export_selection()}{selectedNodeIds.length > 0 ? ` (${selectedNodeIds.length})` : ""}
          </button>
          <button
            class="export-btn"
            title={m.flow_export_title()}
            disabled={exporting || flowNodes.length === 0}
            onclick={() => (exportMenuOpen = !exportMenuOpen)}
          >
            <Icon icon={icons.diagram} size={15} />
            {exporting ? m.flow_exporting() : m.flow_export_label()}
          </button>
          {#if exportMenuOpen}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="export-backdrop" onclick={() => (exportMenuOpen = false)}></div>
            <div class="export-menu" role="menu">
              <button class="export-item" role="menuitem" onclick={() => exportImage("png")}>
                {m.flow_export_png()}
              </button>
              <button class="export-item" role="menuitem" onclick={() => exportImage("svg")}>
                {m.flow_export_svg()}
              </button>
            </div>
          {/if}
        </div>
      </Panel>
    </SvelteFlow>
  </div>

  {#if selectedNode}
    <PropertiesPanel
      nodeId={selectedNode.id}
      kind={selectedNode.data.kind as NodeKind}
      label={selectedNode.data.label as string}
      properties={selectedNode.data.properties as Record<string, string>}
      endpoints={(selectedNode.data.endpoints as Record<string, string>) ?? {}}
      onLabel={updateLabel}
      onProperties={updateProperties}
      onEndpoints={updateEndpoints}
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
    canProbe={contextMenu.canProbe}
    onDelete={() => deleteNodeById(contextMenu!.nodeId)}
    onClose={() => (contextMenu = null)}
  />
{/if}

<SubsetExportModal
  open={subsetOpen}
  nodeCount={selectedNodeIds.length}
  onClose={() => (subsetOpen = false)}
  onConfirm={doSubsetExport}
/>

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
    <button class="tbtn" title={m.flow_edit_label()} onclick={() => editEdgeLabel(edgeToolbar!.id)}>
      <Icon icon={icons.edit} size={15} />
    </button>
    <button class="tbtn danger" title={m.flow_delete_edge()} onclick={() => deleteEdgeById(edgeToolbar!.id)}>
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
  /* Botón de export en un Panel de Svelte Flow (esquina superior derecha). */
  .export {
    position: relative;
    display: flex;
    gap: 0.4rem;
  }
  .export-btn {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.35rem 0.6rem;
    background: var(--karto-color-bg);
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    color: var(--karto-color-text);
    font-family: var(--karto-font-body);
    font-size: 0.8rem;
    cursor: pointer;
  }
  .export-btn:hover:not(:disabled) {
    background: var(--karto-color-surface);
  }
  .export-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .export-backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
  }
  .export-menu {
    position: absolute;
    right: 0;
    top: calc(100% + 0.25rem);
    z-index: 41;
    min-width: 10rem;
    padding: 0.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    background: var(--karto-color-bg);
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    box-shadow: 0 8px 28px rgba(0, 0, 0, 0.45);
  }
  .export-item {
    padding: 0.4rem 0.5rem;
    border: 0;
    border-radius: var(--karto-radius);
    background: transparent;
    color: var(--karto-color-text);
    font-family: var(--karto-font-body);
    font-size: 0.82rem;
    text-align: left;
    cursor: pointer;
  }
  .export-item:hover {
    background: var(--karto-color-surface);
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
    /* WebKitGTK: aislar la animación de opacidad en su propia capa evita que al
       aparecer sobre el borde/sombra del nodo se repinte todo y parpadee. No se
       toca `transform` (lo usa Svelte Flow para posicionar el handle). */
    will-change: opacity;
    -webkit-backface-visibility: hidden;
    backface-visibility: hidden;
    /* Verde apagado (acento mezclado con gris): distingue el punto de conexión
       "disponible" del verde pleno que toma al quedar conectado (.active). */
    background: color-mix(in srgb, var(--karto-color-accent) 55%, #64748b);
    border-color: color-mix(in srgb, var(--karto-color-accent) 55%, #64748b);
  }
  .flow :global(.svelte-flow__node:hover .svelte-flow__handle),
  .flow :global(.svelte-flow__node.selected .svelte-flow__handle),
  .flow :global(.svelte-flow:has(.svelte-flow__handle.connectingfrom) .svelte-flow__handle) {
    opacity: 1;
  }
  /* Handle con una conexión activa: queda visible en reposo como indicador de
     que ese lado del nodo tiene una línea a otro nodo. */
  .flow :global(.svelte-flow__handle.active) {
    opacity: 1;
    background: var(--karto-color-accent);
    border-color: var(--karto-color-accent);
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
