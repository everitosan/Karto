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
 * ¿Se ofrece el modal de onboarding al conectar?
 *
 * De momento **sólo** para el caso `password`, que es el flujo que funciona hoy.
 * El caso `local-key` ya se detecta (ver `keyOnboardingReason`) pero no se ofrece
 * todavía: sustituir la llave del usuario por una de Karto necesita el arranque
 * con la llave existente, que aún no está. Ofrecerlo antes sería prometer un
 * flujo a medias.
 */
export function needsKeyOnboarding(cred: Credential | undefined): boolean {
  return keyOnboardingReason(cred) === "password";
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
