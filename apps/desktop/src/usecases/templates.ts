// Casos de uso de plantillas de conexión. Dos ámbitos (ver decisión de diseño):
//  - Biblioteca a nivel de app (máquina): `list/upsert/delete`.
//  - Ligado por-vault: `linkToVault` copia el comando al vault (viaja con el
//    `.karto`); `vaultList`/`unlink` gestionan los overrides ligados.
import { bridge, type Bridge } from "./tauri";

export interface Template {
  id: string;
  name: string;
  /** Tipo de conexión: `ssh`/`vnc`/`web`/`rdp`. */
  connection: string;
  /** Comando con placeholders: `{host}` `{port}` `{user}` `{key}` `{userhost}`. */
  command: string;
  isDefault: boolean;
  position: number;
}

/** Override de plantilla ligado dentro de un vault. */
export interface VaultTemplate {
  connection: string;
  os: string;
  command: string;
}

export function makeTemplatesUseCases(io: Bridge = bridge) {
  return {
    list(): Promise<Template[]> {
      return io.invoke<Template[]>("template_list");
    },
    upsert(input: {
      id?: string;
      name: string;
      connection: string;
      command: string;
    }): Promise<void> {
      return io.invoke<void>("template_upsert", input);
    },
    remove(id: string): Promise<void> {
      return io.invoke<void>("template_delete", { id });
    },
    /** Liga (copia) una plantilla de la biblioteca al vault abierto. */
    linkToVault(id: string): Promise<void> {
      return io.invoke<void>("template_link_to_vault", { id });
    },
    /** Overrides ligados en el vault abierto. */
    vaultList(): Promise<VaultTemplate[]> {
      return io.invoke<VaultTemplate[]>("template_vault_list");
    },
    /** Desliga el override del vault para un tipo de conexión. */
    unlink(connection: string): Promise<void> {
      return io.invoke<void>("template_vault_unlink", { connection });
    },
  };
}

export const templatesUseCases = makeTemplatesUseCases();
