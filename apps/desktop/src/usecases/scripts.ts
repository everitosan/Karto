// Casos de uso de Scripts remotos.
//
// La biblioteca de scripts vive a nivel de app/máquina (`app_store::scripts`);
// la ejecución corre el script por SSH sobre los equipos seleccionados de un
// diagrama y transmite la salida **en vivo** por un `Channel` de Tauri (evento
// por línea/estado). Ver `usecases/scripts.rs` en el backend.
import { Channel } from "@tauri-apps/api/core";
import { bridge, type Bridge } from "./tauri";

/** Modo de ejecución sobre el conjunto de equipos objetivo. */
export type RunMode = "sequential" | "parallel";

/**
 * Intérprete del script. Shell (bash/python) → host-side por SSH; motores de BD
 * (postgresql/mysql/mariadb/mongodb/redis) → cliente local por red (Modelo B).
 */
export type Interpreter =
  | "bash"
  | "python"
  | "postgresql"
  | "mysql"
  | "mariadb"
  | "mongodb"
  | "redis";

/** Un script guardado en la biblioteca (a nivel de máquina). */
export interface Script {
  id: string;
  name: string;
  /** Cuerpo del script, pasado por stdin al intérprete al ejecutar. */
  body: string;
  position: number;
  /** Carpeta a la que pertenece, o `null` si está suelto. */
  folderId: string | null;
  /** Intérprete con el que se ejecuta (`bash` | `python`). */
  interpreter: Interpreter;
}

/** Carpeta (plana, un nivel) para agrupar scripts. */
export interface ScriptFolder {
  id: string;
  name: string;
  position: number;
}

/**
 * Un equipo de un diagrama con sus capacidades crudas. La compatibilidad y la
 * selectabilidad se calculan en el frontend según el intérprete del script.
 */
export interface ScriptTarget {
  nodeId: string;
  label: string;
  /** Tipo de nodo del catálogo (server, vm, database…). */
  kind: string;
  /** Gestor de BD (`postgresql`, `mysql`…) si el nodo lo declara. */
  gestor: string | null;
  /** Sistema operativo declarado (propiedad `os`); relevante en server/vm. */
  os: string | null;
  /** Nombre de la BD/instancia registrada (propiedad `instancia`); en `database`. */
  instance: string | null;
  /** Tiene credencial SSH con llave (para bash/python). */
  sshKey: boolean;
  /** Tiene credencial de BD (`kind='db'`) para los motores de BD. */
  dbCred: boolean;
}

/** Estado de la ejecución de un script en un equipo concreto. */
export type TargetRunStatus = "pending" | "running" | "ok" | "error";

export interface TargetRun {
  nodeId: string;
  status: TargetRunStatus;
  /** Salida acumulada (stdout+stderr) del equipo. */
  output: string;
  exitCode?: number | null;
  /** Mensaje si el equipo ni siquiera pudo arrancar (p. ej. "requiere llave"). */
  error?: string | null;
}

/** Eventos que emite el backend por el `Channel` (espejo de `RunEvent` en Rust). */
export type RunEvent =
  | { type: "status"; nodeId: string; status: TargetRunStatus }
  | { type: "line"; nodeId: string; line: string }
  | { type: "done"; nodeId: string; exitCode: number | null; error: string | null };

export function makeScriptsUseCases(
  io: Bridge = bridge,
  makeChannel: () => Channel<RunEvent> = () => new Channel<RunEvent>(),
) {
  return {
    list(): Promise<Script[]> {
      return io.invoke<Script[]>("script_list");
    },
    upsert(input: {
      id?: string;
      name: string;
      body: string;
      interpreter: Interpreter;
    }): Promise<void> {
      return io.invoke<void>("script_upsert", input);
    },
    remove(id: string): Promise<void> {
      return io.invoke<void>("script_delete", { id });
    },
    // --- Carpetas ---
    listFolders(): Promise<ScriptFolder[]> {
      return io.invoke<ScriptFolder[]>("script_folder_list");
    },
    createFolder(name: string): Promise<string> {
      return io.invoke<string>("script_folder_create", { name });
    },
    renameFolder(id: string, name: string): Promise<void> {
      return io.invoke<void>("script_folder_rename", { id, name });
    },
    removeFolder(id: string): Promise<void> {
      return io.invoke<void>("script_folder_delete", { id });
    },
    /** Mueve un script a una carpeta (`folderId = null` lo deja suelto). */
    setFolder(id: string, folderId: string | null): Promise<void> {
      return io.invoke<void>("script_set_folder", { id, folderId });
    },
    /** Equipos del diagrama con su elegibilidad para ejecutar. */
    targets(mapId: string): Promise<ScriptTarget[]> {
      return io.invoke<ScriptTarget[]>("script_targets", { mapId });
    },
    /**
     * Ejecuta el script sobre los equipos indicados. `onEvent` recibe la salida
     * en vivo (estado/línea/fin por equipo). La promesa resuelve al terminar todo.
     */
    run(
      input: {
        nodeIds: string[];
        body: string;
        interpreter: Interpreter;
        mode: RunMode;
        contextId?: string | null;
      },
      onEvent: (ev: RunEvent) => void,
    ): Promise<void> {
      const channel = makeChannel();
      channel.onmessage = onEvent;
      return io.invoke<void>("scripts_run", { ...input, onEvent: channel });
    },
  };
}

export const scriptsUseCases = makeScriptsUseCases();
