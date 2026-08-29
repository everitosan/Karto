// Helpers puros del flujo de conexión con onboarding de llave SSH.
// La orquestación reactiva (store + modal) vive en `connectFlow.svelte.ts`;
// aquí solo las decisiones puras, testeables sin UI.
import type { Credential } from "$domain/infra";

/** Credencial objetivo: la indicada, o la predeterminada del nodo. */
export function pickCredential(
  credentials: Credential[],
  credentialId: string | null,
): Credential | undefined {
  if (credentialId) return credentials.find((c) => c.id === credentialId);
  return credentials.find((c) => c.isDefault) ?? credentials[0];
}

/**
 * Por qué convendría ofrecer acceso por llave gestionada por Karto.
 * - `password`: SSH sin llave ninguna.
 * - `local-key`: SSH con llave que **sólo existe en este equipo**. Conecta bien
 *   aquí, pero el vault no se la lleva: al abrir el `.karto` en otro sitio
 *   `keyPath` apunta a un archivo que allí no está.
 */
export type KeyOnboardingReason = "password" | "local-key";

/**
 * ¿Esta credencial se ha quedado sin forma de autenticar? Hay ruta de llave, el
 * archivo no está en este equipo y el diagrama no lleva el material dentro. Pasa
 * al abrir un `.karto` hecho en otra máquina cuya credencial apuntaba a una llave
 * personal: ni conecta, ni se puede aprovisionar una nueva desde aquí.
 *
 * No lo decide Karto por el usuario —el servidor podría admitir contraseña—, pero
 * conviene avisarlo antes de que se coma un `Permission denied (publickey)`.
 */
export function keyIsUnreachable(cred: Credential | undefined): boolean {
  return !!cred && cred.kind === "ssh" && !!cred.keyPath && !cred.keyPresent && !cred.hasVaultKey;
}

/**
 * Clasifica una credencial según si **el vault puede llevársela**, que es la
 * pregunta que importa para la portabilidad —distinta de "¿le falta llave?"—.
 * `null` cuando no aplica: no es SSH, o la llave ya viaja dentro del vault.
 */
export function keyOnboardingReason(
  cred: Credential | undefined,
): KeyOnboardingReason | null {
  if (!cred || cred.kind !== "ssh" || cred.hasVaultKey) return null;
  return cred.keyPath ? "local-key" : "password";
}

/**
 * ¿Se ofrece el modal de onboarding al conectar? Siempre que el vault no pueda
 * llevarse la credencial, sea porque no hay llave (`password`) o porque la que
 * hay sólo existe en este equipo (`local-key`).
 */
export function needsKeyOnboarding(cred: Credential | undefined): boolean {
  return keyOnboardingReason(cred) !== null;
}

/** Opciones elegidas en el modal de onboarding de llave. */
export interface KeyOnboardingChoice {
  registerKey: boolean;
  setDefaultKey: boolean;
  storeInVault: boolean;
}

// Prefijo estable del error que devuelve `connect_node` cuando el vault trae una
// plantilla de conexión y no está marcado como de confianza en esta máquina.
const TEMPLATE_CONFIRM_PREFIX = "confirmación de plantilla requerida:";

/**
 * Si `err` es el aviso de confirmación de plantilla, devuelve el comando que la
 * plantilla ejecutaría; si es cualquier otro error, devuelve `null` (el caller lo
 * propaga). Puro y testeable.
 */
export function templateConfirmCommand(err: unknown): string | null {
  const msg =
    typeof err === "string" ? err : ((err as Error)?.message ?? String(err));
  return msg.startsWith(TEMPLATE_CONFIRM_PREFIX)
    ? msg.slice(TEMPLATE_CONFIRM_PREFIX.length).trim()
    : null;
}
