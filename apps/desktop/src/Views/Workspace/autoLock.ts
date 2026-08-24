// Temporizador de inactividad para el auto-bloqueo del vault. Puro y testeable:
// los temporizadores se inyectan (por defecto usa `setTimeout`/`clearTimeout`).
// `touch()` reinicia la cuenta ante actividad del usuario; al agotarse llama a
// `onIdle`. `timeoutMs <= 0` desactiva el auto-bloqueo.

export interface IdleTimerDeps {
  setTimer?: (cb: () => void, ms: number) => number;
  clearTimer?: (id: number) => void;
}

export interface IdleTimer {
  /** Reinicia la cuenta (llamar ante cada señal de actividad). */
  touch: () => void;
  /** Detiene el temporizador sin disparar `onIdle`. */
  stop: () => void;
}

export function createIdleTimer(
  timeoutMs: number,
  onIdle: () => void,
  deps: IdleTimerDeps = {},
): IdleTimer {
  const setTimer =
    deps.setTimer ?? ((cb, ms) => setTimeout(cb, ms) as unknown as number);
  const clearTimer = deps.clearTimer ?? ((id) => clearTimeout(id));

  let handle: number | null = null;

  function stop() {
    if (handle !== null) {
      clearTimer(handle);
      handle = null;
    }
  }

  function touch() {
    stop();
    if (timeoutMs > 0) {
      handle = setTimer(() => {
        handle = null;
        onIdle();
      }, timeoutMs);
    }
  }

  return { touch, stop };
}
