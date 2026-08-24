// Estado reactivo del contexto de acceso (punto de vista de red) activo.
// El catálogo de contextos vive en el vault; el contexto *activo* es local a
// cada equipo (localStorage), para no errar al abrir el mismo vault desde otra
// red o al entrar/salir de una VPN. Al cambiar el activo, todo el diagrama pasa
// a mostrar y conectar por la dirección de ese contexto sin tocar nodo por nodo.
import type { AccessContext } from "$domain/infra";
import { workspaceUseCases } from "$usecases/workspace";

const STORAGE_KEY = "karto.activeContextId";

function readStored(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

function writeStored(id: string | null): void {
  try {
    if (id) localStorage.setItem(STORAGE_KEY, id);
    else localStorage.removeItem(STORAGE_KEY);
  } catch {
    // localStorage no disponible: el activo vive solo en memoria esta sesión.
  }
}

export const networkContext = $state<{
  contexts: AccessContext[];
  activeId: string | null;
}>({
  contexts: [],
  activeId: null,
});

/** Dirección efectiva de un nodo en el contexto activo (o respaldo hostname). */
export function resolveAddress(
  endpoints: Record<string, string>,
  properties: Record<string, string>,
): string | null {
  const id = networkContext.activeId;
  if (id && endpoints[id]) return endpoints[id];
  return properties.hostname ?? null;
}

/** Carga los contextos del vault y fija el activo (guardado o el primero). */
export async function loadContexts(): Promise<void> {
  const list = await workspaceUseCases.listContexts();
  networkContext.contexts = list;
  const stored = readStored();
  const exists = stored && list.some((c) => c.id === stored);
  networkContext.activeId = exists ? stored : (list[0]?.id ?? null);
}

export function setActiveContext(id: string): void {
  networkContext.activeId = id;
  writeStored(id);
}

export async function createContext(name: string): Promise<void> {
  const created = await workspaceUseCases.createContext(name);
  await loadContexts();
  setActiveContext(created.id);
}

export async function renameContext(id: string, name: string): Promise<void> {
  await workspaceUseCases.renameContext(id, name);
  await loadContexts();
}

export async function deleteContext(id: string): Promise<void> {
  await workspaceUseCases.deleteContext(id);
  // Si se borró el activo, loadContexts reelige uno válido.
  if (networkContext.activeId === id) writeStored(null);
  await loadContexts();
}
