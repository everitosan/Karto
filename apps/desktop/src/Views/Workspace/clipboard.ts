// Gestor de portapapeles con limpieza automática tras copiar un secreto.
// Puro y testeable: la escritura y los temporizadores se inyectan. Guarda lo
// último copiado para no borrar algo que el usuario ya reemplazó por su cuenta.

export interface ClipboardDeps {
  write?: (text: string) => Promise<void>;
  setTimer?: (cb: () => void, ms: number) => number;
  clearTimer?: (id: number) => void;
}

export interface ClipboardManager {
  /**
   * Copia `text` y, si `clearAfterSeconds > 0`, programa limpiarlo pasado ese
   * tiempo (salvo que se copie otra cosa antes).
   */
  copy: (text: string, clearAfterSeconds: number) => Promise<void>;
  /** Limpia el portapapeles ya (p. ej. al bloquear el vault). */
  clearNow: () => Promise<void>;
  /** Cancela la limpieza programada sin tocar el portapapeles. */
  cancel: () => void;
}

async function defaultWrite(text: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    // El portapapeles puede no estar disponible fuera de la app empaquetada.
  }
}

export function createClipboardManager(deps: ClipboardDeps = {}): ClipboardManager {
  const write = deps.write ?? defaultWrite;
  const setTimer =
    deps.setTimer ?? ((cb, ms) => setTimeout(cb, ms) as unknown as number);
  const clearTimer = deps.clearTimer ?? ((id) => clearTimeout(id));

  let handle: number | null = null;
  let lastCopied: string | null = null;

  function cancel() {
    if (handle !== null) {
      clearTimer(handle);
      handle = null;
    }
  }

  async function clearNow() {
    cancel();
    if (lastCopied !== null) {
      lastCopied = null;
      await write("");
    }
  }

  async function copy(text: string, clearAfterSeconds: number) {
    cancel();
    lastCopied = text;
    await write(text);
    if (clearAfterSeconds > 0) {
      handle = setTimer(() => {
        handle = null;
        lastCopied = null;
        void write("");
      }, clearAfterSeconds * 1000);
    }
  }

  return { copy, clearNow, cancel };
}

/** Instancia compartida por la app (usa `navigator.clipboard`). */
export const clipboardManager = createClipboardManager();
