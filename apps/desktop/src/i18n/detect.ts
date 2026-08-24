// Detección del idioma inicial de la interfaz.
//
// Precedencia (de mayor a menor): elección explícita del usuario (persistida por
// Paraglide en localStorage) → idioma del sistema operativo (vía plugin-os) →
// baseLocale (es). La detección del SO solo actúa la primera vez / mientras el
// usuario no haya elegido idioma a mano: si eligió, eso manda siempre.
//
// Se usa plugin-os (no `navigator.language`) porque pregunta al SO real vía Rust,
// más fiable en los tres SO (importante para el empaquetado de Windows).
import { locale as osLocale } from "@tauri-apps/plugin-os";
import {
  setLocale,
  isLocale,
  localStorageKey,
  type Locale,
} from "$paraglide/runtime.js";

/** Reduce un locale del SO ("es-MX", "en_US.UTF-8") a su idioma base soportado. */
function toSupported(raw: string | null): Locale | null {
  if (!raw) return null;
  const base = raw.toLowerCase().split(/[-_.]/)[0];
  return isLocale(base) ? base : null;
}

/**
 * Fija el idioma inicial antes de montar la app. No recarga: se llama en el
 * arranque, así que el primer render ya sale en el idioma correcto. Idempotente y
 * a prueba de fallos (sin backend / sin plugin-os se queda en el baseLocale).
 */
export async function initLocale(): Promise<void> {
  // El usuario ya eligió idioma en una sesión previa → respétalo, no sondees el SO.
  if (localStorage.getItem(localStorageKey)) return;

  try {
    const detected = toSupported(await osLocale());
    if (detected) setLocale(detected, { reload: false });
  } catch {
    // Sin runtime de Tauri (p. ej. solo Vite) o sin permiso: baseLocale (es).
  }
}
