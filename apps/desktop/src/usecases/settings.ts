// Casos de uso de preferencias de la app. Se guardan dentro del vault cifrado
// (tabla `settings`) vía comandos Tauri; aquí se traducen a un objeto tipado.
// `settingsFromMap` es puro y testeable sin backend.
import { bridge, type Bridge } from "./tauri";

export interface AppSettings {
  /** Minutos de inactividad antes de auto-bloquear. 0 = desactivado. */
  autoLockMinutes: number;
  /** Segundos tras copiar un secreto antes de limpiar el portapapeles. 0 = no limpiar. */
  clipboardClearSeconds: number;
}

export const DEFAULT_SETTINGS: AppSettings = {
  autoLockMinutes: 15,
  clipboardClearSeconds: 30,
};

function parseNonNegative(value: string | undefined, fallback: number): number {
  if (value === undefined) return fallback;
  const n = Number(value);
  return Number.isFinite(n) && n >= 0 ? n : fallback;
}

/** Convierte el mapa crudo del backend a `AppSettings` con defaults saneados. */
export function settingsFromMap(raw: Record<string, string>): AppSettings {
  return {
    autoLockMinutes: parseNonNegative(raw.autoLockMinutes, DEFAULT_SETTINGS.autoLockMinutes),
    clipboardClearSeconds: parseNonNegative(
      raw.clipboardClearSeconds,
      DEFAULT_SETTINGS.clipboardClearSeconds,
    ),
  };
}

export function makeSettingsUseCases(io: Bridge = bridge) {
  return {
    async load(): Promise<AppSettings> {
      const raw = await io.invoke<Record<string, string>>("settings_get");
      return settingsFromMap(raw ?? {});
    },
    async set<K extends keyof AppSettings>(key: K, value: AppSettings[K]): Promise<void> {
      await io.invoke<void>("settings_set", { key, value: String(value) });
    },
  };
}

export const settingsUseCases = makeSettingsUseCases();
