// Orquestación del flujo de conexión con onboarding de llave SSH.
// Un único estado compartido (module-level) permite que el panel de propiedades
// y el menú contextual disparen la conexión, y que un solo modal —montado en el
// Canvas— pida las opciones cuando la credencial es SSH por contraseña.
import type { Credential } from "$domain/infra";
import { workspaceUseCases } from "$usecases/workspace";
import { networkContext } from "../networkContext.svelte";
import { needsKeyOnboarding, pickCredential, type KeyOnboardingChoice } from "./connectFlow";

interface Pending {
  nodeId: string;
  credential: Credential;
  /** Se invoca tras aprovisionar (p. ej. recargar credenciales del panel). */
  onProvisioned?: () => void;
}

export const onboarding = $state<{ pending: Pending | null; busy: boolean }>({
  pending: null,
  busy: false,
});

// --- Sondeo de datos del equipo tras conectar por SSH ---
// La conexión deja los datos (hostname, SO, kernel…) en un archivo local que el
// backend lee; aquí reintentamos hasta que estén listos y avisamos al canvas
// para que refresque las propiedades del nodo. Un único listener (lo registra el
// FlowEditor montado) evita acoplar el flujo de conexión con el interior del canvas.
type FactsListener = (nodeId: string, facts: Record<string, string>) => void;
let factsListener: FactsListener | null = null;

export function onFactsCollected(cb: FactsListener | null): void {
  factsListener = cb;
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

async function collectFacts(nodeId: string): Promise<void> {
  // El sondeo remoto tarda unos segundos (auth + comandos); reintenta ~40s.
  for (let i = 0; i < 30; i++) {
    await sleep(1300);
    try {
      const facts = await workspaceUseCases.pollFacts(nodeId);
      if (facts && Object.keys(facts).length > 0) {
        factsListener?.(nodeId, facts);
        return;
      }
    } catch {
      // Vault bloqueado / error transitorio: dejamos de sondear.
      return;
    }
  }
}

/**
 * Punto de entrada desde la UI. Si la credencial objetivo es SSH por contraseña
 * (sin llave), abre el modal de onboarding; si no, conecta directo.
 */
export async function requestConnect(
  nodeId: string,
  credentialId: string | null,
  credentials: Credential[],
  onProvisioned?: () => void,
): Promise<void> {
  const credential = pickCredential(credentials, credentialId);
  if (needsKeyOnboarding(credential)) {
    onboarding.pending = { nodeId, credential: credential!, onProvisioned };
    return;
  }
  await workspaceUseCases.connectNode(nodeId, credentialId, networkContext.activeId);
  // Al conectar por SSH, la terminal sondea el equipo; recogemos los datos.
  if (credential?.kind === "ssh") void collectFacts(nodeId);
}

/** Aplica la elección del modal: aprovisiona la llave o conecta con contraseña. */
export async function confirmOnboarding(choice: KeyOnboardingChoice): Promise<void> {
  const pending = onboarding.pending;
  if (!pending) return;
  onboarding.busy = true;
  try {
    if (choice.registerKey) {
      await workspaceUseCases.provisionSshKey(
        pending.nodeId,
        pending.credential.id,
        choice.setDefaultKey,
        choice.storeInVault,
        networkContext.activeId,
      );
      pending.onProvisioned?.();
    } else {
      await workspaceUseCases.connectNode(
        pending.nodeId,
        pending.credential.id,
        networkContext.activeId,
      );
      void collectFacts(pending.nodeId);
    }
    onboarding.pending = null;
  } finally {
    onboarding.busy = false;
  }
}

export function cancelOnboarding(): void {
  onboarding.pending = null;
}
