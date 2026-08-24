// Señal compartida para enfocar un nodo desde la búsqueda global. La búsqueda
// vive en el panel de diagramas (que controla el mapa activo); al elegir un
// resultado se cambia de diagrama y se deja aquí el nodo a enfocar. El
// `FlowEditor` del mapa destino la observa (vía $effect) y, cuando el nodo ya
// está cargado, lo selecciona y centra; luego limpia la señal. Reactivo para
// cubrir también el caso de que el nodo esté en el diagrama ya abierto (sin
// remonte del canvas).

// Caja mutable module-level: `value` es $state, así que leerlo en un $effect lo
// hace reactivo.
const box = $state<{ value: { mapId: string; nodeId: string } | null }>({ value: null });

/** Pide enfocar un nodo en su diagrama (lo observará el FlowEditor destino). */
export function requestFocus(mapId: string, nodeId: string): void {
  box.value = { mapId, nodeId };
}

/** Lectura reactiva del foco pendiente (o null). */
export function peekFocus(): { mapId: string; nodeId: string } | null {
  return box.value;
}

/** Limpia el foco pendiente una vez atendido. */
export function clearFocus(): void {
  box.value = null;
}
