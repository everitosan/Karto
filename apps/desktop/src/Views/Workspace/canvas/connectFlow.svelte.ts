// Orquestación del flujo de conexión con onboarding de llave SSH.
// Un único estado compartido (module-level) permite que el panel de propiedades
// y el menú contextual disparen la conexión, y que un solo modal —montado en el
// Canvas— pida las opciones cuando la credencial es SSH por contraseña.
import type { Credential } from "$domain/infra";
import { workspaceUseCases } from "$usecases/workspace";
import { networkContext } from "../networkContext.svelte";
import {
  needsKeyOnboarding,
  pickCredential,
  templateConfirmCommand,
  type KeyOnboardingChoice,
} from "./connectFlow";

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

// --- Confirmación de plantilla de vault importado ---
interface TemplateConfirm {
  nodeId: string;
  credentialId: string | null;
  command: string;
}

// Estado del diálogo de confirmación (lo muestra el Canvas, igual que onboarding).
export const templateConfirm = $state<{ pending: TemplateConfirm | null }>({
  pending: null,
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
 * Lanza la conexión capturando el aviso de confirmación de plantilla: si el vault
 * (importado) trae una plantilla personalizada sin confianza, en vez de ejecutarla
 * abre el diálogo de confirmación. Devuelve `true` si conectó, `false` si quedó a
 * la espera de confirmación. Otros errores se propagan.
 */
async function connectOrConfirm(
  nodeId: string,
  credentialId: string | null,
): Promise<boolean> {
  try {
    await workspaceUseCases.connectNode(nodeId, credentialId, networkContext.activeId);
    return true;
  } catch (err) {
    const command = templateConfirmCommand(err);
    if (command === null) throw err;
    templateConfirm.pending = { nodeId, credentialId, command };
    return false;
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
  const connected = await connectOrConfirm(nodeId, credentialId);
  // Al conectar por SSH, la terminal sondea el equipo; recogemos los datos.
  if (connected && credential?.kind === "ssh") void collectFacts(nodeId);
}

/**
 * Aplica la confirmación de la plantilla: marca el vault como de confianza en esta
 * máquina y reintenta la conexión (que ahora sí ejecuta la plantilla).
 */
export async function confirmTemplateTrust(): Promise<void> {
  const pending = templateConfirm.pending;
  if (!pending) return;
  await workspaceUseCases.trustVaultTemplates();
  templateConfirm.pending = null;
  const connected = await connectOrConfirm(pending.nodeId, pending.credentialId);
  if (connected) void collectFacts(pending.nodeId);
}

export function cancelTemplateTrust(): void {
  templateConfirm.pending = null;
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
      const connected = await connectOrConfirm(pending.nodeId, pending.credential.id);
      if (connected) void collectFacts(pending.nodeId);
    }
    onboarding.pending = null;
  } finally {
    onboarding.busy = false;
  }
}

export function cancelOnboarding(): void {
  onboarding.pending = null;
}
