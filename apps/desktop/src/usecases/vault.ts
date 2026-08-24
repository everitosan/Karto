// Casos de uso del vault: traducen intenciones de la UI a comandos del backend Rust.
// No contienen JSX/Svelte; son funciones puras salvo por el puente Tauri inyectable.
import type {
  CreateVaultInput,
  UnlockVaultInput,
  VaultInfo,
} from "$domain/vault";
import { VaultError } from "$domain/vault";
import { bridge, type Bridge } from "./tauri";

function toVaultError(error: unknown): VaultError {
  const message = typeof error === "string" ? error : String(error);
  if (/wrong.?password|invalid.?password|HMAC/i.test(message)) {
    return new VaultError(message, "wrong-password");
  }
  if (/not.?found|no such file/i.test(message)) {
    return new VaultError(message, "not-found");
  }
  return new VaultError(message, "unknown");
}

export function makeVaultUseCases(io: Bridge = bridge) {
  return {
    async status(): Promise<VaultInfo> {
      return io.invoke<VaultInfo>("vault_status");
    },

    async create(input: CreateVaultInput): Promise<VaultInfo> {
      try {
        return await io.invoke<VaultInfo>("vault_create", { ...input });
      } catch (error) {
        throw toVaultError(error);
      }
    },

    async unlock(input: UnlockVaultInput): Promise<VaultInfo> {
      try {
        return await io.invoke<VaultInfo>("vault_unlock", { ...input });
      } catch (error) {
        throw toVaultError(error);
      }
    },

    async lock(): Promise<VaultInfo> {
      return io.invoke<VaultInfo>("vault_lock");
    },

    // Cierra el vault por completo (olvida el path) → vuelve a la selección.
    async close(): Promise<VaultInfo> {
      return io.invoke<VaultInfo>("vault_close");
    },

    // Cambia la contraseña maestra (re-cifra el vault). El backend verifica la
    // actual antes de aplicar; si no coincide, lanza `wrong-password`.
    async rekey(current: string, next: string): Promise<void> {
      try {
        await io.invoke<void>("vault_rekey", { current, new: next });
      } catch (error) {
        throw toVaultError(error);
      }
    },

    // Escribe una copia de respaldo cifrada del vault en `dest`.
    async exportBackup(dest: string): Promise<void> {
      await io.invoke<void>("vault_export", { dest });
    },

    // Exporta un subconjunto de nodos a un `.karto` nuevo cifrado con `password`.
    async exportSubset(input: {
      dest: string;
      password: string;
      nodeIds: string[];
      mapName: string;
      includeCredentials: boolean;
      includeFacts: boolean;
      includeIp: boolean;
      includeNotes: boolean;
    }): Promise<void> {
      await io.invoke<void>("vault_export_subset", input);
    },
  };
}

export const vaultUseCases = makeVaultUseCases();
