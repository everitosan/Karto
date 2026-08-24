// Casos de uso de la sección "Acerca de": abrir los enlaces del autor en el
// navegador del sistema. Sin vault ni credenciales: son enlaces fijos.
import { bridge, type Bridge } from "./tauri";

export function makeAboutUseCases(io: Bridge = bridge) {
  return {
    /** Abre un enlace externo (http/https) en el navegador del sistema. */
    async openExternalUrl(url: string): Promise<void> {
      await io.invoke<void>("open_external_url", { url });
    },
  };
}

export const aboutUseCases = makeAboutUseCases();
