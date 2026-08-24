// Estado reactivo compartido de las preferencias de la app dentro del Workspace.
// Vive junto a la vista (no es lógica de negocio: solo cachea lo que devuelve el
// caso de uso para que varios componentes —panel de propiedades, auto-bloqueo—
// reaccionen a los cambios sin encadenar props).
import {
  DEFAULT_SETTINGS,
  settingsUseCases,
  type AppSettings,
} from "$usecases/settings";

export const appSettings = $state<AppSettings>({ ...DEFAULT_SETTINGS });

/** Carga las preferencias del vault en el estado compartido. */
export async function loadAppSettings(): Promise<void> {
  Object.assign(appSettings, await settingsUseCases.load());
}

/** Actualiza una preferencia en memoria y la persiste en el vault. */
export async function updateAppSetting<K extends keyof AppSettings>(
  key: K,
  value: AppSettings[K],
): Promise<void> {
  appSettings[key] = value;
  await settingsUseCases.set(key, value);
}
