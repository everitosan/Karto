<script lang="ts">
  // Agrupador visual: rectángulo de fondo redimensionable con etiqueta y color.
  // No es un equipo; agrupa nodos visualmente (VPC/subred/zona/región).
  import { NodeResizer, Handle, Position, type NodeProps } from "@xyflow/svelte";

  type ZoneData = {
    label: string;
    properties?: Record<string, string>;
    onResize?: (w: number, h: number) => void;
  };

  let { data, selected }: NodeProps & { data: ZoneData } = $props();

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
  <!-- Puntos de conexión (target entra por la izq., source sale por la der.),
       como los nodos normales. Sin id: así las aristas guardadas reconectan. -->
  <Handle type="target" position={Position.Left} />
  <Handle type="source" position={Position.Right} />
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
</style>
