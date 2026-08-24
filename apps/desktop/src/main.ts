import { mount } from "svelte";
import "@karto/ui/styles.css";
import "./app.css";
import { initLocale } from "$i18n/detect";
import App from "./App.svelte";

// Fija el idioma (elección persistida → idioma del SO → es) antes de montar, para
// que el primer render ya salga traducido y sin recargas. Se hace en un arranque
// async (no top-level await) porque el target del webview de Tauri no lo soporta.
async function bootstrap() {
  await initLocale();
  return mount(App, {
    target: document.getElementById("app")!,
  });
}

export default bootstrap();
