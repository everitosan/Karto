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
 * ¿Conviene ofrecer configurar acceso por llave? Sí cuando la credencial es SSH
 * por contraseña (sin llave registrada). El resto (web, VNC, SSH ya con llave)
 * conecta directo.
 */
export function needsKeyOnboarding(cred: Credential | undefined): boolean {
  return !!cred && cred.kind === "ssh" && !cred.keyPath;
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
