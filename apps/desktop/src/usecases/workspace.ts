// Casos de uso del workspace: traducen intenciones de la UI (crear carpeta,
// mover diagrama, guardar posición de un nodo, revelar un secreto…) a comandos
// del backend Rust. Sin dependencias de Svelte; el puente Tauri es inyectable
// para poder testear con un mock.
import type {
  AccessContext,
  CandidateFile,
  Credential,
  Folder,
  Graph,
  HealthStatus,
  ImportedHost,
  InfraEdge,
  InfraMap,
  InfraNode,
  NodeKind,
  SearchHit,
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
    searchNodes: (query: string) => io.invoke<SearchHit[]>("node_search", { query }),
    // Escribe la imagen exportada (bytes) en la ruta elegida.
    exportWrite: (path: string, data: number[]) =>
      io.invoke<void>("export_write", { path, data }),
    // Lee el sondeo del equipo tras conectar (null si aún no está listo).
    pollFacts: (nodeId: string) =>
      io.invoke<Record<string, string> | null>("facts_poll", { nodeId }),
    // Comprueba (TCP) si el nodo responde en su puerto de servicio.
    checkHealth: (nodeId: string, contextId: string | null) =>
      io.invoke<HealthStatus>("node_health", { nodeId, contextId }),
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
    // Reemplaza las direcciones por contexto del nodo (`contextId` → dirección).
    setNodeEndpoints: (id: string, endpoints: Record<string, string>) =>
      io.invoke<void>("node_set_endpoints", { id, endpoints }),
    deleteNode: (id: string) => io.invoke<void>("node_delete", { id }),

    // --- Contextos de acceso (puntos de vista de red) ---
    listContexts: () => io.invoke<AccessContext[]>("context_list"),
    createContext: (name: string) =>
      io.invoke<AccessContext>("context_create", { name }),
    renameContext: (id: string, name: string) =>
      io.invoke<void>("context_rename", { id, name }),
    deleteContext: (id: string) => io.invoke<void>("context_delete", { id }),
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
    // `contextId` selecciona la dirección del nodo según el punto de vista de red
    // activo (oficina/VPN/…); nulo ⇒ el backend cae al hostname de respaldo.
    connectNode: (
      nodeId: string,
      credentialId: string | null = null,
      contextId: string | null = null,
    ) => io.invoke<void>("connect_node", { nodeId, credentialId, contextId }),

    // Autoriza (en esta máquina) ejecutar las plantillas de conexión que trae el
    // vault abierto. Se llama tras confirmar el aviso de `connect_node` cuando el
    // vault es importado y trae una plantilla personalizada.
    trustVaultTemplates: () => io.invoke<void>("vault_trust_templates", {}),

    // Abre en el navegador la URL de administración del nodo (propiedad
    // `url_admin`/`url`). No usa credenciales: abrir un enlace no requiere secreto.
    openNodeUrl: (nodeId: string) => io.invoke<void>("open_node_url", { nodeId }),

    // Aprovisiona acceso por llave SSH para una credencial de contraseña: genera
    // la llave, la copia al servidor con ssh-copy-id y encadena la conexión por
    // llave (el usuario teclea la contraseña una vez). `setDefaultKey` fija la
    // llave como método por defecto; `storeInVault` guarda la privada en el vault.
    provisionSshKey: (
      nodeId: string,
      credentialId: string,
      setDefaultKey: boolean,
      storeInVault: boolean,
      contextId: string | null = null,
    ) =>
      io.invoke<string>("ssh_provision_key", {
        nodeId,
        credentialId,
        contextId,
        setDefaultKey,
        storeInVault,
      }),

    // ¿Terminó bien el aprovisionamiento lanzado antes? La terminal es
    // interactiva y se lanza sin esperarla, así que el backend sólo toca la
    // credencial cuando ve el marcador de éxito que deja `ssh-copy-id`. Devuelve
    // `true` la única vez que aplica los cambios.
    pollProvision: (
      credentialId: string,
      setDefaultKey: boolean,
      storeInVault: boolean,
    ) =>
      io.invoke<boolean>("ssh_provision_poll", {
        credentialId,
        setDefaultKey,
        storeInVault,
      }),

    // --- Importación SSH ---
    // Candidatos a importar hallados bajo `~/.ssh` (config y config.d/**).
    sshImportCandidates: () =>
      io.invoke<CandidateFile[]>("ssh_import_candidates"),
    // Parsea un archivo de config SSH concreto (sugerencia o soltado).
    sshImportParseFile: (path: string) =>
      io.invoke<ImportedHost[]>("ssh_import_parse_file", { path }),
    // Crea un nodo (con credencial SSH) por host en el diagrama destino. La IP
    // se registra como dirección del contexto `contextId` elegido.
    sshImportHosts: (mapId: string, contextId: string, hosts: ImportedHost[]) =>
      io.invoke<InfraNode[]>("ssh_import_hosts", { mapId, contextId, hosts }),
  };
}

export type WorkspaceUseCases = ReturnType<typeof makeWorkspaceUseCases>;

export const workspaceUseCases = makeWorkspaceUseCases();
