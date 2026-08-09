// Entidades y tipos de negocio del vault. Sin dependencias de UI ni de Tauri.

export type VaultStatus = "no-vault" | "locked" | "unlocked";

export interface VaultInfo {
  /** Ruta absoluta del archivo .karto actualmente seleccionado, si hay alguno. */
  path: string | null;
  status: VaultStatus;
}

export interface CreateVaultInput {
  path: string;
  password: string;
}

export interface UnlockVaultInput {
  path: string;
  password: string;
}

export class VaultError extends Error {
  constructor(
    message: string,
    readonly kind: "wrong-password" | "not-found" | "io" | "unknown" = "unknown",
  ) {
    super(message);
    this.name = "VaultError";
  }
}
