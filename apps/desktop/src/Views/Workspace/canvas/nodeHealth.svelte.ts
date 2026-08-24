// Estado de salud por nodo (health check TCP), en memoria de la sesión. No viaja
// con el vault: es un chequeo puntual de red desde este equipo/contexto. El
// canvas dispara las comprobaciones y `InfraNode` pinta el punto de estado.
import type { HealthState } from "$domain/infra";
import { workspaceUseCases } from "$usecases/workspace";
import { networkContext } from "../networkContext.svelte";

// "checking" es transitorio mientras corre la sonda; el resto lo devuelve el backend.
export type NodeHealth = "checking" | HealthState;

export const nodeHealth = $state<Record<string, NodeHealth>>({});

/** Comprueba un nodo y guarda su estado (marca "checking" mientras tanto). */
export async function checkNode(nodeId: string): Promise<void> {
  nodeHealth[nodeId] = "checking";
  try {
    const res = await workspaceUseCases.checkHealth(nodeId, networkContext.activeId);
    nodeHealth[nodeId] = res.state;
  } catch {
    nodeHealth[nodeId] = "unreachable";
  }
}

/** Comprueba varios nodos en paralelo (p. ej. todos los del diagrama). */
export function checkNodes(nodeIds: string[]): Promise<void> {
  return Promise.all(nodeIds.map(checkNode)).then(() => {});
}
