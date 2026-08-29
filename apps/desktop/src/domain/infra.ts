// Entidades del dominio de infraestructura (carpetas, diagramas, nodos,
// aristas, credenciales). Los nombres de campo casan con la serialización
// `camelCase` del backend Rust.
//
// El catálogo de tipos de nodo (categorías, propiedades e iconos) es dato
// agnóstico de idioma en `@karto/catalog`; aquí solo se re-exporta la estructura.
// Las *etiquetas* visibles (categoría/tipo) se traducen en `$i18n/catalog`, no en
// el dominio ni en el paquete compartido.
export type { NodeKind, NodeCategory } from "@karto/catalog";
export {
  NODE_KINDS,
  NODE_CATALOG,
  NODE_CATEGORIES,
  resolveNodeIcon,
} from "@karto/catalog";

import type { NodeKind } from "@karto/catalog";

export type CredentialKind = "ssh" | "rdp" | "vnc" | "web" | "db";

export interface Folder {
  id: string;
  parentId: string | null;
  name: string;
  color: string | null;
  position: number;
}

export interface InfraMap {
  id: string;
  folderId: string | null;
  name: string;
  /** Viewport de Svelte Flow serializado en JSON (`{x,y,zoom}`). */
  viewport: string;
  position: number;
}

export interface InfraNode {
  id: string;
  mapId: string;
  kind: NodeKind;
  label: string;
  x: number;
  y: number;
  /** Zona contenedora. `x`/`y` son relativos al padre si está presente. */
  parentId: string | null;
  properties: Record<string, string>;
  /**
   * Dirección del nodo por contexto de acceso (`contextId` → dirección). La
   * dirección efectiva depende del contexto activo; el `hostname` de
   * `properties` es el respaldo estable si no hay endpoint para ese contexto.
   */
  endpoints: Record<string, string>;
}

/**
 * Contexto de acceso (punto de vista de red): "Oficina", "VPN", "Público"…
 * Selecciona qué dirección de cada nodo se usa al conectar. El catálogo vive en
 * el vault; el contexto *activo* es estado local de cada equipo.
 */
export interface AccessContext {
  id: string;
  name: string;
  position: number;
}

export interface InfraEdge {
  id: string;
  mapId: string;
  sourceId: string;
  targetId: string;
  label: string | null;
  style: string;
}

export interface Graph {
  nodes: InfraNode[];
  edges: InfraEdge[];
}

/** Estado de salud de un nodo tras la sonda TCP. */
export type HealthState = "reachable" | "unreachable" | "unresolved" | "noTarget";
export interface HealthStatus {
  state: HealthState;
  port: number;
  latencyMs: number | null;
}

/** Acierto de la búsqueda global: un nodo y en qué diagrama vive. */
export interface SearchHit {
  nodeId: string;
  mapId: string;
  mapName: string;
  kind: NodeKind;
  label: string;
  /** Campo que casó, legible: "etiqueta", "hostname · web01", "IP · 10.0.0.5". */
  matched: string;
}

/** Host leído de `~/.ssh/config` en la vista previa de importación. */
export interface ImportedHost {
  alias: string;
  hostname: string | null;
  user: string | null;
  port: number | null;
  identityFile: string | null;
}

/** Archivo candidato a importar hallado bajo `~/.ssh` (config y `config.d/**`). */
export interface CandidateFile {
  path: string;
  name: string;
  hostCount: number;
}

export interface Credential {
  id: string;
  nodeId: string;
  kind: CredentialKind;
  username: string | null;
  port: number | null;
  keyPath: string | null;
  isDefault: boolean;
  /** Opciones SSH extra (texto libre, una por línea; se prefijan con `-o`). */
  options: string | null;
  // El secreto nunca viaja al frontend salvo bajo demanda explícita (revelar).
  extras: string;
  /**
   * ¿El vault lleva dentro el material de la llave privada? Booleano derivado,
   * nunca la llave. Distingue "tiene llave" de "la llave viaja con el vault":
   * una credencial con `keyPath` pero sin esto deja de funcionar en cuanto el
   * `.karto` se abre en otro equipo, porque la ruta no apunta a nada allí.
   */
  hasVaultKey: boolean;
  /**
   * ¿El archivo de la llave existe en **este equipo**? Con `hasVaultKey` en
   * false identifica el callejón sin salida: hay ruta, la llave no está aquí y
   * el diagrama tampoco la lleva, así que no hay con qué autenticar — ni
   * siquiera para instalar una nueva.
   */
  keyPresent: boolean;
}
