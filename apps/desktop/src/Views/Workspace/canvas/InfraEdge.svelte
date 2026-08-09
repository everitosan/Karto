<script lang="ts">
  // Arista personalizada: dibuja la línea según su `data.shape` y delega la
  // etiqueta en BaseEdge (que usa la capa de labels, no foreignObject). Cuando
  // está seleccionada, reporta su punto medio (coords de flujo) al editor, que
  // dibuja la barra flotante como overlay HTML — WebKitGTK no transforma bien el
  // contenido de un <foreignObject> dentro del SVG con zoom/pan.
  import {
    BaseEdge,
    getBezierPath,
    getSmoothStepPath,
    getStraightPath,
    type Position,
  } from "@xyflow/svelte";

  interface EdgeData {
    shape?: string;
    onGeom?: (fx: number, fy: number) => void;
    onDeselect?: () => void;
  }

  interface Props {
    sourceX: number;
    sourceY: number;
    targetX: number;
    targetY: number;
    sourcePosition: Position;
    targetPosition: Position;
    label?: string;
    selected?: boolean;
    markerEnd?: string;
    data?: EdgeData;
  }

  let {
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
    label,
    selected = false,
    markerEnd,
    data,
  }: Props = $props();

  const shape = $derived(data?.shape ?? "default");

  const geom = $derived.by(() => {
    const p = { sourceX, sourceY, sourcePosition, targetX, targetY, targetPosition };
    switch (shape) {
      case "straight":
        return getStraightPath({ sourceX, sourceY, targetX, targetY });
      case "smoothstep":
        return getSmoothStepPath(p);
      case "step":
        return getSmoothStepPath({ ...p, borderRadius: 0 });
      default:
        return getBezierPath(p);
    }
  });
  const path = $derived(geom[0]);
  const labelX = $derived(geom[1]);
  const labelY = $derived(geom[2]);

  // Reporta el punto medio al editor mientras está seleccionada (para la barra).
  $effect(() => {
    if (selected) data?.onGeom?.(labelX, labelY);
    else data?.onDeselect?.();
  });
</script>

<BaseEdge {path} {markerEnd} {label} {labelX} {labelY} />
