// Render de Markdown a HTML saneado. Módulo puro (sin Svelte) para poder
// probarlo aislado. `marked` parsea; `DOMPurify` limpia el HTML resultante
// antes de inyectarlo, imprescindible en un webview (evita XSS desde notas).
import { marked } from "marked";
import DOMPurify from "dompurify";

marked.setOptions({
  gfm: true, // tablas, tachado, autolinks…
  breaks: true, // un salto de línea simple = <br> (natural al tomar notas)
});

// Los enlaces abren fuera y sin fuga de referrer/opener.
let hookInstalled = false;
function installHook() {
  if (hookInstalled) return;
  DOMPurify.addHook("afterSanitizeAttributes", (node) => {
    if (node.tagName === "A") {
      node.setAttribute("target", "_blank");
      node.setAttribute("rel", "noopener noreferrer");
    }
  });
  hookInstalled = true;
}

/** Convierte Markdown a HTML saneado listo para inyectar. */
export function renderMarkdown(source: string): string {
  const raw = marked.parse(source ?? "", { async: false }) as string;
  installHook();
  return DOMPurify.sanitize(raw);
}
