// Casos de uso del workspace: traducen intenciones de la UI (crear carpeta,
// mover diagrama, guardar posición de un nodo, revelar un secreto…) a comandos
// del backend Rust. Sin dependencias de Svelte; el puente Tauri es inyectable
// para poder testear con un mock.
import type {
  Credential,
  Folder,
  Graph,
  InfraEdge,
  InfraMap,
  InfraNode,
  NodeKind,
} from "$domain/infra";
import { bridge, type Bridge } from "./tauri";

export interface CredentialInput {
  id?: string | null;
  nodeId: string;
  kind: string;
  username?: string | null;
  secret?: string | null;
  port?: number | null;
  keyPath?: string | null;
  isDefault: boolean;
  options?: string | null;
}

export function makeWorkspaceUseCases(io: Bridge = bridge) {
  return {
    // --- Carpetas ---
    listFolders: () => io.invoke<Folder[]>("folder_list"),
    createFolder: (name: string, parentId: string | null = null) =>
      io.invoke<Folder>("folder_create", { name, parentId }),
    renameFolder: (id: string, name: string) =>
      io.invoke<void>("folder_rename", { id, name }),
    setFolderColor: (id: string, color: string | null) =>
      io.invoke<void>("folder_set_color", { id, color }),
    moveFolder: (id: string, parentId: string | null, position: number) =>
      io.invoke<void>("folder_move", { id, parentId, position }),
    deleteFolder: (id: string) => io.invoke<void>("folder_delete", { id }),

    // --- Diagramas ---
    listMaps: () => io.invoke<InfraMap[]>("map_list"),
    createMap: (name: string, folderId: string | null = null) =>
      io.invoke<InfraMap>("map_create", { name, folderId }),
    renameMap: (id: string, name: string) =>
      io.invoke<void>("map_rename", { id, name }),
    moveMap: (id: string, folderId: string | null, position: number) =>
      io.invoke<void>("map_move", { id, folderId, position }),
    deleteMap: (id: string) => io.invoke<void>("map_delete", { id }),
    setMapViewport: (id: string, viewport: string) =>
      io.invoke<void>("map_set_viewport", { id, viewport }),

    // --- Grafo ---
    loadGraph: (mapId: string) => io.invoke<Graph>("graph_load", { mapId }),
    createNode: (
      mapId: string,
      kind: NodeKind,
      label: string,
      x: number,
      y: number,
    ) => io.invoke<InfraNode>("node_create", { mapId, kind, label, x, y }),
    setNodePosition: (id: string, x: number, y: number) =>
      io.invoke<void>("node_set_position", { id, x, y }),
    setNodeParent: (id: string, parentId: string | null) =>
      io.invoke<void>("node_set_parent", { id, parentId }),
    renameNode: (id: string, label: string) =>
      io.invoke<void>("node_rename", { id, label }),
    setNodeProperties: (id: string, properties: Record<string, string>) =>
      io.invoke<void>("node_set_properties", { id, properties }),
    deleteNode: (id: string) => io.invoke<void>("node_delete", { id }),
    createEdge: (
      mapId: string,
      sourceId: string,
      targetId: string,
      label: string | null = null,
    ) => io.invoke<InfraEdge>("edge_create", { mapId, sourceId, targetId, label }),
    setEdgeLabel: (id: string, label: string | null) =>
      io.invoke<void>("edge_set_label", { id, label }),
    setEdgeStyle: (id: string, style: string) =>
      io.invoke<void>("edge_set_style", { id, style }),
    deleteEdge: (id: string) => io.invoke<void>("edge_delete", { id }),

    // --- Credenciales ---
    listCredentials: (nodeId: string) =>
      io.invoke<Credential[]>("credential_list", { nodeId }),
    upsertCredential: (input: CredentialInput) =>
      io.invoke<Credential>("credential_upsert", {
        id: input.id ?? null,
        nodeId: input.nodeId,
        kind: input.kind,
        username: input.username ?? null,
        secret: input.secret ?? null,
        port: input.port ?? null,
        keyPath: input.keyPath ?? null,
        isDefault: input.isDefault,
        options: input.options ?? null,
      }),
    revealCredential: (id: string) =>
      io.invoke<string | null>("credential_reveal", { id }),
    deleteCredential: (id: string) =>
      io.invoke<void>("credential_delete", { id }),

    // --- Conexión ---
    // Lanza la conexión del nodo. Con `credentialId` nulo usa la predeterminada.
    // El secreto nunca viaja aquí: lo revela y lo consume el backend.
    connectNode: (nodeId: string, credentialId: string | null = null) =>
      io.invoke<void>("connect_node", { nodeId, credentialId }),
  };
}

export type WorkspaceUseCases = ReturnType<typeof makeWorkspaceUseCases>;

export const workspaceUseCases = makeWorkspaceUseCases();
