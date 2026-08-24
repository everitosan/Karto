// Entidades del dominio de infraestructura (carpetas, diagramas, nodos,
// aristas, credenciales). Los nombres de campo casan con la serialización
// `camelCase` del backend Rust.
//
// El catálogo de tipos de nodo (categorías, etiquetas, propiedades e iconos)
// es la fuente compartida en `@karto/catalog`; aquí solo se re-exporta.
export type { NodeKind, NodeCategory } from "@karto/catalog";
export {
  NODE_KINDS,
  NODE_KIND_LABELS,
  NODE_CATALOG,
  NODE_CATEGORIES,
  CATEGORY_LABELS,
  nodesByCategory,
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
}
