<script lang="ts">
  // Agrupador visual: rectángulo de fondo redimensionable con etiqueta y color.
  // No es un equipo; agrupa nodos visualmente (VPC/subred/zona/región).
  import { NodeResizer, Handle, Position, type NodeProps } from "@xyflow/svelte";

  type ZoneData = {
    label: string;
    properties?: Record<string, string>;
    onResize?: (w: number, h: number) => void;
    /** Lados (top/right/bottom/left) con alguna arista conectada. */
    activeHandles?: string[];
  };

  let { data, selected }: NodeProps & { data: ZoneData } = $props();

  const isActive = (side: string) => data.activeHandles?.includes(side) ?? false;

  const COLORS: Record<string, string> = {
    slate: "#64748b",
    green: "#22c55e",
    blue: "#3b82f6",
    amber: "#f59e0b",
    rose: "#f43f5e",
    violet: "#8b5cf6",
  };
  const hex = $derived(COLORS[data.properties?.color ?? "slate"] ?? COLORS.slate);
</script>

<NodeResizer
  isVisible={selected}
  minWidth={140}
  minHeight={90}
  onResizeEnd={(_, p) => data.onResize?.(p.width, p.height)}
/>

<div class="zone" class:selected style="--zc: {hex};">
  <span class="zone-label">{data.label}</span>
  <!-- 4 puntos de conexión con id (como los nodos): con ConnectionMode.Loose
       cada lado sirve de origen y destino, y el id deja que la arista recuerde
       el lado al recargar y marque el indicador de conexión activa. -->
  <Handle type="source" position={Position.Top} id="top" class={isActive("top") ? "active" : undefined} />
  <Handle type="source" position={Position.Right} id="right" class={isActive("right") ? "active" : undefined} />
  <Handle type="source" position={Position.Bottom} id="bottom" class={isActive("bottom") ? "active" : undefined} />
  <Handle type="source" position={Position.Left} id="left" class={isActive("left") ? "active" : undefined} />
</div>

<style>
  .zone {
    position: relative;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    border: 1.5px dashed color-mix(in srgb, var(--zc) 55%, transparent);
    border-radius: 12px;
    background: color-mix(in srgb, var(--zc) 8%, transparent);
  }
  .zone.selected {
    border-style: solid;
  }
  .zone-label {
    position: absolute;
    top: 6px;
    left: 10px;
    font-size: 0.8rem;
    font-weight: 600;
    color: color-mix(in srgb, var(--zc) 65%, #ffffff);
    background: var(--karto-color-bg, #090d15);
    padding: 1px 6px;
    border-radius: 6px;
    pointer-events: none;
    white-space: nowrap;
  }

  /* Handles de redimensionado de la zona: cuadrados blancos con borde, más
     grandes que los puntos de conexión (redondos, en el centro de cada lado),
     para que quede claro que son "agarres" de resize y no puntos de conexión. */
  :global(.svelte-flow__node-zone .svelte-flow__resize-control.handle) {
    width: 7px;
    height: 7px;
    border-radius: 2px;
    background: #ffffff;
    border: 1.25px solid var(--karto-color-border, #334155);
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.4);
  }
  /* Líneas de borde para redimensionar: apenas visibles, solo como pista. */
  :global(.svelte-flow__node-zone .svelte-flow__resize-control.line) {
    border-color: transparent;
  }
</style>
