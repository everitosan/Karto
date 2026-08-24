<script lang="ts">
  // Nodo custom del canvas: icono por tipo, etiqueta y un resumen (IP visible).
  // Componente de nodo de Svelte Flow; recibe `data` y `selected` como props.
  import { Handle, Position, type NodeProps } from "@xyflow/svelte";
  import { Icon, TechIcon } from "@karto/ui";
  import { resolveNodeIcon } from "@karto/catalog";
  import type { NodeKind } from "$domain/infra";
  import { resolveAddress } from "../networkContext.svelte";
  import { nodeHealth } from "./nodeHealth.svelte";

  type InfraData = {
    kind: NodeKind;
    label: string;
    properties: Record<string, string>;
    /** Dirección por contexto (`contextId` → dirección). */
    endpoints?: Record<string, string>;
    /** Lados (top/right/bottom/left) con alguna arista conectada. */
    activeHandles?: string[];
  };

  let { id, data, selected }: NodeProps & { data: InfraData } = $props();

  const isActive = (side: string) => data.activeHandles?.includes(side) ?? false;

  // Estado del health check: tiñe el icono del nodo. Gris por defecto (sin
  // comprobar); verde si responde o si al conectar se obtuvieron datos (prueba
  // indirecta); rojo/ámbar según el fallo.
  const health = $derived(nodeHealth[id]);
  const HEALTH_COLOR: Record<string, string> = {
    checking: "#94a3b8",
    reachable: "#22c55e",
    unreachable: "#ef4444",
    unresolved: "#f59e0b",
    noTarget: "#64748b",
  };
  const DEFAULT_COLOR = "#64748b"; // gris (aún sin comprobar)
  const iconColor = $derived(health ? (HEALTH_COLOR[health] ?? DEFAULT_COLOR) : DEFAULT_COLOR);
  const healthTitle: Record<string, string> = {
    checking: "Comprobando…",
    reachable: "Responde",
    unreachable: "No responde",
    unresolved: "No se resuelve el host",
    noTarget: "Sin dirección para comprobar",
  };

  const icon = $derived(resolveNodeIcon(data.kind, data.properties));

  // Resumen de la caja: la dirección del contexto activo (o hostname de respaldo),
  // y si no, otra propiedad representativa. Reacciona al contexto activo.
  const summary = $derived(
    resolveAddress(data.endpoints ?? {}, data.properties ?? {}) ??
      data.properties?.host ??
      data.properties?.url_admin ??
      null,
  );
</script>

<div class="node {data.kind}" class:selected>
  <!-- 4 puntos de conexión (uno por lado). Con ConnectionMode.Loose cada uno
       sirve tanto de origen como de destino: se puede unir cualquier lado con
       cualquier otro. El `id` permite que la arista recuerde por qué lado sale
       y entra al recargar. -->
  <Handle type="source" position={Position.Top} id="top" class={isActive("top") ? "active" : undefined} />
  <Handle type="source" position={Position.Right} id="right" class={isActive("right") ? "active" : undefined} />
  <Handle type="source" position={Position.Bottom} id="bottom" class={isActive("bottom") ? "active" : undefined} />
  <Handle type="source" position={Position.Left} id="left" class={isActive("left") ? "active" : undefined} />
  <div
    class="icon"
    class:brand={icon.type === "devicon"}
    class:checking={health === "checking"}
    style="--ic: {iconColor}"
    title={health ? healthTitle[health] : undefined}
  >
    {#if icon.type === "devicon"}
      <TechIcon name={icon.name} size={22} />
    {:else}
      <Icon icon={icon.icon} size={20} />
    {/if}
  </div>
  <div class="body">
    <span class="label">{data.label}</span>
    {#if summary}
      <span class="summary">{summary}</span>
    {/if}
  </div>
</div>

<style>
  .node {
    position: relative;
    display: flex;
    align-items: center;
    gap: 0.55rem;
    min-width: 9rem;
    max-width: 15rem;
    padding: 0.55rem 0.7rem;
    background: var(--karto-color-surface, #131a26);
    border: 1px solid var(--karto-color-border, #334155);
    border-radius: 10px;
    color: var(--karto-color-text, #e2e8f0);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.35);
  }
  .node.selected {
    border-color: var(--karto-color-accent, #11b245);
    box-shadow: 0 0 0 1px var(--karto-color-accent, #11b245);
  }
  .icon {
    display: grid;
    place-items: center;
    width: 2rem;
    height: 2rem;
    flex-shrink: 0;
    border-radius: 8px;
    /* `--ic` lo fija el nodo según el estado de salud (gris por defecto). */
    background: color-mix(in srgb, var(--ic, #64748b) 18%, transparent);
    color: var(--ic, #64748b);
    transition: background 0.25s ease, color 0.25s ease, box-shadow 0.25s ease;
  }
  /* Los logos de marca (Devicon) llevan su propio color: fondo neutro + aro de
     estado para que el color de salud siga siendo visible. */
  .icon.brand {
    background: color-mix(in srgb, #ffffff 8%, transparent);
    box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--ic, #64748b) 55%, transparent);
  }
  .icon.checking {
    animation: health-pulse 0.9s ease-in-out infinite;
  }
  @keyframes health-pulse {
    0%,
    100% {
      opacity: 0.5;
    }
    50% {
      opacity: 1;
    }
  }
  .body {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .label {
    font-weight: 600;
    font-size: 0.9rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .summary {
    font-size: 0.75rem;
    opacity: 0.65;
    font-family: var(--karto-font-body, monospace);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
