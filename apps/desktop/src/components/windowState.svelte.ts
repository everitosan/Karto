// Estado reactivo de la ventana (sin decoración). `maximized` gobierna el marco
// redondeado (esquinas rectas al maximizar) y desactiva las zonas de resize.
// Se sincroniza con la ventana real de Tauri; fuera de Tauri queda en `false`.
import { getCurrentWindow } from "@tauri-apps/api/window";

export const windowState = $state<{ maximized: boolean }>({ maximized: false });

/** Arranca la sincronización con la ventana. Devuelve un limpiador. */
export function initWindowState(): () => void {
  const win = getCurrentWindow();

  const refresh = async () => {
    try {
      windowState.maximized = await win.isMaximized();
    } catch {
      // Sin backend Tauri: no hay estado de ventana que reflejar.
    }
  };

  void refresh();

  let unlisten: (() => void) | undefined;
  win
    .onResized(() => void refresh())
    .then((fn) => (unlisten = fn))
    .catch(() => {});

  return () => unlisten?.();
}
