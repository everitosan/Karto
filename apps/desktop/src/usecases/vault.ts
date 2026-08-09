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
  };
}

export const vaultUseCases = makeVaultUseCases();
