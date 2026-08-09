// Entidades del dominio de infraestructura (carpetas, diagramas, nodos,
// aristas, credenciales). Los nombres de campo casan con la serialización
// `camelCase` del backend Rust.
//
// El catálogo de tipos de nodo (categorías, etiquetas, propiedades e iconos)
// es la fuente compartida en `@karto/ui`; aquí solo se re-exporta.
export type { NodeKind, NodeCategory } from "@karto/ui";
export {
  NODE_KINDS,
  NODE_KIND_LABELS,
  NODE_CATALOG,
  NODE_CATEGORIES,
  CATEGORY_LABELS,
  nodesByCategory,
  resolveNodeIcon,
} from "@karto/ui";

import type { NodeKind } from "@karto/ui";

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
