import { describe, expect, it, vi } from "vitest";
import { createIdleTimer } from "./autoLock";
import { createClipboardManager } from "./clipboard";
import { settingsFromMap, DEFAULT_SETTINGS } from "$usecases/settings";

// --- Temporizador de inactividad (auto-bloqueo) ---

describe("createIdleTimer", () => {
  it("dispara onIdle al agotarse el tiempo", () => {
    const onIdle = vi.fn();
    const t = createIdleTimer(1000, onIdle, {
      setTimer: (cb) => {
        cb(); // ejecuta de inmediato para simular vencimiento
        return 1;
      },
      clearTimer: () => {},
    });
    t.touch();
    expect(onIdle).toHaveBeenCalledTimes(1);
  });

  it("touch reinicia el temporizador (limpia el anterior)", () => {
    const clearTimer = vi.fn();
    let id = 0;
    const t = createIdleTimer(1000, () => {}, {
      setTimer: () => ++id,
      clearTimer,
    });
    t.touch(); // programa id=1
    t.touch(); // limpia id=1, programa id=2
    expect(clearTimer).toHaveBeenCalledWith(1);
  });

  it("no programa nada si timeout <= 0 (auto-bloqueo desactivado)", () => {
    const setTimer = vi.fn();
    const t = createIdleTimer(0, () => {}, { setTimer, clearTimer: () => {} });
    t.touch();
    expect(setTimer).not.toHaveBeenCalled();
  });
});

// --- Gestor de portapapeles con limpieza automática ---

describe("createClipboardManager", () => {
  it("copia el texto y programa limpiarlo tras N segundos", async () => {
    const writes: string[] = [];
    let fire: (() => void) | null = null;
    const mgr = createClipboardManager({
      write: async (t) => {
        writes.push(t);
      },
      setTimer: (cb) => {
        fire = cb;
        return 1;
      },
      clearTimer: () => {},
    });

    await mgr.copy("s3cret", 30);
    expect(writes).toEqual(["s3cret"]);
    expect(fire).not.toBeNull();

    fire!(); // simula el vencimiento
    await Promise.resolve();
    expect(writes).toEqual(["s3cret", ""]);
  });

  it("no programa limpieza si clearAfterSeconds es 0", async () => {
    const setTimer = vi.fn();
    const mgr = createClipboardManager({
      write: async () => {},
      setTimer,
      clearTimer: () => {},
    });
    await mgr.copy("x", 0);
    expect(setTimer).not.toHaveBeenCalled();
  });

  it("clearNow limpia lo copiado y cancela el temporizador pendiente", async () => {
    const writes: string[] = [];
    const clearTimer = vi.fn();
    const mgr = createClipboardManager({
      write: async (t) => {
        writes.push(t);
      },
      setTimer: () => 7,
      clearTimer,
    });
    await mgr.copy("secreto", 30);
    await mgr.clearNow();
    expect(clearTimer).toHaveBeenCalledWith(7);
    expect(writes).toEqual(["secreto", ""]);
  });

  it("clearNow no escribe si no hay nada copiado", async () => {
    const write = vi.fn(async () => {});
    const mgr = createClipboardManager({ write, setTimer: () => 1, clearTimer: () => {} });
    await mgr.clearNow();
    expect(write).not.toHaveBeenCalled();
  });
});

// --- Parseo de preferencias ---

describe("settingsFromMap", () => {
  it("usa defaults cuando faltan claves", () => {
    expect(settingsFromMap({})).toEqual(DEFAULT_SETTINGS);
  });

  it("parsea valores numéricos válidos", () => {
    expect(settingsFromMap({ autoLockMinutes: "5", clipboardClearSeconds: "0" })).toEqual({
      autoLockMinutes: 5,
      clipboardClearSeconds: 0,
    });
  });

  it("cae al default ante valores inválidos o negativos", () => {
    expect(settingsFromMap({ autoLockMinutes: "-3", clipboardClearSeconds: "abc" })).toEqual(
      DEFAULT_SETTINGS,
    );
  });
});
