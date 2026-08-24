// Casos de uso de vaults recientes. La lista vive en el directorio de config
// del SO (no en el vault, no viaja con el `.karto`); aquí solo se puentea a los
// comandos Tauri. `default_vault_dir` sugiere dónde crear uno nuevo.
import { bridge, type Bridge } from "./tauri";

export interface RecentVault {
  /** Ruta absoluta del archivo `.karto`. */
  path: string;
  /** Epoch en milisegundos de la última apertura. */
  lastOpened: number;
}

export function makeRecentsUseCases(io: Bridge = bridge) {
  return {
    /** Lista los recientes (el backend purga los que ya no existen en disco). */
    list(): Promise<RecentVault[]> {
      return io.invoke<RecentVault[]>("recents_list");
    },
    /** Olvida un vault; devuelve la lista actualizada. */
    forget(path: string): Promise<RecentVault[]> {
      return io.invoke<RecentVault[]>("recents_forget", { path });
    },
    /** Directorio sugerido para crear un vault nuevo (home del usuario). */
    defaultVaultDir(): Promise<string> {
      return io.invoke<string>("default_vault_dir");
    },
  };
}

export const recentsUseCases = makeRecentsUseCases();
